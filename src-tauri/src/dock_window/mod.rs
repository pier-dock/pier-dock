//! Janela da dock: posicionamento na base do monitor primário, reserva da área de trabalho,
//! click-through fora da barra e ocultação automática.
//!
//! O estado (medidas vindas do frontend, modo de ocultar, visibilidade) é comum; cada sistema tem
//! um backend com `init`, `apply_layout` e `hit_rect_changed`.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;

use serde::Deserialize;
use tauri::{AppHandle, Manager, WebviewWindow};

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

pub use backend::init;
#[cfg(windows)]
pub use backend::{remove_appbar, reregister};

pub const DOCK_LABEL: &str = "dock";

/// A dock está à mostra (ou escondida pelo "ocultar automaticamente").
static VISIBLE: AtomicBool = AtomicBool::new(true);

const AUTO_HIDE_NEVER: u8 = 0;
const AUTO_HIDE_SMART: u8 = 1;
#[allow(dead_code)]
const AUTO_HIDE_ALWAYS: u8 = 2;
static AUTO_HIDE: AtomicU8 = AtomicU8::new(AUTO_HIDE_SMART);

/// Quanto tempo a dock continua à mostra depois que o mouse sai dela.
#[allow(dead_code)]
const HOVER_GRACE_MS: u64 = 700;

/// Medidas em pixels lógicos (CSS), informadas pelo frontend.
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    /// Altura reservada na base da tela (a área que janelas maximizadas não ocupam).
    pub reserved_height: f64,
    /// Altura total da janela, incluindo a folga para a magnificação e tooltips.
    pub window_height: f64,
    /// Quanto a área clicável cresce para cima enquanto o mouse está na dock.
    pub hover_headroom: f64,
}

#[derive(Clone, Copy, Default, Deserialize)]
pub struct HitRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    /// A área clicável vai do topo da barra até a base da tela; a altura só serve ao vidro.
    pub height: f64,
    /// Raio dos cantos da barra, para o vidro acompanhar.
    #[serde(default)]
    pub radius: f64,
}

static LAYOUT: Mutex<Layout> = Mutex::new(Layout { reserved_height: 72.0, window_height: 170.0, hover_headroom: 80.0 });
static HIT: Mutex<HitRect> = Mutex::new(HitRect { x: 0.0, y: 0.0, width: 0.0, height: 0.0, radius: 0.0 });

fn layout() -> Layout {
    *LAYOUT.lock().unwrap()
}

fn hit_rect() -> HitRect {
    *HIT.lock().unwrap()
}

fn auto_hide() -> u8 {
    AUTO_HIDE.load(Ordering::SeqCst)
}

/// A dock deve estar à mostra? `hovering` inclui a folga depois que o mouse sai.
#[allow(dead_code)]
fn want_visible(hovering: bool, app_focused: impl FnOnce() -> bool) -> bool {
    match auto_hide() {
        AUTO_HIDE_NEVER => true,
        AUTO_HIDE_SMART => hovering || !app_focused(),
        _ => hovering,
    }
}

/// Atualiza a visibilidade e avisa o frontend, que anima a dock para dentro/fora da tela; o vidro
/// nativo acompanha.
#[allow(dead_code)]
fn set_visible(window: &WebviewWindow, visible: bool) {
    use tauri::Emitter;
    if VISIBLE.swap(visible, Ordering::SeqCst) != visible {
        let _ = window.emit("dock-visibility", visible);
        crate::backdrop::set_hidden(window, !visible);
    }
}

fn parse_auto_hide(mode: &str) -> u8 {
    match mode {
        "never" => AUTO_HIDE_NEVER,
        "always" => AUTO_HIDE_ALWAYS,
        _ => AUTO_HIDE_SMART,
    }
}

/// Troca o modo de ocultar. Com ocultação ligada a dock não reserva área de trabalho, então
/// janelas maximizadas usam a tela inteira (como a taskbar em "ocultar automaticamente").
pub fn set_auto_hide(app: &AppHandle, mode: &str) {
    let mode = parse_auto_hide(mode);
    if AUTO_HIDE.swap(mode, Ordering::SeqCst) != mode {
        if let Some(window) = app.get_webview_window(DOCK_LABEL) {
            backend::apply_layout(&window);
        }
    }
}

pub fn init_auto_hide(mode: &str) {
    AUTO_HIDE.store(parse_auto_hide(mode), Ordering::SeqCst);
}

#[tauri::command]
pub fn dock_visible() -> bool {
    VISIBLE.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn set_dock_layout(window: WebviewWindow, layout: Layout) {
    *LAYOUT.lock().unwrap() = layout;
    if let Some(dock) = window.app_handle().get_webview_window(DOCK_LABEL) {
        backend::apply_layout(&dock);
    }
}

/// O mouse entrou ou saiu da área clicável da dock. Só o Linux usa: lá o click-through é feito
/// recortando a região de input da janela, e o hover vem do próprio WebView.
#[tauri::command]
pub fn set_dock_hover(hovering: bool) {
    backend::set_hover(hovering);
}

/// A barra mudou de tamanho (itens, magnificação): atualiza a área clicável e o vidro.
#[tauri::command]
pub fn set_hit_rect(app: AppHandle, rect: HitRect) {
    *HIT.lock().unwrap() = rect;
    if let Some(window) = app.get_webview_window(DOCK_LABEL) {
        let glass = crate::backdrop::Rect { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
        crate::backdrop::set_frame(&window, glass, rect.radius);
    }
    backend::hit_rect_changed(&app);
}

/// A faixa de prévias das janelas abriu, mudou de tamanho (`Some`) ou fechou (`None`): o vidro
/// acompanha, como o da barra.
#[tauri::command]
pub fn set_preview_glass(app: AppHandle, rect: Option<HitRect>) {
    let Some(window) = app.get_webview_window(DOCK_LABEL) else { return };
    match rect {
        Some(r) => crate::backdrop::set_named_frame(&window, "preview", crate::backdrop::Rect { x: r.x, y: r.y, width: r.width, height: r.height }, r.radius),
        None => crate::backdrop::remove_named(&window, "preview"),
    }
}

/// Retângulo da área reservada, em pixels físicos de tela — usado para ancorar painéis.
pub fn reserved_top_and_scale(window: &WebviewWindow) -> Option<(f64, f64, f64, f64)> {
    let monitor = window.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let reserved = layout().reserved_height * scale;
    let bottom = monitor.position().y as f64 + monitor.size().height as f64;
    Some((bottom - reserved, scale, monitor.position().x as f64, monitor.size().width as f64))
}

/// Onde a janela da dock fica, em pixels físicos de tela.
#[derive(Clone, Copy, Debug)]
// O macOS não reserva área de trabalho, então só posiciona a janela.
#[cfg_attr(target_os = "macos", allow(dead_code))]
struct Placement {
    x: i32,
    /// Topo da janela.
    y: i32,
    width: u32,
    height: u32,
    /// Base do monitor primário.
    bottom: i32,
    /// Altura que janelas maximizadas não devem ocupar.
    reserved: i32,
}

/// Posiciona a janela na base do monitor primário, ocupando a largura toda.
fn place_window(window: &WebviewWindow) -> Option<Placement> {
    use tauri::{PhysicalPosition, PhysicalSize};
    let layout = layout();
    let monitor = window.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let pos = monitor.position();
    let size = monitor.size();
    let win_h = (layout.window_height * scale).round() as i32;
    let reserved = (layout.reserved_height * scale).round() as i32;
    let bottom = pos.y + size.height as i32;
    let _ = window.set_size(PhysicalSize::new(size.width, win_h as u32));
    let _ = window.set_position(PhysicalPosition::new(pos.x, bottom - win_h));
    Some(Placement { x: pos.x, y: bottom - win_h, width: size.width, height: win_h as u32, bottom, reserved })
}
