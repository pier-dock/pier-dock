//! Dock no macOS: janela flutuante em todos os Spaces, fora do ⌘` e parada no Mission Control,
//! com click-through fora da barra (thread que acompanha o cursor, como no Windows). O macOS não
//! tem API pública para reservar área da tela, então a dock fica por cima das janelas.
//!
//! Sem `FullScreenAuxiliary` a janela não entra nos Spaces de apps em tela cheia: lá a dock some
//! sozinha, como o Windows faz com a thread de tela cheia.

use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSWindow, NSWindowCollectionBehavior};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use super::{
    at_dock_edge, clickable_area_with, hit_rect_for, layout, over_pet_for, place_window, place_window_on, set_extra_visible, set_visible,
    store_extra_visible, visible_for, want_visible, Band, HOVER_GRACE_MS,
};
use crate::platform::macos::pointer;

/// Altura da barra de menus de cada tela, em pixels físicos, pelo canto de cima à esquerda da
/// tela. A dock no topo ficaria atrás dela (nem o macOS nem nós reservamos essa faixa).
static TOP_INSETS: Mutex<Vec<((i32, i32), i32)>> = Mutex::new(Vec::new());

/// Quanto a dock desce quando está na borda de cima do monitor.
pub fn top_inset(monitor: Band) -> i32 {
    // Os pontos do `NSScreen` viram pixels com arredondamento: tolera um pixel de diferença.
    let near = |(x, y): (i32, i32)| (x - monitor.left).abs() <= 1 && (y - monitor.top).abs() <= 1;
    TOP_INSETS.lock().unwrap().iter().find(|(origin, _)| near(*origin)).map_or(0, |(_, inset)| *inset)
}

/// Relê a barra de menus de cada tela. O `NSScreen` só pode ser lido na thread principal; de outra
/// thread, espera a leitura de lá.
fn refresh_insets(window: &WebviewWindow) {
    fn read(mtm: MainThreadMarker) -> Vec<((i32, i32), i32)> {
        let screens = NSScreen::screens(mtm);
        // O `frame` usa pontos com a origem embaixo, à esquerda da tela principal; o Tauri usa o
        // canto de cima, em pixels de cada tela.
        let Some(main_height) = screens.firstObject().map(|s| s.frame().size.height) else { return Vec::new() };
        screens
            .iter()
            .map(|screen| {
                let (frame, visible, scale) = (screen.frame(), screen.visibleFrame(), screen.backingScaleFactor());
                let top = main_height - (frame.origin.y + frame.size.height);
                let menu_bar = (frame.origin.y + frame.size.height) - (visible.origin.y + visible.size.height);
                (((frame.origin.x * scale).round() as i32, (top * scale).round() as i32), (menu_bar * scale).round() as i32)
            })
            .collect()
    }
    let insets = match MainThreadMarker::new() {
        Some(mtm) => read(mtm),
        None => {
            let (tx, rx) = mpsc::channel();
            let _ = window.run_on_main_thread(move || {
                if let Some(mtm) = MainThreadMarker::new() {
                    let _ = tx.send(read(mtm));
                }
            });
            let Ok(insets) = rx.recv_timeout(Duration::from_secs(1)) else { return };
            insets
        }
    };
    *TOP_INSETS.lock().unwrap() = insets;
}

fn set_collection_behavior(window: &WebviewWindow) {
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(ptr) = w.ns_window() else { return };
        let ns_window = unsafe { &*(ptr as *const NSWindow) };
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
    });
}

pub fn init(window: &WebviewWindow) {
    set_collection_behavior(window);
    pointer::forward(window);
    apply_layout(window);
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
    start_pointer_thread(window, set_visible);
}

/// Janela extra: igual à principal, mas com a visibilidade decidida pelo hover do próprio monitor.
/// Começa escondida, como a principal, até a thread de ponteiro decidir.
pub fn init_extra(window: &WebviewWindow, monitor_id: &str) {
    store_extra_visible(window.label(), false);
    set_collection_behavior(window);
    pointer::forward(window);
    // Um monitor recém-plugado ainda não está em `TOP_INSETS`: só a dock principal relê a lista.
    refresh_insets(window);
    let _ = place_window_on(window, monitor_id);
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
    start_pointer_thread(window, set_extra_visible);
}

pub fn apply_layout(window: &WebviewWindow) {
    refresh_insets(window);
    place_window(window);
}

/// Mesma lógica do `start_pointer_thread` do Windows. Posição e tamanho da janela só mudam com o
/// layout, então são relidos a cada meio segundo em vez de a cada volta (cada leitura é uma ida
/// à thread principal). Roda enquanto a janela existir; `update` aplica a visibilidade decidida.
fn start_pointer_thread(window: &WebviewWindow, update: impl Fn(&WebviewWindow, bool) + Send + 'static) {
    let app = window.app_handle().clone();
    let label = window.label().to_string();
    std::thread::spawn(move || {
        let mut ignoring = true;
        let mut hover_until = Instant::now();
        let mut frame: Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64, f64)> = None;
        let mut frame_read = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(if ignoring { 30 } else { 16 }));
            let Some(window) = app.get_webview_window(&label) else { return };
            if frame.is_none() || frame_read.elapsed() > Duration::from_millis(500) {
                frame = match (window.outer_position(), window.outer_size(), window.scale_factor()) {
                    (Ok(pos), Ok(size), Ok(scale)) => Some((pos, size, scale, pointer::primary_scale(&app))),
                    _ => None,
                };
                frame_read = Instant::now();
            }
            let Some((pos, size, scale, primary)) = frame else { continue };
            let Some(cursor) = pointer::cursor_points(&app, primary) else { continue };
            let hit = hit_rect_for(&label);
            if hit.width <= 0.0 {
                continue;
            }
            let window_band = Band::new(pos.x, pos.y, pos.x + size.width as i32, pos.y + size.height as i32);
            let headroom = if ignoring { 0.0 } else { layout().hover_headroom };
            let window_size = (window_band.width() as f64 / scale, window_band.height() as f64 / scale);
            // A área clicável vai da barra até dentro da tela; a margem entre a barra e a borda
            // do monitor também conta como "na dock".
            let (x0, y0, x1, y1) = clickable_area_with(hit, window_size, headroom);
            let (win_left, win_top) = (pos.x as f64, pos.y as f64);
            let (left, top) = (win_left + x0 * scale, win_top + y0 * scale);
            let (right, bottom) = (win_left + x1 * scale, win_top + y1 * scale);
            // O cursor em pixels da tela da dock, como a posição da janela.
            let (cx, cy) = (cursor.0 * scale, cursor.1 * scale);
            let over_pet = over_pet_for(&label, (cx - win_left) / scale, (cy - win_top) / scale);
            let over_dock = visible_for(&label) && ((cx >= left && cx <= right && cy >= top && cy <= bottom) || over_pet);
            // O cursor para na última linha de pontos da tela, um ponto (`scale` pixels) antes da
            // borda onde a dock encosta.
            let at_edge = at_dock_edge(cx, cy, window_band, scale);

            let now = Instant::now();
            if over_dock || at_edge {
                hover_until = now + Duration::from_millis(HOVER_GRACE_MS);
            }
            // A dock fica à mostra com o próprio painel aberto, mesmo com o cursor fora dela.
            let app_focused = || crate::windows_list::app_has_focus() && !crate::panels::owns_open_panel(&label);
            update(&window, want_visible(now < hover_until, app_focused));

            if over_dock == ignoring {
                ignoring = !over_dock;
                let _ = window.set_ignore_cursor_events(ignoring);
                if ignoring {
                    let _ = window.emit_to(window.label(), "dock-pointer-left", ());
                }
            }
        }
    });
}

pub fn hit_rect_changed(_app: &AppHandle) {}

/// Em pontos: com telas de escalas diferentes, só eles se comparam entre monitores (`contains`).
pub fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    pointer::cursor_points(app, pointer::primary_scale(app))
}

/// O hover vem da thread que acompanha o cursor.
pub fn set_hover(_label: &str, _hovering: bool) {}
