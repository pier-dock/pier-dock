//! Dock no Linux. A janela roda em X11/XWayland como `_NET_WM_WINDOW_TYPE_DOCK` (acima das
//! janelas, fora do alt-tab, sem foco). A área de trabalho é reservada pela extensão do GNOME ou,
//! sem ela, por struts X11.
//!
//! Click-through: no XWayland o cursor global não é confiável fora das janelas X11, então em vez
//! de acompanhá-lo a dock recorta a região de input da própria janela — a barra (mais a folga para
//! a magnificação enquanto o mouse está nela) ou, escondida, uma faixa de 2px na borda da tela. O
//! hover vem do próprio WebView (`set_dock_hover`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use gtk::prelude::*;
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{
    auto_hide, clickable_area, edge_name, layout, place_window, position, reveal_strip, set_visible, want_visible, AUTO_HIDE_NEVER, DOCK_LABEL,
    HOVER_GRACE_MS, LEFT, RIGHT, TOP,
};
use crate::platform::linux::{gnome_shell, x11};

static HOVER: AtomicBool = AtomicBool::new(false);

pub fn init(window: &WebviewWindow) {
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(gtk_window) = w.gtk_window() else { return };
        gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
        // Hover pelo GTK, independente do WebView: entrar na faixa de 2px da dock escondida
        // nem sempre gera um mouseenter na página. Sair para um filho (o próprio WebView) chega
        // como `Inferior` e não conta.
        gtk_window.add_events(gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK);
        gtk_window.connect_enter_notify_event(|_, _| {
            HOVER.store(true, Ordering::SeqCst);
            gtk::glib::Propagation::Proceed
        });
        gtk_window.connect_leave_notify_event(|_, event| {
            if event.detail() != gtk::gdk::NotifyType::Inferior {
                HOVER.store(false, Ordering::SeqCst);
            }
            gtk::glib::Propagation::Proceed
        });
    });
    apply_layout(window);
    let _ = window.show();
    let _ = window.set_visible_on_all_workspaces(true);
    update_input_region(window);
    start_visibility_thread(window.clone());
}

/// Posiciona a janela na borda escolhida do monitor primário e reserva a área de trabalho.
pub fn apply_layout(window: &WebviewWindow) {
    let Some(p) = place_window(window) else { return };
    let reserve = auto_hide() == AUTO_HIDE_NEVER;
    gnome_shell::set_reserved(edge_name(), if reserve { layout().reserved.round() as u32 } else { 0 });
    // Com a extensão, quem reserva é o Shell; struts num cliente XWayland nem sempre valem.
    let strut = if reserve && !gnome_shell::available() { Some(x11_edge()) } else { None };
    let band = [p.band.left, p.band.top, p.band.right, p.band.bottom];
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(gtk_window) = w.gtk_window() else { return };
        // Não redimensionável, o GTK trava a janela no tamanho natural do WebView (200px), maior
        // que a dock, e ela sobra para baixo da tela. (O tao reaplica o `resizable: false` da
        // config no primeiro configure, por isso isto roda a cada layout.)
        gtk_window.set_resizable(true);
        // Redimensionável e sem decoração, o tao transforma os 5px da borda em alças de
        // redimensionar (cursor de resize, arrastar muda o tamanho) — justamente onde fica a faixa
        // que revela a dock escondida. "Decorada" desliga isso, e o Mutter nunca desenha moldura
        // em janelas tipo dock.
        gtk_window.set_decorated(true);
        let scale = gtk_window.scale_factor().max(1) as u32;
        gtk_window.resize((p.width / scale) as i32, (p.height / scale) as i32);
        // A janela só tem XID depois de realizada, e o pedido direto ao gerenciador de janelas
        // garante a posição mesmo quando o GTK ainda guarda a altura antiga.
        let xid = gtk_window.window().and_then(|g| g.downcast::<gdkx11::X11Window>().ok()).map(|x| x.xid() as u32);
        if let (Some(x11), Some(xid)) = (x11::get(), xid) {
            match strut {
                Some(edge) => x11.set_strut(xid, edge, band),
                None => x11.clear_strut(xid),
            }
            x11.move_resize(xid, p.x, p.y, p.width, p.height);
        }
    });
    update_input_region(window);
}

/// Borda da tela na linguagem dos struts X11.
fn x11_edge() -> x11::Edge {
    match position() {
        TOP => x11::Edge::Top,
        LEFT => x11::Edge::Left,
        RIGHT => x11::Edge::Right,
        _ => x11::Edge::Bottom,
    }
}

/// Recorta a região que recebe o mouse (em pixels lógicos da janela).
fn update_input_region(window: &WebviewWindow) {
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Some(gdk_window) = w.gtk_window().ok().and_then(|g| g.window()) else { return };
        let size = (gdk_window.width() as f64, gdk_window.height() as f64);
        let (x0, y0, x1, y1) = if super::dock_visible() {
            let headroom = if HOVER.load(Ordering::SeqCst) { layout().hover_headroom } else { 0.0 };
            clickable_area(size, headroom)
        } else {
            reveal_strip(2.0, size)
        };
        let rect = gtk::cairo::RectangleInt::new(x0 as i32, y0 as i32, ((x1 - x0) as i32).max(1), ((y1 - y0) as i32).max(1));
        gdk_window.input_shape_combine_region(&gtk::cairo::Region::create_rectangle(&rect), 0, 0);
    });
}

/// Decide quando a dock aparece ou some (ocultar automaticamente) e mantém a região de input
/// coerente com isso.
fn start_visibility_thread(window: WebviewWindow) {
    std::thread::spawn(move || {
        let mut hover_until = Instant::now();
        let mut last = (super::dock_visible(), false);
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let now = Instant::now();
            let hover = HOVER.load(Ordering::SeqCst);
            if hover {
                hover_until = now + Duration::from_millis(HOVER_GRACE_MS);
            }
            set_visible(&window, want_visible(now < hover_until, crate::windows_list::app_has_focus));
            let state = (super::dock_visible(), hover);
            if state != last {
                last = state;
                update_input_region(&window);
            }
        }
    });
}

pub fn set_hover(hovering: bool) {
    HOVER.store(hovering, Ordering::SeqCst);
}

pub fn hit_rect_changed(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(DOCK_LABEL) {
        update_input_region(&window);
    }
}

/// Nada fica fora do alcance das janelas na borda de cima no Linux.
pub fn top_inset() -> i32 {
    0
}
