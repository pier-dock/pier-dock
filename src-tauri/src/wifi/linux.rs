//! Wi-Fi no Linux pelo NetworkManager (D-Bus do sistema). Ligar, conectar e desconectar passam pelo
//! polkit, que libera isso para o usuário da sessão ativa.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use super::{WifiNetwork, WifiState, ERR_AUTH, ERR_UNSUPPORTED};
use crate::platform::linux::dbus;

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";
const WIRELESS: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const ACCESS_POINT: &str = "org.freedesktop.NetworkManager.AccessPoint";
const ACTIVE: &str = "org.freedesktop.NetworkManager.Connection.Active";
const SETTINGS: &str = "org.freedesktop.NetworkManager.Settings";
const SETTINGS_CONNECTION: &str = "org.freedesktop.NetworkManager.Settings.Connection";

/// NM_DEVICE_TYPE_WIFI.
const DEVICE_TYPE_WIFI: u32 = 2;
/// NM_802_11_AP_FLAGS_PRIVACY (WEP).
const AP_FLAGS_PRIVACY: u32 = 0x1;
/// NM_802_11_AP_SEC_KEY_MGMT_PSK, _802_1X e _SAE.
const SEC_KEY_MGMT_PSK: u32 = 0x100;
const SEC_KEY_MGMT_8021X: u32 = 0x200;
const SEC_KEY_MGMT_SAE: u32 = 0x400;
/// NM_ACTIVE_CONNECTION_STATE_ACTIVATED e _DEACTIVATED.
const ACTIVE_ACTIVATED: u32 = 2;
const ACTIVE_DEACTIVATED: u32 = 4;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

type ConnectionSettings = HashMap<String, HashMap<String, OwnedValue>>;

fn proxy(conn: &Connection, path: &str, interface: &str) -> Result<Proxy<'static>, String> {
    dbus::proxy(conn, NM, path, interface).ok_or_else(|| format!("NetworkManager indisponível ({interface})"))
}

/// O primeiro dispositivo Wi-Fi.
fn wifi_device(conn: &Connection) -> Option<OwnedObjectPath> {
    let nm = proxy(conn, NM_PATH, NM).ok()?;
    let devices: Vec<OwnedObjectPath> = nm.call("GetDevices", &()).ok()?;
    devices.into_iter().find(|path| {
        proxy(conn, path.as_str(), DEVICE).ok().and_then(|d| d.get_property::<u32>("DeviceType").ok()) == Some(DEVICE_TYPE_WIFI)
    })
}

/// Segurança pelo que o ponto de acesso anuncia: aberta, senha (WPA/WPA2 ou WPA3) ou corporativa.
#[derive(Debug, PartialEq, Clone, Copy)]
enum Security {
    Open,
    Psk,
    Sae,
    Enterprise,
}

fn security(flags: u32, wpa: u32, rsn: u32) -> Security {
    if (wpa | rsn) & SEC_KEY_MGMT_8021X != 0 {
        Security::Enterprise
    } else if rsn & SEC_KEY_MGMT_SAE != 0 && rsn & SEC_KEY_MGMT_PSK == 0 {
        // Só WPA3; redes de transição (WPA2 + WPA3) anunciam PSK também e aceitam wpa-psk.
        Security::Sae
    } else if wpa != 0 || rsn != 0 || flags & AP_FLAGS_PRIVACY != 0 {
        Security::Psk
    } else {
        Security::Open
    }
}

struct AccessPoint {
    path: OwnedObjectPath,
    ssid: String,
    strength: u8,
    security: Security,
}

fn access_points(conn: &Connection, device: &OwnedObjectPath) -> Vec<AccessPoint> {
    let Ok(wireless) = proxy(conn, device.as_str(), WIRELESS) else { return Vec::new() };
    let paths: Vec<OwnedObjectPath> = wireless.call("GetAllAccessPoints", &()).unwrap_or_default();
    paths
        .into_iter()
        .filter_map(|path| {
            let ap = proxy(conn, path.as_str(), ACCESS_POINT).ok()?;
            let ssid: Vec<u8> = ap.get_property("Ssid").ok()?;
            let flag = |name: &str| ap.get_property::<u32>(name).unwrap_or(0);
            Some(AccessPoint {
                ssid: String::from_utf8_lossy(&ssid).into_owned(),
                strength: ap.get_property("Strength").unwrap_or(0),
                security: security(flag("Flags"), flag("WpaFlags"), flag("RsnFlags")),
                path,
            })
        })
        .collect()
}

/// Conexões Wi-Fi salvas, por SSID.
fn saved_connections(conn: &Connection) -> HashMap<String, OwnedObjectPath> {
    let Ok(settings) = proxy(conn, SETTINGS_PATH, SETTINGS) else { return HashMap::new() };
    let paths: Vec<OwnedObjectPath> = settings.call("ListConnections", &()).unwrap_or_default();
    paths
        .into_iter()
        .filter_map(|path| {
            let connection = proxy(conn, path.as_str(), SETTINGS_CONNECTION).ok()?;
            let config: ConnectionSettings = connection.call("GetSettings", &()).ok()?;
            let ssid: Vec<u8> = config.get("802-11-wireless")?.get("ssid")?.try_clone().ok()?.try_into().ok()?;
            Some((String::from_utf8_lossy(&ssid).into_owned(), path))
        })
        .collect()
}

fn active_ssid(conn: &Connection, device: &OwnedObjectPath) -> Option<String> {
    let wireless = proxy(conn, device.as_str(), WIRELESS).ok()?;
    let ap: OwnedObjectPath = wireless.get_property("ActiveAccessPoint").ok()?;
    if ap.as_str() == "/" {
        return None;
    }
    let ssid: Vec<u8> = proxy(conn, ap.as_str(), ACCESS_POINT).ok()?.get_property("Ssid").ok()?;
    Some(String::from_utf8_lossy(&ssid).into_owned())
}

pub fn status() -> WifiState {
    let Some(conn) = dbus::system() else { return WifiState::default() };
    let Some(device) = wifi_device(conn) else { return WifiState::default() };
    let enabled = proxy(conn, NM_PATH, NM).ok().and_then(|nm| nm.get_property::<bool>("WirelessEnabled").ok()).unwrap_or(false);
    let mut state = WifiState { available: true, enabled, can_toggle: true, can_connect: true, ..Default::default() };
    if !enabled {
        return state;
    }
    let current = active_ssid(conn, &device);
    let saved = saved_connections(conn);
    state.networks = access_points(conn, &device)
        .into_iter()
        .map(|ap| WifiNetwork {
            connected: current.as_ref() == Some(&ap.ssid),
            known: saved.contains_key(&ap.ssid),
            ..WifiNetwork::new(ap.ssid, ap.strength, ap.security != Security::Open)
        })
        .collect();
    state
}

pub fn scan() -> Result<(), String> {
    let conn = dbus::system().ok_or("D-Bus do sistema indisponível")?;
    let device = wifi_device(conn).ok_or("sem interface Wi-Fi")?;
    let options: HashMap<&str, Value> = HashMap::new();
    // Falha se uma busca acabou de rodar; o resultado anterior continua valendo.
    let _ = proxy(conn, device.as_str(), WIRELESS)?.call::<_, _, ()>("RequestScan", &(options,));
    Ok(())
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let conn = dbus::system().ok_or("D-Bus do sistema indisponível")?;
    proxy(conn, NM_PATH, NM)?.set_property("WirelessEnabled", enabled).map_err(|e| e.to_string())
}

/// Espera a conexão ativa chegar em "ativada" ou cair.
fn wait_activated(conn: &Connection, active: &OwnedObjectPath) -> bool {
    let started = Instant::now();
    while started.elapsed() < CONNECT_TIMEOUT {
        // Quando a ativação falha o objeto some do barramento.
        let Ok(state) = proxy(conn, active.as_str(), ACTIVE).and_then(|p| p.get_property::<u32>("State").map_err(|e| e.to_string())) else {
            return false;
        };
        match state {
            ACTIVE_ACTIVATED => return true,
            ACTIVE_DEACTIVATED => return false,
            _ => std::thread::sleep(Duration::from_millis(400)),
        }
    }
    false
}

pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    let conn = dbus::system().ok_or("D-Bus do sistema indisponível")?;
    let device = wifi_device(conn).ok_or("sem interface Wi-Fi")?;
    let nm = proxy(conn, NM_PATH, NM)?;
    let root = ObjectPath::try_from("/").expect("caminho fixo");

    if let (Some(saved), None) = (saved_connections(conn).get(ssid), password) {
        let active: OwnedObjectPath = nm.call("ActivateConnection", &(saved, &device, &root)).map_err(|e| e.to_string())?;
        return if wait_activated(conn, &active) { Ok(()) } else { Err(ERR_AUTH.into()) };
    }

    let ap = access_points(conn, &device).into_iter().filter(|ap| ap.ssid == ssid).max_by_key(|ap| ap.strength).ok_or("rede não encontrada")?;
    let mut settings: HashMap<&str, HashMap<&str, Value>> = HashMap::new();
    match (ap.security, password) {
        (Security::Enterprise, _) => return Err(ERR_UNSUPPORTED.into()),
        (Security::Open, _) => {}
        (_, None) => return Err(ERR_AUTH.into()),
        (security, Some(password)) => {
            let key_mgmt = if security == Security::Sae { "sae" } else { "wpa-psk" };
            settings.insert("802-11-wireless-security", HashMap::from([("key-mgmt", Value::from(key_mgmt)), ("psk", Value::from(password))]));
        }
    }
    let (created, active): (OwnedObjectPath, OwnedObjectPath) =
        nm.call("AddAndActivateConnection", &(settings, &device, &ap.path)).map_err(|e| e.to_string())?;
    if wait_activated(conn, &active) {
        return Ok(());
    }
    // Sem isso a senha errada ficaria salva e o NetworkManager tentaria de novo sozinho.
    if let Ok(connection) = proxy(conn, created.as_str(), SETTINGS_CONNECTION) {
        let _ = connection.call::<_, _, ()>("Delete", &());
    }
    Err(ERR_AUTH.into())
}

pub fn disconnect() -> Result<(), String> {
    let conn = dbus::system().ok_or("D-Bus do sistema indisponível")?;
    let device = wifi_device(conn).ok_or("sem interface Wi-Fi")?;
    let active: OwnedObjectPath = proxy(conn, device.as_str(), DEVICE)?.get_property("ActiveConnection").map_err(|e| e.to_string())?;
    if active.as_str() == "/" {
        return Ok(());
    }
    proxy(conn, NM_PATH, NM)?.call::<_, _, ()>("DeactivateConnection", &(&active,)).map_err(|e| e.to_string())
}

/// O Linux não pede permissão para ver as redes; o botão abre as configurações de Wi-Fi.
pub fn request_permission(_app: &tauri::AppHandle) {
    crate::system::open_shell("network".into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_security_from_ap_flags() {
        assert_eq!(security(0, 0, 0), Security::Open);
        assert_eq!(security(AP_FLAGS_PRIVACY, 0, 0), Security::Psk);
        // WPA2-PSK: pares/grupo CCMP (0x8 | 0x80) + PSK (0x100).
        assert_eq!(security(AP_FLAGS_PRIVACY, 0, 0x188), Security::Psk);
        assert_eq!(security(AP_FLAGS_PRIVACY, 0, SEC_KEY_MGMT_SAE | SEC_KEY_MGMT_PSK | 0x88), Security::Psk);
        assert_eq!(security(AP_FLAGS_PRIVACY, 0, SEC_KEY_MGMT_SAE | 0x88), Security::Sae);
        assert_eq!(security(AP_FLAGS_PRIVACY, 0, SEC_KEY_MGMT_8021X | 0x88), Security::Enterprise);
    }
}
