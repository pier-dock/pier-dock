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
    /// Endereço no macOS, caminho do objeto no BlueZ e id do `DeviceInformation` no Windows.
    pub id: String,
    pub name: String,
    /// "audio", "mouse", "keyboard", "gamepad", "phone" ou "other".
    pub kind: &'static str,
    pub battery: Option<u8>,
    pub connected: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bluetooth {
    /// O computador tem um adaptador Bluetooth.
    pub available: bool,
    pub powered: bool,
    /// Dá para ligar e desligar pelo Pier (no macOS não há API pública para isso).
    pub can_toggle: bool,
    /// Dá para conectar e desconectar pelo Pier; senão o clique abre as configurações.
    pub can_connect: bool,
    /// Os pareados, com os conectados primeiro.
    pub devices: Vec<BluetoothDevice>,
}

impl Bluetooth {
    fn absent() -> Self {
        Bluetooth { available: false, powered: false, can_toggle: false, can_connect: false, devices: Vec::new() }
    }
}

fn sort_devices(devices: &mut [BluetoothDevice]) {
    devices.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
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
    tauri::async_runtime::spawn_blocking(|| {
        let mut bluetooth = backend::bluetooth();
        sort_devices(&mut bluetooth.devices);
        bluetooth
    })
    .await
    .err_str()
}

#[tauri::command]
pub async fn set_bluetooth(powered: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::set_bluetooth(powered)).await.err_str()?
}

/// Espera a conexão terminar: assim o popover sabe se ela falhou.
#[tauri::command]
pub async fn bluetooth_connect(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::connect_bluetooth(&id)).await.err_str()?
}

#[tauri::command]
pub async fn bluetooth_disconnect(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::disconnect_bluetooth(&id)).await.err_str()?
}

/// Abre a gaveta de ícones ocultos da bandeja do Windows encostada na dock, centrada no widget
/// (`anchor_x`/`anchor_y` em pixels lógicos da dock). Nos outros sistemas não existe.
#[tauri::command]
pub async fn open_tray(app: tauri::AppHandle, anchor_x: f64, anchor_y: f64) -> Result<(), String> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        let dock = app.get_webview_window(crate::dock_window::DOCK_LABEL).ok_or("dock não encontrada")?;
        let reserved = crate::dock_window::reserved_rect(&dock).ok_or("monitor não encontrado")?;
        let dock_pos = dock.outer_position().err_str()?;
        let scale = reserved.scale;
        // O ponto de ancoragem fica um pouco fora da faixa reservada; a gaveta cresce para dentro
        // da tela a partir dele.
        let gap = 8.0 * scale;
        let (x, y) = match crate::dock_window::position() {
            crate::dock_window::TOP => (dock_pos.x as f64 + anchor_x * scale, reserved.band.bottom as f64 + gap),
            crate::dock_window::LEFT => (reserved.band.right as f64 + gap, dock_pos.y as f64 + anchor_y * scale),
            crate::dock_window::RIGHT => (reserved.band.left as f64 - gap, dock_pos.y as f64 + anchor_y * scale),
            _ => (dock_pos.x as f64 + anchor_x * scale, reserved.band.top as f64 - gap),
        };
        let edge = crate::dock_window::position();
        tauri::async_runtime::spawn_blocking(move || backend::open_tray(x.round() as i32, y.round() as i32, edge)).await.err_str()?
    }
    #[cfg(not(windows))]
    {
        let _ = (app, anchor_x, anchor_y);
        Err("a bandeja só existe no Windows".into())
    }
}

/// Abre os menus e apps nativos: `start`, `quick-settings`, `notifications`, `system-monitor`,
/// `calendar`, `bluetooth`, `sound` e `network` (as configurações de Bluetooth, de som e de Wi-Fi).
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
    fn sorts_connected_devices_first() {
        let device = |name: &str, connected: bool| super::BluetoothDevice { id: name.into(), name: name.into(), kind: "other", battery: None, connected };
        let mut devices = vec![device("mouse", false), device("Keyboard", true), device("AirPods", false), device("Watch", true)];
        super::sort_devices(&mut devices);
        let order: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(order, ["Keyboard", "Watch", "AirPods", "mouse"]);
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
