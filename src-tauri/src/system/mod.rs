//! Itens de "bandeja": bateria, rede, volume, Bluetooth e atalhos para os menus nativos do sistema
//! (Menu Iniciar / grade de apps, configurações rápidas, notificações). Cada sistema tem um backend
//! com `battery`, `network`, `volume`, `set_volume`, `bluetooth`, `set_bluetooth` e `open_shell`.

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

use serde::Serialize;

use crate::util::ErrStr;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub present: bool,
    pub percent: u8,
    pub charging: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub connected: bool,
    pub internet: bool,
    pub wifi: bool,
    pub signal_bars: Option<u8>,
    pub name: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub battery: Battery,
    pub network: Network,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub level: f32,
    pub muted: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothDevice {
    pub name: String,
    /// "audio", "mouse", "keyboard", "gamepad", "phone" ou "other".
    pub kind: &'static str,
    pub battery: Option<u8>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bluetooth {
    /// O computador tem um adaptador Bluetooth.
    pub available: bool,
    pub powered: bool,
    /// Dá para ligar e desligar pelo Pier (no macOS não há API pública para isso).
    pub can_toggle: bool,
    /// Só os conectados.
    pub devices: Vec<BluetoothDevice>,
}

impl Bluetooth {
    fn absent() -> Self {
        Bluetooth { available: false, powered: false, can_toggle: false, devices: Vec::new() }
    }
}

/// Tipo do dispositivo pelo Class of Device do Bluetooth clássico (Assigned Numbers, "Baseband").
#[cfg_attr(target_os = "linux", allow(dead_code))]
fn kind_from_class(class: u32) -> &'static str {
    const MAJOR_PHONE: u32 = 0x02;
    const MAJOR_AUDIO: u32 = 0x04;
    const MAJOR_PERIPHERAL: u32 = 0x05;
    let major = (class >> 8) & 0x1F;
    let minor = (class >> 2) & 0x3F;
    match major {
        MAJOR_PHONE => "phone",
        MAJOR_AUDIO => "audio",
        // Periférico: os bits altos do minor dizem teclado (01) ou apontador (10); os baixos,
        // joystick (0001) ou gamepad (0010).
        MAJOR_PERIPHERAL if matches!(minor & 0x0F, 1 | 2) => "gamepad",
        MAJOR_PERIPHERAL if (minor >> 4) & 0x3 == 2 => "mouse",
        MAJOR_PERIPHERAL if (minor >> 4) & 0x3 != 0 => "keyboard",
        _ => "other",
    }
}

impl Battery {
    fn absent() -> Self {
        Battery { present: false, percent: 0, charging: false }
    }
}

impl Network {
    fn disconnected() -> Self {
        Network { connected: false, internet: false, wifi: false, signal_bars: None, name: String::new() }
    }
}

#[tauri::command]
pub async fn system_status() -> Result<SystemStatus, String> {
    tauri::async_runtime::spawn_blocking(|| SystemStatus { battery: backend::battery(), network: backend::network() })
        .await
        .err_str()
}

#[tauri::command]
pub async fn get_volume() -> Result<Volume, String> {
    tauri::async_runtime::spawn_blocking(backend::volume).await.err_str()?
}

#[tauri::command]
pub async fn set_volume(level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::set_volume(level.map(|l| l.clamp(0.0, 1.0)), muted))
        .await
        .err_str()?
}

/// Separado do `system_status` porque no macOS a primeira leitura pede a permissão de Bluetooth: só
/// roda com o widget ligado.
#[tauri::command]
pub async fn bluetooth_status() -> Result<Bluetooth, String> {
    tauri::async_runtime::spawn_blocking(backend::bluetooth).await.err_str()
}

#[tauri::command]
pub async fn set_bluetooth(powered: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::set_bluetooth(powered)).await.err_str()?
}

/// Abre a gaveta de ícones ocultos da bandeja do Windows acima de `anchor_x` (px da dock). Nos
/// outros sistemas não existe.
#[tauri::command]
pub async fn open_tray(app: tauri::AppHandle, anchor_x: f64) -> Result<(), String> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        let dock = app.get_webview_window(crate::dock_window::DOCK_LABEL).ok_or("dock não encontrada")?;
        let (reserved_top, scale, _, _) = crate::dock_window::reserved_top_and_scale(&dock).ok_or("monitor não encontrado")?;
        let x = dock.outer_position().err_str()?.x as f64 + anchor_x * scale;
        let bottom = reserved_top - 8.0 * scale;
        tauri::async_runtime::spawn_blocking(move || backend::open_tray(x.round() as i32, bottom.round() as i32)).await.err_str()?
    }
    #[cfg(not(windows))]
    {
        let _ = (app, anchor_x);
        Err("a bandeja só existe no Windows".into())
    }
}

/// Abre os menus e apps nativos: `start`, `quick-settings`, `notifications`, `system-monitor`,
/// `calendar`, `bluetooth` e `sound` (as configurações de Bluetooth e de som).
#[tauri::command]
pub fn open_shell(kind: String) {
    backend::open_shell(&kind);
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "smoke test: lê o estado real do sistema"]
    fn reads_system_status() {
        println!("{:?}", super::backend::battery());
        println!("{:?}", super::backend::network());
        println!("{:?}", super::backend::volume());
        println!("{:?}", super::backend::bluetooth());
    }

    #[test]
    fn maps_class_of_device_to_kind() {
        assert_eq!(super::kind_from_class(0x240404), "audio");
        assert_eq!(super::kind_from_class(0x002580), "mouse");
        assert_eq!(super::kind_from_class(0x002540), "keyboard");
        assert_eq!(super::kind_from_class(0x002508), "gamepad");
        assert_eq!(super::kind_from_class(0x5a020c), "phone");
        assert_eq!(super::kind_from_class(0x00010c), "other");
    }
}
