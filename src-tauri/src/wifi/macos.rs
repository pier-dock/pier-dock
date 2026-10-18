//! Wi-Fi no macOS pelo CoreWLAN. Desde o macOS 14 os SSIDs só aparecem com a permissão de
//! Localização; sem ela a busca devolve as redes com o nome vazio.

use std::sync::Mutex;

use objc2_core_location::{CLAuthorizationStatus, CLLocationManager};
use objc2_core_wlan::{CWInterface, CWSecurity, CWWiFiClient};
use objc2_foundation::NSString;

use super::{WifiNetwork, WifiState, ERR_AUTH};
use crate::util::{run, spawn_detached};

const PRIVACY_LOCATION: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices";

/// Resultado da última busca: `scanForNetworksWithName` leva alguns segundos, então o `status`
/// (chamado a cada poucos segundos) não busca.
static LAST_SCAN: Mutex<Option<(Vec<WifiNetwork>, bool)>> = Mutex::new(None);

fn interface() -> Option<objc2::rc::Retained<CWInterface>> {
    unsafe { CWWiFiClient::sharedWiFiClient().interface() }
}

/// RSSI em dBm → 0–100 (−50 dBm ou mais = 100), a mesma escala de `system::backend::signal_bars`.
fn strength(rssi: isize) -> u8 {
    (2 * (rssi as i64 + 100)).clamp(0, 100) as u8
}

fn known_ssids(interface: &CWInterface) -> Vec<String> {
    let Some(config) = (unsafe { interface.configuration() }) else { return Vec::new() };
    let profiles = unsafe { config.networkProfiles() };
    (0..profiles.count()).filter_map(|i| unsafe { profiles.objectAtIndex(i).ssid() }).map(|s| s.to_string()).collect()
}

pub fn status() -> WifiState {
    let Some(interface) = interface() else { return WifiState::default() };
    let enabled = unsafe { interface.powerOn() };
    let mut state = WifiState { available: true, enabled, can_toggle: true, can_connect: true, ..Default::default() };
    if !enabled {
        return state;
    }
    let current = unsafe { interface.ssid() }.map(|s| s.to_string());
    let known = known_ssids(&interface);
    let (mut networks, redacted) = LAST_SCAN.lock().unwrap().clone().unwrap_or_default();
    if let Some(ssid) = &current {
        if !networks.iter().any(|n| &n.ssid == ssid) {
            networks.push(WifiNetwork::new(ssid.clone(), strength(unsafe { interface.rssiValue() }), true));
        }
    }
    for network in &mut networks {
        network.connected = current.as_ref() == Some(&network.ssid);
        network.known = network.connected || known.contains(&network.ssid);
    }
    state.needs_permission = redacted && networks.is_empty();
    state.networks = networks;
    state
}

pub fn scan() -> Result<(), String> {
    let interface = interface().ok_or("sem interface Wi-Fi")?;
    let found = unsafe { interface.scanForNetworksWithName_error(None) }.map_err(|e| e.localizedDescription().to_string())?;
    let mut redacted = false;
    let mut networks = Vec::new();
    for network in found.iter() {
        let Some(ssid) = (unsafe { network.ssid() }).map(|s| s.to_string()).filter(|s| !s.is_empty()) else {
            redacted = true;
            continue;
        };
        let secured = !unsafe { network.supportsSecurity(CWSecurity::None) };
        networks.push(WifiNetwork::new(ssid, strength(unsafe { network.rssiValue() }), secured));
    }
    *LAST_SCAN.lock().unwrap() = Some((networks, redacted));
    Ok(())
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let interface = interface().ok_or("sem interface Wi-Fi")?;
    unsafe { interface.setPower_error(enabled) }.map_err(|e| e.localizedDescription().to_string())
}

pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    let interface = interface().ok_or("sem interface Wi-Fi")?;
    let Some(password) = password else {
        // Sem senha (rede salva ou aberta): o `networksetup` usa a senha guardada no Keychain do
        // sistema, que o CoreWLAN não lê sem pedir acesso ao usuário.
        let name = unsafe { interface.interfaceName() }.map(|n| n.to_string()).ok_or("sem interface Wi-Fi")?;
        let output = run("/usr/sbin/networksetup", &["-setairportnetwork", &name, ssid]).ok_or("falha ao conectar")?;
        // Ele sai com 0 mesmo quando falha e só avisa na saída ("Could not find network…").
        return if output.trim().is_empty() { Ok(()) } else { Err(ERR_AUTH.into()) };
    };
    let found = unsafe { interface.scanForNetworksWithName_error(Some(&NSString::from_str(ssid))) }
        .map_err(|e| e.localizedDescription().to_string())?;
    let network = found.iter().next().ok_or_else(|| format!("rede não encontrada: {ssid}"))?;
    // -3924 (kCWInvalidFormatErr) e -3905 (kCWTimeoutErr) são o que a senha errada costuma dar.
    unsafe { interface.associateToNetwork_password_error(&network, Some(&NSString::from_str(password))) }.map_err(|e| match e.code() {
        -3924 | -3905 => ERR_AUTH.to_string(),
        _ => e.localizedDescription().to_string(),
    })
}

pub fn disconnect() -> Result<(), String> {
    let interface = interface().ok_or("sem interface Wi-Fi")?;
    unsafe { interface.disassociate() };
    Ok(())
}

/// Na primeira vez o sistema mostra o pedido; depois de negado, só pelas Configurações.
pub fn request_permission(app: &tauri::AppHandle) {
    let status = unsafe { CLLocationManager::new().authorizationStatus() };
    if status != CLAuthorizationStatus::NotDetermined {
        let _ = spawn_detached("/usr/bin/open", &[PRIVACY_LOCATION]);
        return;
    }
    let _ = app.run_on_main_thread(|| {
        let manager = unsafe { CLLocationManager::new() };
        unsafe { manager.requestWhenInUseAuthorization() };
        // O pedido some se o gerenciador for liberado antes da resposta.
        std::mem::forget(manager);
    });
}
