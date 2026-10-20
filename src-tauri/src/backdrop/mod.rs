//! Material "vidro líquido": o desfoque nativo do sistema atrás de um retângulo arredondado da
//! página (a barra da dock, um popover). O `backdrop-filter` do CSS só enxerga a própria página,
//! não o que está atrás da janela, então o desfoque tem de vir do sistema:
//!
//! - macOS: `NSGlassEffectView` (macOS 26) ou `NSVisualEffectView` atrás do WebView;
//! - Windows 11: uma janela de fundo com o `HostBackdropBrush` logo abaixo da janela do WebView;
//! - Linux: a região de blur do KWin (`_KDE_NET_WM_BLUR_BEHIND_REGION`).
//!
//! Sem suporte (Windows 10, GNOME) o frontend usa o material sólido. A tinta, a borda e as
//! sombras são sempre do CSS, por cima.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

pub use backend::{forget, refresh, remove_named, set_frame, set_hidden, set_named_frame};

/// Retângulo em pixels CSS, relativo ao topo esquerdo do WebView.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// O material escolhido nas configurações é o vidro (e não o sólido).
static GLASS: AtomicBool = AtomicBool::new(true);

pub fn supported() -> bool {
    backend::supported()
}

/// Desenhar o vidro: o usuário escolheu o vidro e o sistema sabe desenhá-lo.
pub fn active() -> bool {
    GLASS.load(Ordering::SeqCst) && supported()
}

/// O vidro da barra da dock nasce escondido enquanto ela está fora da tela (abrindo o Pier), para
/// aparecer junto com ela.
fn starts_hidden(key: &str) -> bool {
    key == crate::dock_window::DOCK_LABEL && !crate::dock_window::dock_visible()
}

pub fn init_material(material: &str) {
    GLASS.store(material != "solid", Ordering::SeqCst);
}

/// Troca o material de todas as janelas abertas.
pub fn set_material(app: &AppHandle, material: &str) {
    let glass = material != "solid";
    if GLASS.swap(glass, Ordering::SeqCst) == glass {
        return;
    }
    for window in app.webview_windows().values() {
        refresh(window);
    }
    crate::panels::apply_settings_material(app);
}
