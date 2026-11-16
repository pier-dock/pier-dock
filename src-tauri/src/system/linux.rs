//! Bandeja no Linux: bateria pelo sysfs, rede pelo NetworkManager (D-Bus), volume pelo `pactl`
//! (PulseAudio ou PipeWire), Bluetooth pelo BlueZ (D-Bus) e menus do GNOME pela extensão.

use std::collections::HashMap;
use std::path::Path;

use zbus::zvariant::OwnedValue;

use super::{Battery, Bluetooth, BluetoothDevice, DoNotDisturb, Network, Volume};
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
        .ok_or(crate::i18n::tr("não foi possível ler o volume (pactl)", "couldn't read the volume (pactl)"))?;
    let muted = run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]).is_some_and(|out| out.contains("yes"));
    Ok(Volume { level, muted })
}

pub fn set_volume(level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    if let Some(level) = level {
        let percent = format!("{}%", (level * 100.0).round() as u32);
        run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &percent]).ok_or(crate::i18n::tr("falha ao ajustar o volume", "couldn't set the volume"))?;
    }
    if let Some(muted) = muted {
        run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", if muted { "1" } else { "0" }]).ok_or(crate::i18n::tr("falha ao silenciar", "couldn't mute"))?;
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
    let flag = |i: &Interfaces, name: &str| prop::<bool>(i, "org.bluez.Device1", name).unwrap_or(false);
    // O BlueZ também lista os dispositivos que só foram vistos numa busca; `Bonded` cobre os LE que
    // guardaram a chave sem marcar `Paired`.
    let devices = objects
        .iter()
        .filter(|(_, i)| flag(i, "Paired") || flag(i, "Bonded") || flag(i, "Trusted") || flag(i, "Connected"))
        .map(|(path, i)| BluetoothDevice {
            id: path.clone(),
            name: ["Alias", "Name", "Address"].into_iter()
                .filter_map(|key| prop::<String>(i, "org.bluez.Device1", key))
                .find(|name| !name.trim().is_empty()).unwrap_or_else(|| path.clone()),
            kind: kind_from_icon(&prop::<String>(i, "org.bluez.Device1", "Icon").unwrap_or_default()),
            battery: prop::<u8>(i, "org.bluez.Battery1", "Percentage").map(|p| p.min(100)),
            connected: flag(i, "Connected"),
        })
        .collect();
    Bluetooth { available: true, powered, can_toggle: true, can_connect: true, devices }
}

pub fn bluetooth() -> Bluetooth {
    bluez_objects().map(|objects| bluetooth_from_objects(&objects)).unwrap_or_else(Bluetooth::absent)
}

pub fn do_not_disturb() -> DoNotDisturb {
    DoNotDisturb { available: false, enabled: false }
}

pub fn set_do_not_disturb(_enabled: bool) -> Result<(), String> {
    Err(crate::i18n::tr("o Não incomodar só existe no Windows", "Do Not Disturb only exists on Windows").into())
}

pub fn set_bluetooth(powered: bool) -> Result<(), String> {
    let conn = dbus::system().ok_or(crate::i18n::tr("sem acesso ao D-Bus do sistema", "no access to the system D-Bus"))?;
    let objects = bluez_objects().ok_or(crate::i18n::tr("BlueZ indisponível", "BlueZ unavailable"))?;
    for (path, _) in objects.iter().filter(|(_, i)| i.contains_key("org.bluez.Adapter1")) {
        let adapter = dbus::proxy(conn, BLUEZ, path, "org.bluez.Adapter1").ok_or(crate::i18n::tr("adaptador não encontrado", "adapter not found"))?;
        adapter.set_property("Powered", powered).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn call_device(id: &str, method: &str) -> Result<(), String> {
    let conn = dbus::system().ok_or(crate::i18n::tr("sem acesso ao D-Bus do sistema", "no access to the system D-Bus"))?;
    let device = dbus::proxy(conn, BLUEZ, id, "org.bluez.Device1").ok_or(crate::i18n::tr("dispositivo não encontrado", "device not found"))?;
    device.call_method(method, &()).map(drop).map_err(|e| e.to_string())
}

pub fn connect_bluetooth(id: &str) -> Result<(), String> {
    call_device(id, "Connect")
}

pub fn disconnect_bluetooth(id: &str) -> Result<(), String> {
    call_device(id, "Disconnect")
}

/// Launch the settings application for the current desktop, retaining fallbacks for mixed installs.
pub fn open_shell(kind: &str) -> Result<(), String> {
    if let Some(shell) = gnome_shell::shell() {
        let result = match kind {
            "start" => Some(shell.show_apps()),
            "quick-settings" => Some(shell.open_quick_settings()),
            "notifications" => Some(shell.open_calendar()),
            _ => None,
        };
        if let Some(result) = result { return result.map_err(|e| e.to_string()); }
    }
    match kind {
        "start" => {
            let x11 = x11::get().ok_or("Application launcher is unavailable in this session")?;
            x11.tap_super();
            Ok(())
        },
        "bluetooth" | "sound" | "network" | "quick-settings" => {
            let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
            super::settings::launch(&super::settings::candidates(&desktop, kind), |program, args| spawn_detached(program, args))
        },
        "system-monitor" => spawn_detached("gnome-system-monitor", &[]),
        "calendar" => spawn_detached("gnome-calendar", &[]),
        _ => Err(format!("System action unavailable: {kind}")),
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
        let paired: Interfaces =
            [("org.bluez.Device1".to_string(), props(&[("Connected", OwnedValue::from(false)), ("Paired", OwnedValue::from(true))]))].into();
        let seen: Interfaces = [("org.bluez.Device1".to_string(), props(&[("Connected", OwnedValue::from(false))]))].into();
        let objects = vec![
            ("/org/bluez/hci0".to_string(), adapter),
            ("/org/bluez/hci0/dev_1".to_string(), headset),
            ("/org/bluez/hci0/dev_2".to_string(), paired),
            ("/org/bluez/hci0/dev_3".to_string(), seen),
        ];
        let bt = bluetooth_from_objects(&objects);
        assert!(bt.available && bt.powered);
        assert_eq!(bt.devices.len(), 2);
        assert_eq!(bt.devices[0].id, "/org/bluez/hci0/dev_1");
        assert_eq!(bt.devices[0].name, "WH-1000XM4");
        assert_eq!(bt.devices[0].kind, "audio");
        assert_eq!(bt.devices[0].battery, Some(80));
        assert!(bt.devices[0].connected);
        assert_eq!(bt.devices[1].id, "/org/bluez/hci0/dev_2");
        assert!(!bt.devices[1].connected);
        assert!(!bluetooth_from_objects(&[]).available);
    }

    #[test]
    fn saved_devices_survive_radio_off_and_have_names() {
        let value = |text: &str| OwnedValue::try_from(zbus::zvariant::Value::from(text)).unwrap();
        let adapter = |powered| [("org.bluez.Adapter1".into(), [("Powered".into(), OwnedValue::from(powered))].into())].into();
        let device = |flag: &str, name_key: &str, name: &str| -> Interfaces {
            [("org.bluez.Device1".into(), [(flag.into(), OwnedValue::from(true)), ("Alias".into(), value("")), (name_key.into(), value(name))].into())].into()
        };
        let objects = vec![
            ("/hci0".into(), adapter(false)), ("/hci1".into(), adapter(false)),
            ("/hci0/paired".into(), device("Paired", "Name", "Keyboard")),
            ("/hci1/bonded".into(), device("Bonded", "Address", "AA:BB:CC")),
            ("/hci1/trusted".into(), device("Trusted", "Name", "Mouse")),
            ("/hci0/seen".into(), device("Seen", "Name", "Not saved")),
        ];
        let bt = bluetooth_from_objects(&objects);
        assert!(!bt.powered);
        assert_eq!(bt.devices.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["Keyboard", "AA:BB:CC", "Mouse"]);
        assert!(bt.devices.iter().all(|d| !d.connected));
    }

    #[test]
    fn maps_signal_strength_to_bars() {
        assert_eq!(signal_bars(0), 0);
        assert_eq!(signal_bars(1), 1);
        assert_eq!(signal_bars(55), 3);
        assert_eq!(signal_bars(100), 5);
    }
}
