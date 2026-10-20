//! Janelas no Linux: pela extensão do GNOME Shell (Wayland ou X11) ou, em sessões X11 de outras
//! DEs, por EWMH.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use tauri::{AppHandle, Emitter};

use super::{mark_dirty, WindowInfo, WindowsSnapshot};
use crate::dock_window::DOCK_LABEL;
use crate::platform::linux::{dbus, desktop_entry, gnome_shell, is_x11_session, x11};

/// Última janela de outro processo que esteve em foco.
static LAST_FOREGROUND: AtomicI64 = AtomicI64::new(0);
/// Um app (não o desktop, a visão geral ou a própria dock) está em foco.
static APP_FOCUSED: AtomicBool = AtomicBool::new(false);

pub fn start(app: AppHandle) {
    gnome_shell::watch(mark_dirty, mark_dirty, move |index| {
        let _ = app.emit_to(DOCK_LABEL, "shortcut-app", index as usize);
    });
    if is_x11_session() {
        x11::watch(mark_dirty);
    }
}

fn x11_session() -> Option<&'static x11::X11> {
    x11::get().filter(|_| is_x11_session())
}

fn exe_of(pid: u32) -> String {
    std::fs::read_link(format!("/proc/{pid}/exe")).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

fn note_focus(id: i64, pid: u32, is_app: bool) {
    let own = pid == std::process::id();
    if !own {
        LAST_FOREGROUND.store(id, Ordering::SeqCst);
    }
    APP_FOCUSED.store(is_app && !own, Ordering::SeqCst);
}

pub fn snapshot() -> WindowsSnapshot {
    let own = std::process::id() as i32;
    let windows = if let Some(snap) = gnome_shell::list_windows() {
        match snap.focus {
            Some(f) => note_focus(f.id as i64, f.pid as u32, !snap.overview),
            None => APP_FOCUSED.store(false, Ordering::SeqCst),
        }
        snap.windows
            .into_iter()
            .filter(|w| w.pid != own)
            .map(|w| WindowInfo {
                id: w.id as i64,
                exe: exe_of(w.pid as u32),
                aumid: w.app_id.or_else(|| w.wm_class.as_deref().and_then(desktop_entry::by_wm_class).map(|e| e.id)),
                title: w.title,
                minimized: w.minimized,
                pid: w.pid as u32,
            })
            .collect()
    } else if let Some(x) = x11_session() {
        match x.active_window() {
            Some(active) => note_focus(active as i64, x.pid(active), true),
            None => APP_FOCUSED.store(false, Ordering::SeqCst),
        }
        x.windows()
            .into_iter()
            .filter(|w| w.pid != own as u32)
            .map(|w| WindowInfo {
                id: w.id as i64,
                exe: exe_of(w.pid),
                aumid: w.wm_class.as_deref().and_then(desktop_entry::by_wm_class).map(|e| e.id),
                title: w.title,
                minimized: w.minimized,
                pid: w.pid,
            })
            .collect()
    } else {
        Vec::new()
    };
    WindowsSnapshot { windows, foreground: LAST_FOREGROUND.load(Ordering::SeqCst) }
}

pub fn app_has_focus() -> bool {
    APP_FOCUSED.load(Ordering::SeqCst)
}

pub fn foreground() -> i64 {
    LAST_FOREGROUND.load(Ordering::SeqCst)
}

pub fn is_minimized(id: i64) -> bool {
    if let Some(snap) = gnome_shell::list_windows() {
        return snap.windows.iter().any(|w| w.id as i64 == id && w.minimized);
    }
    x11_session().is_some_and(|x| x.is_minimized(id as u32))
}

pub fn activate(id: i64) {
    if let Some(shell) = gnome_shell::shell() {
        let _ = shell.activate(id as u64);
    } else if let Some(x) = x11_session() {
        x.activate(id as u32);
    }
    LAST_FOREGROUND.store(id, Ordering::SeqCst);
}

pub fn minimize(id: i64) {
    if let Some(shell) = gnome_shell::shell() {
        let _ = shell.minimize(id as u64);
    } else if let Some(x) = x11_session() {
        x.minimize(id as u32);
    }
}

pub fn close(id: i64) {
    if let Some(shell) = gnome_shell::shell() {
        let _ = shell.close(id as u64);
    } else if let Some(x) = x11_session() {
        x.close(id as u32);
    }
}

/// Mostra a área de trabalho. Em X11 é o `_NET_SHOWING_DESKTOP` do EWMH, que o gerenciador de
/// janelas obedece; no Wayland, cada área de trabalho tem a sua ação: a extensão do GNOME ou o
/// `showDesktop` do KWin (KDE).
pub fn toggle_desktop() -> Result<(), String> {
    if is_x11_session() {
        return x11::get().map(|x| x.toggle_showing_desktop()).ok_or_else(|| "sem conexão com o X11".into());
    }
    if let Some(shell) = gnome_shell::shell() {
        if shell.version().unwrap_or(0) >= gnome_shell::SHOW_DESKTOP_VERSION {
            return shell.toggle_desktop().map_err(|e| e.to_string());
        }
    }
    kwin_toggle_desktop()
}

/// KDE Plasma no Wayland: o KWin expõe o estado e a ação de mostrar a área de trabalho no D-Bus.
fn kwin_toggle_desktop() -> Result<(), String> {
    let conn = dbus::session().ok_or("sem acesso ao D-Bus da sessão")?;
    let kwin = dbus::proxy(conn, "org.kde.KWin", "/KWin", "org.kde.KWin").ok_or("nem a extensão do GNOME nem o KWin responderam")?;
    let showing: bool = kwin.get_property("showingDesktop").map_err(|e| e.to_string())?;
    kwin.call_method("showDesktop", &(!showing,)).map(|_| ()).map_err(|e| e.to_string())
}

/// Prévia de uma janela. Só vale quando a lista vem do X11, aí o id é um XID de verdade; com a
/// extensão do GNOME os ids são do Shell e capturar por eles pegaria outra janela.
pub fn thumbnail(id: i64) -> Option<Vec<u8>> {
    if gnome_shell::list_windows().is_some() {
        return None;
    }
    super::png_thumbnail(x11_session()?.thumbnail(id as u32)?)
}
