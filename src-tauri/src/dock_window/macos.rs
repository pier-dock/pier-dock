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

use super::{at_dock_edge, clickable_area, hit_rect, layout, place_window, set_visible, want_visible, Band, HOVER_GRACE_MS};
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

pub fn init(window: &WebviewWindow) {
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
    pointer::forward(window);
    apply_layout(window);
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
    start_pointer_thread(window.clone());
}

pub fn apply_layout(window: &WebviewWindow) {
    refresh_insets(window);
    place_window(window);
}

/// Mesma lógica do `start_pointer_thread` do Windows. Posição e tamanho da janela só mudam com o
/// layout, então são relidos a cada meio segundo em vez de a cada volta (cada leitura é uma ida
/// à thread principal).
fn start_pointer_thread(window: WebviewWindow) {
    std::thread::spawn(move || {
        let app = window.app_handle().clone();
        let mut ignoring = true;
        let mut hover_until = Instant::now();
        let mut frame: Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> = None;
        let mut frame_read = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(if ignoring { 30 } else { 16 }));
            if frame.is_none() || frame_read.elapsed() > Duration::from_millis(500) {
                frame = match (window.outer_position(), window.outer_size(), window.scale_factor()) {
                    (Ok(pos), Ok(size), Ok(scale)) => Some((pos, size, scale)),
                    _ => None,
                };
                frame_read = Instant::now();
            }
            let (Some((pos, size, scale)), Ok(cursor)) = (frame, app.cursor_position()) else { continue };
            let hit = hit_rect();
            if hit.width <= 0.0 {
                continue;
            }
            let window_band = Band::new(pos.x, pos.y, pos.x + size.width as i32, pos.y + size.height as i32);
            let headroom = if ignoring { 0.0 } else { layout().hover_headroom };
            let window_size = (window_band.width() as f64 / scale, window_band.height() as f64 / scale);
            // A área clicável vai da barra até dentro da tela; a margem entre a barra e a borda
            // do monitor também conta como "na dock".
            let (x0, y0, x1, y1) = clickable_area(window_size, headroom);
            let (win_left, win_top) = (pos.x as f64, pos.y as f64);
            let (left, top) = (win_left + x0 * scale, win_top + y0 * scale);
            let (right, bottom) = (win_left + x1 * scale, win_top + y1 * scale);
            let (cx, cy) = (cursor.x, cursor.y);
            let over_dock = super::dock_visible() && cx >= left && cx <= right && cy >= top && cy <= bottom;
            // O cursor para na última linha de pontos da tela, um ponto (`scale` pixels) antes da
            // borda onde a dock encosta.
            let at_edge = at_dock_edge(cx, cy, window_band, scale);

            let now = Instant::now();
            if over_dock || at_edge {
                hover_until = now + Duration::from_millis(HOVER_GRACE_MS);
            }
            set_visible(&window, want_visible(now < hover_until, crate::windows_list::app_has_focus));

            if over_dock == ignoring {
                ignoring = !over_dock;
                let _ = window.set_ignore_cursor_events(ignoring);
                if ignoring {
                    let _ = window.emit("dock-pointer-left", ());
                }
            }
        }
    });
}

pub fn hit_rect_changed(_app: &AppHandle) {}

pub fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    app.cursor_position().ok().map(|c| (c.x, c.y))
}

/// O hover vem da thread que acompanha o cursor.
pub fn set_hover(_hovering: bool) {}
