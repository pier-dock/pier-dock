//! Esconde e restaura a barra nativa do sistema: a taskbar do Windows, o Dash do GNOME ou o Dock
//! do macOS. O estado original é guardado de forma que um crash ou `--restore-taskbar` consigam
//! devolvê-la exatamente como estava.

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

pub use backend::{hide, restore};
#[cfg(windows)]
pub use backend::{show_for_tray, start_watchdog};

/// Garante que a barra nativa volte em pânico e quando o processo é encerrado pelo terminal
/// (Ctrl+C no `tauri dev`, fechar o console, SIGTERM).
pub fn install_crash_guards() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("{info}");
        restore();
        default_hook(info);
    }));
    backend::install_signal_guard();
}
