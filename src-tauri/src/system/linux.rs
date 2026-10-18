//! Bandeja no Linux: bateria pelo sysfs, rede pelo NetworkManager (D-Bus), volume pelo `pactl`
//! (PulseAudio ou PipeWire), Bluetooth pelo BlueZ (D-Bus) e menus do GNOME pela extensão.

use std::collections::HashMap;
use std::path::Path;

use zbus::zvariant::OwnedValue;

use super::{Battery, Bluetooth, BluetoothDevice, Network, Volume};
use crate::platform::linux::{dbus, gnome_shell, x11};
use crate::util::{run, spawn_detached};

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

pub fn battery() -> Battery {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else { return Battery::absent() };
    let supplies: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    // `scope=Device` são baterias de mouse/fone, não do computador.
    let Some(battery) = supplies
        .iter()
        .find(|p| read(&p.join("type")) == "Battery" && read(&p.join("scope")) != "Device" && p.join("capacity").exists())
    else {
        return Battery::absent();
    };
    let on_ac = supplies.iter().any(|p| read(&p.join("type")) == "Mains" && read(&p.join("online")) == "1");
    let status = read(&battery.join("status"));
    Battery {
        present: true,
        percent: read(&battery.join("capacity")).parse::<u8>().unwrap_or(0).min(100),
        charging: on_ac || status == "Charging" || status == "Full",
    }
}

const NM: &str = "org.freedesktop.NetworkManager";
/// NM_CONNECTIVITY_FULL; 2 = portal cativo, 3 = rede local sem internet.
const CONNECTIVITY_FULL: u32 = 4;

pub fn network() -> Network {
    network_from_nm().unwrap_or_else(Network::disconnected)
}

fn network_from_nm() -> Option<Network> {
    let conn = dbus::system()?;
    let nm = dbus::proxy(conn, NM, "/org/freedesktop/NetworkManager", NM)?;
    let connectivity: u32 = nm.get_property("Connectivity").ok()?;
    let primary: zbus::zvariant::OwnedObjectPath = nm.get_property("PrimaryConnection").ok()?;
    if primary.as_str() == "/" {
        return Some(Network::disconnected());
    }
    let active = dbus::proxy(conn, NM, primary.as_str(), "org.freedesktop.NetworkManager.Connection.Active")?;
    let kind: String = active.get_property("Type").unwrap_or_default();
    let name: String = active.get_property("Id").unwrap_or_default();
    let wifi = kind == "802-11-wireless";
    let signal_bars = if wifi { wifi_strength(conn, &active).map(signal_bars) } else { None };
    Some(Network { connected: connectivity > 1, internet: connectivity == CONNECTIVITY_FULL, wifi, signal_bars, name })
}

fn wifi_strength(conn: &zbus::blocking::Connection, active: &zbus::blocking::Proxy) -> Option<u8> {
    let devices: Vec<zbus::zvariant::OwnedObjectPath> = active.get_property("Devices").ok()?;
    let device = dbus::proxy(conn, NM, devices.first()?.as_str(), "org.freedesktop.NetworkManager.Device.Wireless")?;
    let ap: zbus::zvariant::OwnedObjectPath = device.get_property("ActiveAccessPoint").ok()?;
    dbus::proxy(conn, NM, ap.as_str(), "org.freedesktop.NetworkManager.AccessPoint")?.get_property("Strength").ok()
}

/// Intensidade 0–100 → 0–5 barras (a mesma escala do Windows).
fn signal_bars(strength: u8) -> u8 {
    strength.min(100).div_ceil(20)
}

/// Primeiro "NN%" da saída do `pactl get-sink-volume`.
fn parse_volume_percent(output: &str) -> Option<f32> {
    output.split('/').map(str::trim).find_map(|part| part.strip_suffix('%')?.trim().parse::<f32>().ok()).map(|p| p / 100.0)
}

pub fn volume() -> Result<Volume, String> {
    let level = run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])
        .as_deref()
        .and_then(parse_volume_percent)
        .ok_or("não foi possível ler o volume (pactl)")?;
    let muted = run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]).is_some_and(|out| out.contains("yes"));
    Ok(Volume { level, muted })
}

pub fn set_volume(level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    if let Some(level) = level {
        let percent = format!("{}%", (level * 100.0).round() as u32);
        run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &percent]).ok_or("falha ao ajustar o volume")?;
    }
    if let Some(muted) = muted {
        run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", if muted { "1" } else { "0" }]).ok_or("falha ao silenciar")?;
    }
    Ok(())
}

const BLUEZ: &str = "org.bluez";

/// Propriedades de cada interface de um objeto do BlueZ (`Adapter1`, `Device1`, `Battery1`…).
type Interfaces = HashMap<String, HashMap<String, OwnedValue>>;

fn bluez_objects() -> Option<Vec<(String, Interfaces)>> {
    let manager = zbus::blocking::fdo::ObjectManagerProxy::builder(dbus::system()?)
        .destination(BLUEZ)
        .ok()?
        .path("/")
        .ok()?
        .build()
        .ok()?;
    let objects = manager.get_managed_objects().ok()?;
    Some(
        objects
            .into_iter()
            .map(|(path, interfaces)| (path.to_string(), interfaces.into_iter().map(|(name, props)| (name.to_string(), props)).collect()))
            .collect(),
    )
}

fn prop<T: TryFrom<OwnedValue>>(interfaces: &Interfaces, interface: &str, name: &str) -> Option<T> {
    T::try_from(interfaces.get(interface)?.get(name)?.try_clone().ok()?).ok()
}

/// Tipo do dispositivo pelo ícone freedesktop que o BlueZ anuncia (`audio-headset`, `input-mouse`…).
fn kind_from_icon(icon: &str) -> &'static str {
    match icon {
        i if i.starts_with("audio-") => "audio",
        "input-mouse" | "input-tablet" => "mouse",
        "input-keyboard" => "keyboard",
        "input-gaming" => "gamepad",
        "phone" => "phone",
        _ => "other",
    }
}

fn bluetooth_from_objects(objects: &[(String, Interfaces)]) -> Bluetooth {
    let adapters: Vec<_> = objects.iter().filter(|(_, i)| i.contains_key("org.bluez.Adapter1")).collect();
    if adapters.is_empty() {
        return Bluetooth::absent();
    }
    let powered = adapters.iter().any(|(_, i)| prop::<bool>(i, "org.bluez.Adapter1", "Powered").unwrap_or(false));
    let devices = objects
        .iter()
        .filter(|(_, i)| prop::<bool>(i, "org.bluez.Device1", "Connected").unwrap_or(false))
        .map(|(_, i)| BluetoothDevice {
            name: prop::<String>(i, "org.bluez.Device1", "Alias").unwrap_or_default(),
            kind: kind_from_icon(&prop::<String>(i, "org.bluez.Device1", "Icon").unwrap_or_default()),
            battery: prop::<u8>(i, "org.bluez.Battery1", "Percentage").map(|p| p.min(100)),
        })
        .collect();
    Bluetooth { available: true, powered, can_toggle: true, devices }
}

pub fn bluetooth() -> Bluetooth {
    bluez_objects().map(|objects| bluetooth_from_objects(&objects)).unwrap_or_else(Bluetooth::absent)
}

pub fn set_bluetooth(powered: bool) -> Result<(), String> {
    let conn = dbus::system().ok_or("sem acesso ao D-Bus do sistema")?;
    let objects = bluez_objects().ok_or("BlueZ indisponível")?;
    for (path, _) in objects.iter().filter(|(_, i)| i.contains_key("org.bluez.Adapter1")) {
        let adapter = dbus::proxy(conn, BLUEZ, path, "org.bluez.Adapter1").ok_or("adaptador não encontrado")?;
        adapter.set_property("Powered", powered).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Configurações de Bluetooth: as do GNOME, as do Blueman (Linux Mint e outras) ou as do Cinnamon.
fn open_bluetooth_settings() {
    let _ = spawn_detached("gnome-control-center", &["bluetooth"])
        .or_else(|_| spawn_detached("blueman-manager", &[]))
        .or_else(|_| spawn_detached("cinnamon-settings", &["bluetooth"]));
}

fn open_sound_settings() {
    let _ = spawn_detached("gnome-control-center", &["sound"])
        .or_else(|_| spawn_detached("systemsettings", &["kcm_pulseaudio"]))
        .or_else(|_| spawn_detached("pavucontrol", &[]))
        .or_else(|_| spawn_detached("cinnamon-settings", &["sound"]));
}

/// Grade de apps, menu de configurações rápidas e calendário/notificações do GNOME. Sem a
/// extensão, o menu de apps abre pela tecla Super (Cinnamon, MATE e Xfce, sessões X11) e as
/// configurações pelo app de Configurações. O Monitor do Sistema e o Agenda do GNOME são apps
/// comuns.
pub fn open_shell(kind: &str) {
    match (gnome_shell::shell(), kind) {
        (Some(shell), "start") => drop(shell.show_apps()),
        (Some(shell), "quick-settings") => drop(shell.open_quick_settings()),
        (Some(shell), "notifications") => drop(shell.open_calendar()),
        (None, "start") => {
            if let Some(x11) = x11::get() {
                x11.tap_super();
            }
        }
        (None, "quick-settings") => drop(spawn_detached("gnome-control-center", &[])),
        (_, "system-monitor") => drop(spawn_detached("gnome-system-monitor", &[])),
        (_, "calendar") => drop(spawn_detached("gnome-calendar", &[])),
        (_, "bluetooth") => open_bluetooth_settings(),
        (_, "sound") => open_sound_settings(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pactl_volume() {
        let out = "Volume: front-left: 32768 /  50% / -18,06 dB,   front-right: 32768 /  50% / -18,06 dB\n        balance 0,00\n";
        assert_eq!(parse_volume_percent(out), Some(0.5));
        assert_eq!(parse_volume_percent("Volume: mono: 65536 / 100% / 0,00 dB"), Some(1.0));
        assert_eq!(parse_volume_percent("Failed"), None);
    }

    #[test]
    fn reads_bluez_objects() {
        let props = |pairs: &[(&str, OwnedValue)]| pairs.iter().map(|(k, v)| (k.to_string(), v.try_clone().unwrap())).collect();
        let adapter: Interfaces = [("org.bluez.Adapter1".to_string(), props(&[("Powered", OwnedValue::from(true))]))].into();
        let headset: Interfaces = [
            (
                "org.bluez.Device1".to_string(),
                props(&[
                    ("Connected", OwnedValue::from(true)),
                    ("Alias", OwnedValue::try_from(zbus::zvariant::Value::from("WH-1000XM4")).unwrap()),
                    ("Icon", OwnedValue::try_from(zbus::zvariant::Value::from("audio-headset")).unwrap()),
                ]),
            ),
            ("org.bluez.Battery1".to_string(), props(&[("Percentage", OwnedValue::from(80u8))])),
        ]
        .into();
        let paired: Interfaces = [("org.bluez.Device1".to_string(), props(&[("Connected", OwnedValue::from(false))]))].into();
        let objects = vec![
            ("/org/bluez/hci0".to_string(), adapter),
            ("/org/bluez/hci0/dev_1".to_string(), headset),
            ("/org/bluez/hci0/dev_2".to_string(), paired),
        ];
        let bt = bluetooth_from_objects(&objects);
        assert!(bt.available && bt.powered);
        assert_eq!(bt.devices.len(), 1);
        assert_eq!(bt.devices[0].name, "WH-1000XM4");
        assert_eq!(bt.devices[0].kind, "audio");
        assert_eq!(bt.devices[0].battery, Some(80));
        assert!(!bluetooth_from_objects(&[]).available);
    }

    #[test]
    fn maps_signal_strength_to_bars() {
        assert_eq!(signal_bars(0), 0);
        assert_eq!(signal_bars(1), 1);
        assert_eq!(signal_bars(55), 3);
        assert_eq!(signal_bars(100), 5);
    }
}
