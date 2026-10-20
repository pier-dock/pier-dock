//! Infraestrutura específica de cada sistema, compartilhada pelos módulos de features, e o que o
//! frontend precisa saber sobre o sistema (`platform_info`).

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    /// "windows", "linux" ou "macos".
    pub os: &'static str,
    /// "ok", "needs-extension", "needs-relogin", "needs-accessibility" ou "unsupported".
    pub window_list: &'static str,
    /// Dá para esconder a barra nativa (taskbar, Dash do GNOME, Dock do macOS)?
    pub hide_native: bool,
    /// Os atalhos Ctrl+Alt+1..9 funcionam?
    pub shortcuts: bool,
    /// O sistema desenha o vidro (blur atrás da janela)? Sem ele o material é sempre sólido.
    pub glass: bool,
    /// Dá para ler o cursor em qualquer monitor (a dock pode seguir o cursor)?
    pub follow_cursor: bool,
    /// A área reservada acompanha a dock fora do monitor principal? Falso só com a extensão do
    /// GNOME anterior à versão 4.
    pub reserve_any_monitor: bool,
}

#[tauri::command]
pub async fn platform_info(window: tauri::WebviewWindow) -> Result<PlatformInfo, String> {
    #[allow(unused_mut)]
    let mut info = tauri::async_runtime::spawn_blocking(info).await.map_err(|e| e.to_string())?;
    #[cfg(windows)]
    if matches!(window.label(), crate::panels::SETTINGS_LABEL | crate::panels::WHATS_NEW_LABEL) && !crate::panels::mica_applied() {
        info.glass = false;
    }
    let _ = window;
    Ok(info)
}

#[cfg(windows)]
fn info() -> PlatformInfo {
    PlatformInfo { os: "windows", window_list: "ok", hide_native: true, shortcuts: true, glass: crate::backdrop::supported(), follow_cursor: true, reserve_any_monitor: true }
}

#[cfg(target_os = "linux")]
fn info() -> PlatformInfo {
    linux::info()
}

#[cfg(target_os = "macos")]
fn info() -> PlatformInfo {
    macos::info()
}

/// Ações de configuração do sistema pedidas pela tela de configurações: instalar a extensão do
/// GNOME (`install-extension`) ou pedir a permissão de Acessibilidade do macOS
/// (`request-accessibility`).
#[tauri::command]
pub async fn platform_setup(action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || setup(&action)).await.map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn setup(action: &str) -> Result<(), String> {
    Err(format!("ação não suportada no Windows: {action}"))
}

#[cfg(target_os = "linux")]
fn setup(action: &str) -> Result<(), String> {
    match action {
        "install-extension" => linux::gnome_shell::install_extension(),
        _ => Err(format!("ação não suportada no Linux: {action}")),
    }
}

#[cfg(target_os = "macos")]
fn setup(action: &str) -> Result<(), String> {
    match action {
        "request-accessibility" => {
            macos::ax::request_trust();
            Ok(())
        }
        _ => Err(format!("ação não suportada no macOS: {action}")),
    }
}

/// SIGINT/SIGTERM/SIGHUP: roda `cleanup` (restaurar a barra nativa) e encerra.
#[cfg(unix)]
pub fn on_terminate(cleanup: fn()) {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGINT, SIGTERM, SIGHUP]) else { return };
    std::thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            cleanup();
            std::process::exit(128 + signal);
        }
    });
}
