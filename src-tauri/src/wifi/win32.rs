//! Wi-Fi no Windows pela Native Wifi API (wlanapi) e o rádio pelo WinRT. Desde o Windows 11 24H2 a
//! lista de redes exige a permissão de Localização: sem ela vem `ERROR_ACCESS_DENIED`.

use std::time::{Duration, Instant};

use windows::core::{GUID, HSTRING, PCWSTR};
use windows::Devices::Radios::{Radio, RadioKind, RadioState};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SUCCESS, HANDLE};
use windows::Win32::NetworkManagement::WiFi::{
    dot11_BSS_type_any, wlan_connection_mode_profile, WlanCloseHandle, WlanConnect, WlanDeleteProfile, WlanDisconnect, WlanEnumInterfaces,
    WlanFreeMemory, WlanGetAvailableNetworkList, WlanOpenHandle, WlanScan, WlanSetProfile, DOT11_AUTH_ALGORITHM, DOT11_AUTH_ALGO_80211_OPEN,
    DOT11_AUTH_ALGO_RSNA_PSK, DOT11_AUTH_ALGO_WPA3_SAE, DOT11_AUTH_ALGO_WPA_PSK, WLAN_AVAILABLE_NETWORK_CONNECTED,
    WLAN_AVAILABLE_NETWORK_HAS_PROFILE, WLAN_AVAILABLE_NETWORK_LIST, WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO_LIST,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::{WifiNetwork, WifiState, ERR_AUTH, ERR_UNSUPPORTED};
use crate::util::ErrStr;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Sessão da wlanapi, fechada ao sair do escopo.
struct Wlan(HANDLE);

impl Wlan {
    fn open() -> Result<Self, String> {
        let mut version = 0;
        let mut handle = HANDLE::default();
        // 2 = API do Windows Vista em diante.
        check(unsafe { WlanOpenHandle(2, None, &mut version, &mut handle) })?;
        Ok(Wlan(handle))
    }

    /// A primeira interface Wi-Fi.
    fn interface(&self) -> Option<GUID> {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if unsafe { WlanEnumInterfaces(self.0, None, &mut list) } != ERROR_SUCCESS.0 || list.is_null() {
            return None;
        }
        let guid = unsafe { ((*list).dwNumberOfItems > 0).then(|| (*list).InterfaceInfo[0].InterfaceGuid) };
        unsafe { WlanFreeMemory(list as *const _) };
        guid
    }

    fn networks(&self, interface: &GUID) -> Result<Vec<Available>, u32> {
        let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
        let code = unsafe { WlanGetAvailableNetworkList(self.0, interface, 0, None, &mut list) };
        if code != ERROR_SUCCESS.0 || list.is_null() {
            return Err(code);
        }
        let networks = unsafe {
            // A lista é declarada com um item, mas o Windows aloca `dwNumberOfItems`.
            let items = std::slice::from_raw_parts((*list).Network.as_ptr(), (*list).dwNumberOfItems as usize);
            items
                .iter()
                .map(|n| Available {
                    ssid: String::from_utf8_lossy(&n.dot11Ssid.ucSSID[..(n.dot11Ssid.uSSIDLength as usize).min(32)]).into_owned(),
                    profile: wide_to_string(&n.strProfileName),
                    quality: n.wlanSignalQuality.min(100) as u8,
                    secured: n.bSecurityEnabled.as_bool(),
                    auth: n.dot11DefaultAuthAlgorithm,
                    connected: n.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0,
                    known: n.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE != 0,
                })
                .collect()
        };
        unsafe { WlanFreeMemory(list as *const _) };
        Ok(networks)
    }
}

impl Drop for Wlan {
    fn drop(&mut self) {
        unsafe { WlanCloseHandle(self.0, None) };
    }
}

struct Available {
    ssid: String,
    profile: String,
    quality: u8,
    secured: bool,
    auth: DOT11_AUTH_ALGORITHM,
    connected: bool,
    known: bool,
}

fn check(code: u32) -> Result<(), String> {
    if code == ERROR_SUCCESS.0 {
        Ok(())
    } else {
        Err(windows::core::Error::from(windows::core::HRESULT::from_win32(code)).message())
    }
}

fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

fn wifi_radio() -> Option<Radio> {
    let radios = Radio::GetRadiosAsync().ok()?.get().ok()?;
    radios.into_iter().find(|r| r.Kind().is_ok_and(|k| k == RadioKind::WiFi))
}

pub fn status() -> WifiState {
    let radio = wifi_radio();
    let Ok(wlan) = Wlan::open() else { return WifiState::default() };
    let Some(interface) = wlan.interface() else { return WifiState::default() };
    let enabled = radio.as_ref().map_or(true, |r| r.State().is_ok_and(|s| s == RadioState::On));
    let mut state = WifiState { available: true, enabled, can_toggle: radio.is_some(), can_connect: true, ..Default::default() };
    if !enabled {
        return state;
    }
    match wlan.networks(&interface) {
        Ok(networks) => {
            state.networks = networks
                .into_iter()
                .map(|n| WifiNetwork { known: n.known, connected: n.connected, ..WifiNetwork::new(n.ssid, n.quality, n.secured) })
                .collect()
        }
        Err(code) => state.needs_permission = code == ERROR_ACCESS_DENIED.0,
    }
    state
}

pub fn scan() -> Result<(), String> {
    let wlan = Wlan::open()?;
    let interface = wlan.interface().ok_or("sem interface Wi-Fi")?;
    check(unsafe { WlanScan(wlan.0, &interface, None, None, None) })
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let radio = wifi_radio().ok_or("sem rádio Wi-Fi")?;
    radio.SetStateAsync(if enabled { RadioState::On } else { RadioState::Off }).err_str()?.get().err_str()?;
    Ok(())
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Perfil WLAN para uma rede pessoal (`authentication` = open, WPAPSK, WPA2PSK ou WPA3SAE). O SSID
/// vai também em hexadecimal, que é o que o Windows usa quando o nome não é ASCII.
fn profile_xml(ssid: &str, authentication: &str, password: Option<&str>) -> String {
    let hex: String = ssid.bytes().map(|b| format!("{b:02X}")).collect();
    let name = escape_xml(ssid);
    let (encryption, key) = match password {
        Some(password) if authentication != "open" => (
            "AES",
            format!("<sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{}</keyMaterial></sharedKey>", escape_xml(password)),
        ),
        _ => ("none", String::new()),
    };
    format!(
        r#"<?xml version="1.0"?><WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1"><name>{name}</name><SSIDConfig><SSID><hex>{hex}</hex><name>{name}</name></SSID></SSIDConfig><connectionType>ESS</connectionType><connectionMode>auto</connectionMode><MSM><security><authEncryption><authentication>{authentication}</authentication><encryption>{encryption}</encryption><useOneX>false</useOneX></authEncryption>{key}</security></MSM></WLANProfile>"#
    )
}

fn authentication(auth: DOT11_AUTH_ALGORITHM) -> Option<&'static str> {
    match auth {
        DOT11_AUTH_ALGO_80211_OPEN => Some("open"),
        DOT11_AUTH_ALGO_WPA_PSK => Some("WPAPSK"),
        DOT11_AUTH_ALGO_RSNA_PSK => Some("WPA2PSK"),
        DOT11_AUTH_ALGO_WPA3_SAE => Some("WPA3SAE"),
        // Corporativas (802.1X), WEP e OWE ficam para as Configurações.
        _ => None,
    }
}

pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    let wlan = Wlan::open()?;
    let interface = wlan.interface().ok_or("sem interface Wi-Fi")?;
    let networks = wlan.networks(&interface).map_err(|code| check(code).unwrap_err())?;
    let target = networks.iter().filter(|n| n.ssid == ssid).max_by_key(|n| n.quality).ok_or("rede não encontrada")?;

    let mut created = false;
    let profile = if password.is_none() && !target.profile.is_empty() {
        target.profile.clone()
    } else {
        let auth = authentication(target.auth).ok_or(ERR_UNSUPPORTED)?;
        if auth != "open" && password.is_none() {
            return Err(ERR_AUTH.into());
        }
        let xml = profile_xml(ssid, auth, password);
        let mut reason = 0;
        check(unsafe { WlanSetProfile(wlan.0, &interface, 0, &HSTRING::from(xml), PCWSTR::null(), true, None, &mut reason) })?;
        created = !target.known;
        ssid.to_string()
    };

    let profile_name = HSTRING::from(profile.as_str());
    let params = WLAN_CONNECTION_PARAMETERS {
        wlanConnectionMode: wlan_connection_mode_profile,
        strProfile: PCWSTR(profile_name.as_ptr()),
        pDot11Ssid: std::ptr::null_mut(),
        pDesiredBssidList: std::ptr::null_mut(),
        dot11BssType: dot11_BSS_type_any,
        dwFlags: 0,
    };
    check(unsafe { WlanConnect(wlan.0, &interface, &params, None) })?;

    // O WlanConnect volta na hora; a senha errada só aparece quando a conexão não se completa.
    let started = Instant::now();
    while started.elapsed() < CONNECT_TIMEOUT {
        std::thread::sleep(Duration::from_millis(500));
        if wlan.networks(&interface).is_ok_and(|list| list.iter().any(|n| n.ssid == ssid && n.connected)) {
            return Ok(());
        }
    }
    if created {
        unsafe { WlanDeleteProfile(wlan.0, &interface, &profile_name, None) };
    }
    Err(ERR_AUTH.into())
}

pub fn disconnect() -> Result<(), String> {
    let wlan = Wlan::open()?;
    let interface = wlan.interface().ok_or("sem interface Wi-Fi")?;
    check(unsafe { WlanDisconnect(wlan.0, &interface, None) })
}

pub fn request_permission(_app: &tauri::AppHandle) {
    crate::util::com_init();
    unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from("ms-settings:privacy-location"), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_profile_xml() {
        let xml = profile_xml("Café & Co", "WPA2PSK", Some("p<a>ss\"'"));
        assert!(xml.contains("<name>Café &amp; Co</name>"));
        assert!(xml.contains("<hex>436166C3A9202620436F</hex>"));
        assert!(xml.contains("<authentication>WPA2PSK</authentication><encryption>AES</encryption>"));
        assert!(xml.contains("<keyMaterial>p&lt;a&gt;ss&quot;&apos;</keyMaterial>"));
        let open = profile_xml("Free", "open", None);
        assert!(open.contains("<authentication>open</authentication><encryption>none</encryption>"));
        assert!(!open.contains("sharedKey"));
    }
}
