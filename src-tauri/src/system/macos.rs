//! Bandeja no macOS: bateria pelo `pmset`, rede pela rota padrão e CoreWLAN, volume pelo
//! AppleScript (Standard Additions, sem pedir permissão de Automação), Bluetooth pelo IOBluetooth e
//! os menus do sistema pelos itens da barra de menus do Central de Controle.

use objc2_core_wlan::CWWiFiClient;
use objc2_io_bluetooth::{BluetoothHCIPowerState, IOBluetoothDevice, IOBluetoothHostController};

use super::{kind_from_class, Battery, Bluetooth, BluetoothDevice, Network, Volume};
use crate::platform::macos::ax::Element;
use crate::platform::macos::{osascript, workspace};
use crate::util::{run, spawn_detached};

/// Saída do `pmset -g batt`:
/// `Now drawing from 'AC Power'\n -InternalBattery-0 (id=…)\t87%; charging; 1:02 remaining present: true`.
fn parse_pmset(output: &str) -> Battery {
    let Some(line) = output.lines().find(|l| l.contains("InternalBattery")) else { return Battery::absent() };
    let percent = line
        .split('\t')
        .nth(1)
        .and_then(|rest| rest.split('%').next())
        .and_then(|p| p.trim().parse::<u8>().ok())
        .unwrap_or(0)
        .min(100);
    let status = line.split(';').nth(1).map(str::trim).unwrap_or_default();
    let on_ac = output.lines().next().is_some_and(|l| l.contains("AC Power"));
    Battery { present: true, percent, charging: on_ac || matches!(status, "charging" | "charged" | "finishing charge") }
}

pub fn battery() -> Battery {
    run("/usr/bin/pmset", &["-g", "batt"]).map(|out| parse_pmset(&out)).unwrap_or_else(Battery::absent)
}

fn default_interface() -> Option<String> {
    let out = run("/sbin/route", &["-n", "get", "default"])?;
    out.lines().find_map(|l| l.trim().strip_prefix("interface:")).map(|i| i.trim().to_string())
}

/// RSSI em dBm → 0–5 barras, na mesma escala do Windows e do Linux (−50 dBm ou mais = 5).
fn signal_bars(rssi: i64) -> u8 {
    let strength = (2 * (rssi + 100)).clamp(0, 100) as u8;
    strength.div_ceil(20)
}

pub fn network() -> Network {
    let Some(interface) = default_interface() else { return Network::disconnected() };
    let wifi = unsafe { CWWiFiClient::sharedWiFiClient().interface() }
        .filter(|w| unsafe { w.interfaceName() }.is_some_and(|n| n.to_string() == interface));
    let (is_wifi, signal_bars, name) = match wifi {
        // O SSID exige permissão de Localização desde o macOS 14; sem ela vem vazio.
        Some(w) => {
            let ssid = unsafe { w.ssid() }.map(|s| s.to_string()).filter(|s| !s.is_empty());
            (true, Some(signal_bars(unsafe { w.rssiValue() } as i64)), ssid.unwrap_or_else(|| "Wi‑Fi".into()))
        }
        None => (false, None, "Ethernet".into()),
    };
    Network { connected: true, internet: true, wifi: is_wifi, signal_bars, name }
}

/// Saída de `(output volume as text) & "," & (output muted as text)`: `75,false`. Saídas sem
/// controle de volume (HDMI, algumas interfaces USB) dão `missing value`.
fn parse_volume(output: &str) -> Option<Volume> {
    let (level, muted) = output.trim().split_once(',')?;
    Some(Volume { level: level.parse::<f32>().ok()? / 100.0, muted: muted == "true" })
}

pub fn volume() -> Result<Volume, String> {
    osascript("set s to get volume settings\nreturn (output volume of s as text) & \",\" & (output muted of s as text)")
        .as_deref()
        .and_then(parse_volume)
        .ok_or_else(|| "esta saída de áudio não tem controle de volume".into())
}

pub fn set_volume(level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    if let Some(level) = level {
        osascript(&format!("set volume output volume {}", (level * 100.0).round() as u32)).ok_or("falha ao ajustar o volume")?;
    }
    if let Some(muted) = muted {
        osascript(&format!("set volume output muted {muted}")).ok_or("falha ao silenciar")?;
    }
    Ok(())
}

/// A bateria dos dispositivos só sai de APIs privadas do IOBluetooth, então fica de fora.
pub fn bluetooth() -> Bluetooth {
    let Some(controller) = (unsafe { IOBluetoothHostController::defaultController() }) else { return Bluetooth::absent() };
    let powered = unsafe { controller.powerState() } == BluetoothHCIPowerState::ON;
    let paired = unsafe { IOBluetoothDevice::pairedDevices() };
    let devices = paired
        .iter()
        .flat_map(|list| list.iter())
        .filter_map(|object| object.downcast::<IOBluetoothDevice>().ok())
        .filter(|device| unsafe { device.isConnected() })
        .map(|device| BluetoothDevice {
            name: unsafe { device.nameOrAddress() }.map(|n| n.to_string()).unwrap_or_default(),
            kind: kind_from_class(unsafe { device.classOfDevice() }),
            battery: None,
        })
        .collect();
    Bluetooth { available: true, powered, can_toggle: false, devices }
}

pub fn set_bluetooth(_powered: bool) -> Result<(), String> {
    Err("o macOS não permite ligar o Bluetooth por apps".into())
}

/// Clica num item da barra de menus do Central de Controle pelo identificador (que, ao
/// contrário do título, não é traduzido). Exige a permissão de Acessibilidade.
fn press_menu_extra(identifier: &str) -> bool {
    let Some(app) = workspace::pid_of("com.apple.controlcenter").and_then(Element::application) else { return false };
    let Some(bar) = app.element("AXExtrasMenuBar") else { return false };
    bar.elements("AXChildren")
        .into_iter()
        .find(|item| item.string("AXIdentifier").as_deref() == Some(identifier))
        .is_some_and(|item| item.perform(accessibility_sys::kAXPressAction))
}

/// `start` abre a grade de apps (o app Apps no macOS 26, o Launchpad antes dele),
/// `quick-settings` o Central de Controle, `notifications` a Central de Notificações,
/// `system-monitor` o Monitor de Atividade e `calendar` o Calendário.
pub fn open_shell(kind: &str) {
    match kind {
        "start" => {
            let apps = "/System/Applications/Apps.app";
            let _ = if std::path::Path::new(apps).exists() {
                spawn_detached("/usr/bin/open", &[apps])
            } else {
                spawn_detached("/usr/bin/open", &["-a", "Launchpad"])
            };
        }
        "quick-settings" => {
            if !press_menu_extra("com.apple.menuextra.controlcenter") {
                let _ = spawn_detached("/usr/bin/open", &["x-apple.systempreferences:"]);
            }
        }
        "notifications" => {
            press_menu_extra("com.apple.menuextra.clock");
        }
        "system-monitor" => drop(spawn_detached("/usr/bin/open", &["-b", "com.apple.ActivityMonitor"])),
        "calendar" => drop(spawn_detached("/usr/bin/open", &["-b", "com.apple.iCal"])),
        "bluetooth" => drop(spawn_detached("/usr/bin/open", &["x-apple.systempreferences:com.apple.BluetoothSettings"])),
        "sound" => drop(spawn_detached("/usr/bin/open", &["x-apple.systempreferences:com.apple.Sound-Settings.extension"])),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pmset_battery() {
        let on_battery = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=6946915)\t87%; discharging; 5:39 remaining present: true\n";
        let b = parse_pmset(on_battery);
        assert!(b.present && !b.charging);
        assert_eq!(b.percent, 87);
        let charging = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t100%; charged; 0:00 remaining present: true\n";
        let b = parse_pmset(charging);
        assert!(b.charging);
        assert_eq!(b.percent, 100);
        assert!(!parse_pmset("Now drawing from 'AC Power'\n").present);
    }

    #[test]
    fn parses_volume_settings() {
        let v = parse_volume("75,false\n").unwrap();
        assert_eq!(v.level, 0.75);
        assert!(!v.muted);
        assert!(parse_volume("0,true").unwrap().muted);
        assert!(parse_volume("missing value,false").is_none());
    }

    #[test]
    fn maps_rssi_to_bars() {
        assert_eq!(signal_bars(-40), 5);
        assert_eq!(signal_bars(-70), 3);
        assert_eq!(signal_bars(-100), 0);
    }
}
