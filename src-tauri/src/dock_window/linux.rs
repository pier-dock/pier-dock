//! Dock no Linux. A janela roda em X11/XWayland como `_NET_WM_WINDOW_TYPE_DOCK` (acima das
//! janelas, fora do alt-tab, sem foco). A área de trabalho é reservada pela extensão do GNOME ou,
//! sem ela, por struts X11.
//!
//! Click-through: no XWayland o cursor global não é confiável fora das janelas X11, então em vez
//! de acompanhá-lo a dock recorta a região de input da própria janela — a barra (mais a folga para
//! a magnificação enquanto o mouse está nela) ou, escondida, uma faixa de 2px na borda da tela. O
//! hover vem do próprio WebView (`set_dock_hover`).

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{
    auto_hide, clickable_area_with, edge_name, hit_rect_for, is_dock_bar, layout, pet_rect_for, place_window, place_window_on, position, reveal_strip, set_visible,
    want_visible, Placement, AUTO_HIDE_NEVER, HOVER_GRACE_MS, LEFT, RIGHT, TOP,
};
use crate::platform::linux::{gnome_shell, x11};

/// As docks com o mouse em cima, por rótulo: a folga de hover só vale na janela do cursor, mas
/// qualquer uma delas revela todas.
static HOVER: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

pub fn init(window: &WebviewWindow) {
    prepare_window(window);
    apply_layout(window);
    let _ = window.show();
    let _ = window.set_visible_on_all_workspaces(true);
    update_input_region(window);
    start_visibility_thread(window.clone());
}

/// Uma dock extra recebe a mesma preparação da principal. Só a visibilidade é decidida pela thread
/// da principal, e só a principal reserva área de trabalho.
pub fn init_extra(window: &WebviewWindow, monitor_id: &str) {
    prepare_window(window);
    apply_layout_extra(window, monitor_id);
    let _ = window.show();
    let _ = window.set_visible_on_all_workspaces(true);
}

/// O que faz da janela uma dock, antes de ela aparecer: o tipo e o hover pelo GTK.
fn prepare_window(window: &WebviewWindow) {
    let w = window.clone();
    let label = window.label().to_string();
    let _ = window.run_on_main_thread(move || {
        let Ok(gtk_window) = w.gtk_window() else { return };
        gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
        // Hover pelo GTK, independente do WebView: entrar na faixa de 2px da dock escondida
        // nem sempre gera um mouseenter na página. Sair para um filho (o próprio WebView) chega
        // como `Inferior` e não conta.
        gtk_window.add_events(gtk::gdk::EventMask::ENTER_NOTIFY_MASK | gtk::gdk::EventMask::LEAVE_NOTIFY_MASK);
        let enter_label = label.clone();
        gtk_window.connect_enter_notify_event(move |_, _| {
            set_hover(&enter_label, true);
            gtk::glib::Propagation::Proceed
        });
        gtk_window.connect_leave_notify_event(move |_, event| {
            if event.detail() != gtk::gdk::NotifyType::Inferior {
                set_hover(&label, false);
            }
            gtk::glib::Propagation::Proceed
        });
        // A região de input depende do tamanho da janela, que o GTK muda depois do pedido: sem
        // isto a faixa que revela a dock escondida ficaria onde a borda estava antes.
        let resized = w.clone();
        gtk_window.connect_configure_event(move |_, _| {
            update_input_region(&resized);
            false
        });
        // Realizada, a janela já tem XID: o layout e a região de input valem antes de ela aparecer,
        // e nenhum clique cai numa janela inteira à mostra.
        gtk_window.realize();
    });
}

/// Posiciona a janela na borda escolhida do monitor da dock e reserva a área de trabalho.
pub fn apply_layout(window: &WebviewWindow) {
    let Some(p) = place_window(window) else { return };
    let reserve = auto_hide() == AUTO_HIDE_NEVER;
    let center = ((p.monitor.left + p.monitor.right) / 2, (p.monitor.top + p.monitor.bottom) / 2);
    gnome_shell::set_reserved(edge_name(), if reserve { layout().reserved.round() as u32 } else { 0 }, center);
    // Com a extensão, quem reserva é o Shell; struts num cliente XWayland nem sempre valem.
    let strut = if reserve && !gnome_shell::available() { Some(x11_edge()) } else { None };
    apply_geometry(window, p, strut);
}

/// Posiciona uma dock extra no monitor dela, sem reservar área de trabalho.
pub fn apply_layout_extra(window: &WebviewWindow, monitor_id: &str) {
    let Some(p) = place_window_on(window, monitor_id) else { return };
    apply_geometry(window, p, None);
}

/// Faz a janela assumir a posição e o tamanho calculados e recorta a região que recebe o mouse.
fn apply_geometry(window: &WebviewWindow, p: Placement, strut: Option<x11::Edge>) {
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

/// Recorta a região que recebe o mouse (em pixels lógicos da janela), com a barra medida por ela.
fn update_input_region(window: &WebviewWindow) {
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Some(gdk_window) = w.gtk_window().ok().and_then(|g| g.window()) else { return };
        let label = w.label();
        let size = (gdk_window.width() as f64, gdk_window.height() as f64);
        let (x0, y0, x1, y1) = if super::dock_visible() {
            let headroom = if is_hovered(label) { layout().hover_headroom } else { 0.0 };
            clickable_area_with(hit_rect_for(label), size, headroom)
        } else {
            reveal_strip(2.0, size)
        };
        let rect = gtk::cairo::RectangleInt::new(x0 as i32, y0 as i32, ((x1 - x0) as i32).max(1), ((y1 - y0) as i32).max(1));
        let region = gtk::cairo::Region::create_rectangle(&rect);
        if let Some(pet) = pet_rect_for(label).filter(|_| super::dock_visible()) {
            let pet = gtk::cairo::RectangleInt::new(pet.x as i32, pet.y as i32, pet.width as i32, pet.height as i32);
            let _ = region.union_rectangle(&pet);
        }
        gdk_window.input_shape_combine_region(&region, 0, 0);
    });
}

/// A região de input de todas as docks, a principal e as extras.
fn update_all_input_regions(app: &AppHandle) {
    for (label, window) in app.webview_windows() {
        if is_dock_bar(&label) {
            update_input_region(&window);
        }
    }
}

/// Decide quando as docks aparecem ou somem (ocultar automaticamente) e mantém a região de input
/// delas coerente com isso.
fn start_visibility_thread(window: WebviewWindow) {
    std::thread::spawn(move || {
        let mut hover_until = Instant::now();
        let mut last = (super::dock_visible(), BTreeSet::new());
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let now = Instant::now();
            let hovered = HOVER.lock().unwrap().clone();
            if !hovered.is_empty() {
                hover_until = now + Duration::from_millis(HOVER_GRACE_MS);
            }
            set_visible(&window, want_visible(now < hover_until, crate::windows_list::app_has_focus));
            let state = (super::dock_visible(), hovered);
            if state != last {
                last = state;
                update_all_input_regions(window.app_handle());
            }
        }
    });
}

pub fn set_hover(label: &str, hovering: bool) {
    let mut hovered = HOVER.lock().unwrap();
    if hovering {
        hovered.insert(label.to_string());
    } else {
        hovered.remove(label);
    }
}

fn is_hovered(label: &str) -> bool {
    HOVER.lock().unwrap().contains(label)
}

pub fn hit_rect_changed(app: &AppHandle) {
    update_all_input_regions(app);
}

/// O cursor global, pela extensão do GNOME quando ela sabe (no XWayland o do X11 só se move sobre
/// janelas X11).
pub fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    if let Some((x, y)) = gnome_shell::pointer() {
        return Some((x as f64, y as f64));
    }
    app.cursor_position().ok().map(|c| (c.x, c.y))
}

/// Nada fica fora do alcance das janelas na borda de cima no Linux.
pub fn top_inset(_monitor: super::Band) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_belongs_to_each_window() {
        set_hover("dock-extra-test-hover", true);
        assert!(is_hovered("dock-extra-test-hover"));
        assert!(!is_hovered("dock-extra-test-other"));
        set_hover("dock-extra-test-other", true);
        set_hover("dock-extra-test-hover", false);
        assert!(!is_hovered("dock-extra-test-hover"));
        assert!(is_hovered("dock-extra-test-other"));
        set_hover("dock-extra-test-other", false);
        assert!(!is_hovered("dock-extra-test-other"));
    }
}
