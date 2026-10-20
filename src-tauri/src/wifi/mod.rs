//! Popover de Wi-Fi: redes disponíveis, conectar (com senha nas protegidas), desconectar e ligar ou
//! desligar o rádio. Cada sistema tem um backend com `status`, `scan`, `set_enabled`, `connect`,
//! `disconnect` e `request_permission`.
//!
//! Erros que o popover trata de um jeito próprio vêm como códigos: `"auth"` (senha errada) e
//! `"unsupported"` (rede corporativa ou algo que só as configurações do sistema fazem).

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

use serde::Serialize;

use crate::util::ErrStr;

pub const ERR_AUTH: &str = "auth";
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub const ERR_UNSUPPORTED: &str = "unsupported";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    pub ssid: String,
    /// 0–100.
    pub strength: u8,
    pub signal_bars: u8,
    pub secured: bool,
    /// O sistema tem um perfil salvo (conecta sem pedir senha).
    pub known: bool,
    pub connected: bool,
}

impl WifiNetwork {
    fn new(ssid: String, strength: u8, secured: bool) -> Self {
        let strength = strength.min(100);
        WifiNetwork { ssid, strength, signal_bars: strength.div_ceil(20), secured, known: false, connected: false }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct WifiState {
    /// O computador tem uma interface Wi-Fi.
    pub available: bool,
    pub enabled: bool,
    pub can_toggle: bool,
    /// Dá para conectar pelo Pier; senão o clique numa rede abre as configurações.
    pub can_connect: bool,
    /// As redes só aparecem com a permissão de Localização (macOS 14+, Windows 11 24H2+).
    pub needs_permission: bool,
    pub networks: Vec<WifiNetwork>,
}

/// Uma linha por SSID (cada ponto de acesso aparece separado), sem redes ocultas, com a conectada
/// primeiro e o resto pelo sinal.
fn merge(networks: Vec<WifiNetwork>) -> Vec<WifiNetwork> {
    let mut merged: Vec<WifiNetwork> = Vec::new();
    for network in networks.into_iter().filter(|n| !n.ssid.is_empty()) {
        match merged.iter_mut().find(|m| m.ssid == network.ssid) {
            Some(existing) => {
                existing.known |= network.known;
                existing.connected |= network.connected;
                if network.strength > existing.strength {
                    existing.strength = network.strength;
                    existing.signal_bars = network.signal_bars;
                }
            }
            None => merged.push(network),
        }
    }
    merged.sort_by(|a, b| b.connected.cmp(&a.connected).then(b.strength.cmp(&a.strength)).then_with(|| a.ssid.cmp(&b.ssid)));
    merged
}

#[tauri::command]
pub async fn wifi_status() -> Result<WifiState, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut state = backend::status();
        state.networks = merge(std::mem::take(&mut state.networks));
        state
    })
    .await
    .err_str()
}

/// Pede uma busca nova. No Windows e no Linux ela termina em segundo plano e o `wifi_status`
/// seguinte já traz o resultado; no macOS a chamada espera a busca.
#[tauri::command]
pub async fn wifi_scan() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(backend::scan).await.err_str()?
}

#[tauri::command]
pub async fn wifi_set_enabled(enabled: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::set_enabled(enabled)).await.err_str()?
}

/// Espera a conexão terminar: assim o popover sabe se a senha estava errada.
#[tauri::command]
pub async fn wifi_connect(ssid: String, password: Option<String>) -> Result<(), String> {
    let password = password.filter(|p| !p.is_empty());
    tauri::async_runtime::spawn_blocking(move || backend::connect(&ssid, password.as_deref())).await.err_str()?
}

#[tauri::command]
pub async fn wifi_disconnect() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(backend::disconnect).await.err_str()?
}

/// O Windows 11 24H2 só mostra os nomes das redes para apps com acesso à Localização.
#[cfg(windows)]
pub fn needs_location() -> bool {
    backend::status().needs_permission
}

#[tauri::command]
pub fn wifi_request_permission(app: tauri::AppHandle) {
    backend::request_permission(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(ssid: &str, strength: u8) -> WifiNetwork {
        WifiNetwork::new(ssid.into(), strength, true)
    }

    #[test]
    fn merges_access_points_and_sorts() {
        let home = WifiNetwork { connected: true, ..net("Home", 40) };
        let office_2g = WifiNetwork { known: true, ..net("Office", 55) };
        let networks = vec![net("Cafe", 70), office_2g, home, net("", 90), net("Office", 80)];
        let merged = merge(networks);
        let order: Vec<&str> = merged.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(order, ["Home", "Office", "Cafe"]);
        assert_eq!(merged[1].strength, 80);
        assert_eq!(merged[1].signal_bars, 4);
        assert!(merged[1].known);
    }

    #[test]
    fn strength_to_bars() {
        assert_eq!(WifiNetwork::new("a".into(), 0, false).signal_bars, 0);
        assert_eq!(WifiNetwork::new("a".into(), 1, false).signal_bars, 1);
        assert_eq!(WifiNetwork::new("a".into(), 100, false).signal_bars, 5);
        assert_eq!(WifiNetwork::new("a".into(), 250, false).strength, 100);
    }

    #[test]
    #[ignore = "smoke test: lê as redes Wi-Fi do sistema"]
    fn reads_wifi_state() {
        let _ = backend::scan();
        let mut state = backend::status();
        state.networks = merge(state.networks);
        println!("{state:#?}");
    }
}
