//! Taskbar do Windows.
//!
//! A taskbar é colocada em "ocultar automaticamente" (para liberar a área de trabalho que ela
//! reserva) e suas janelas são escondidas. O estado original fica salvo em disco, para que um
//! crash ou `pier.exe --restore-taskbar` consigam devolvê-la exatamente como estava.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{BOOL, COLORREF, HWND, LPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetLayeredWindowAttributes, GetWindowLongPtrW, IsWindowVisible, SetForegroundWindow,
    SetLayeredWindowAttributes, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, LAYERED_WINDOW_ATTRIBUTES_FLAGS, LWA_ALPHA, SW_HIDE,
    SW_SHOW, SW_SHOWNA, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};

use crate::util::data_dir;

static HIDDEN: AtomicBool = AtomicBool::new(false);
static SHOWN_FOR_TRAY: AtomicBool = AtomicBool::new(false);
static PRIMARY_TRAY: AtomicIsize = AtomicIsize::new(0);
/// Se fomos nós que pusemos `WS_EX_LAYERED` na taskbar (e então devemos tirá-lo).
static ADDED_LAYERED: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize)]
struct SavedState {
    original_state: u32,
}

fn state_path() -> std::path::PathBuf {
    data_dir().join("taskbar-state.json")
}

fn taskbar_windows() -> Vec<HWND> {
    let mut found = Vec::new();
    unsafe {
        if let Ok(h) = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) {
            if !h.is_invalid() {
                found.push(h);
            }
        }
        let mut prev = HWND::default();
        while let Ok(h) = FindWindowExW(HWND::default(), prev, w!("Shell_SecondaryTrayWnd"), PCWSTR::null()) {
            if h.is_invalid() {
                break;
            }
            found.push(h);
            prev = h;
        }
    }
    found
}

fn appbar_data() -> APPBARDATA {
    APPBARDATA { cbSize: std::mem::size_of::<APPBARDATA>() as u32, ..Default::default() }
}

fn get_state() -> u32 {
    let mut abd = appbar_data();
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut abd) as u32 }
}

fn set_state(state: u32) {
    let mut abd = appbar_data();
    abd.lParam = LPARAM(state as isize);
    unsafe {
        SHAppBarMessage(ABM_SETSTATE, &mut abd);
    }
}

/// Deixa a taskbar invisível e sem receber cliques sem escondê-la de fato: com `SW_HIDE` ela não
/// expõe a seta da bandeja à UI Automation.
fn make_transparent(h: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        if style & WS_EX_LAYERED.0 == 0 {
            ADDED_LAYERED.store(true, Ordering::SeqCst);
        }
        SetWindowLongPtrW(h, GWL_EXSTYLE, (style | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0) as isize);
        let _ = SetLayeredWindowAttributes(h, COLORREF(0), 0, LWA_ALPHA);
    }
}

/// Desfaz `make_transparent`. Alfa 0 só pode ter vindo do Pier, o que permite reconhecer a
/// transparência mesmo em outro processo (`--restore-taskbar` depois de um crash).
fn make_opaque(h: HWND, added_layered: bool) {
    unsafe {
        let (mut alpha, mut flags) = (0u8, LAYERED_WINDOW_ATTRIBUTES_FLAGS(0));
        let ours = GetLayeredWindowAttributes(h, None, Some(&mut alpha as *mut _), Some(&mut flags as *mut _)).is_ok() && flags.contains(LWA_ALPHA) && alpha == 0;
        if !ours {
            return;
        }
        let _ = SetLayeredWindowAttributes(h, COLORREF(0), 255, LWA_ALPHA);
        let mut style = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32 & !WS_EX_TRANSPARENT.0;
        if added_layered {
            style &= !WS_EX_LAYERED.0;
        }
        SetWindowLongPtrW(h, GWL_EXSTYLE, style as isize);
    }
}

fn make_all_opaque() {
    let added_layered = ADDED_LAYERED.swap(false, Ordering::SeqCst);
    for h in taskbar_windows() {
        make_opaque(h, added_layered);
    }
}

fn set_visible(visible: bool) {
    for h in taskbar_windows() {
        unsafe {
            let _ = ShowWindow(h, if visible { SW_SHOW } else { SW_HIDE });
        }
    }
}

pub fn hide() {
    let path = state_path();
    // Se o arquivo já existe, uma execução anterior não restaurou (crash): o estado atual é o
    // nosso "autohide", então mantemos o original salvo.
    if !path.exists() {
        let saved = SavedState { original_state: get_state() };
        let _ = std::fs::write(&path, serde_json::to_string(&saved).unwrap_or_default());
    }
    set_state(ABS_AUTOHIDE);
    set_visible(false);
    if let Some(&tray) = taskbar_windows().first() {
        PRIMARY_TRAY.store(tray.0 as isize, Ordering::SeqCst);
    }
    HIDDEN.store(true, Ordering::SeqCst);
}

pub fn restore() {
    HIDDEN.store(false, Ordering::SeqCst);
    crate::dock_window::remove_appbar();
    let path = state_path();
    if let Some(saved) = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<SavedState>(&s).ok())
    {
        set_state(saved.original_state);
        let _ = std::fs::remove_file(&path);
    }
    make_all_opaque();
    set_visible(true);
}

fn is_hidden() -> bool {
    HIDDEN.load(Ordering::SeqCst)
}

/// Escondida (`SW_HIDE`) ou recolhida, a taskbar não expõe a seta da bandeja à UI Automation. Aqui
/// ela sobe, transparente, só enquanto a seta é acionada: com a gaveta aberta o Windows revela a
/// taskbar inteira, então ela é escondida de novo assim que a gaveta aparece.
pub fn show_for_tray(shown: bool) {
    if !is_hidden() {
        return;
    }
    SHOWN_FOR_TRAY.store(shown, Ordering::SeqCst);
    if shown {
        let windows = taskbar_windows();
        for &h in &windows {
            make_transparent(h);
            unsafe {
                let _ = ShowWindow(h, SW_SHOWNA);
            }
        }
        // Recolhida (ocultar automaticamente), a taskbar não monta os botões da bandeja; ativá-la
        // faz ela subir, como o mouse no fim da tela.
        if let Some(&primary) = windows.first() {
            unsafe {
                let _ = SetForegroundWindow(primary);
            }
        }
    } else {
        set_visible(false);
        make_all_opaque();
    }
}

/// Reaplica o ocultamento se a taskbar reaparecer (o Win11 às vezes a reexibe) e avisa quando o
/// explorer.exe reinicia, já que um explorer novo perde o registro da nossa AppBar.
pub fn start_watchdog(on_explorer_restart: impl Fn() + Send + 'static) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(1500));
        if !is_hidden() || SHOWN_FOR_TRAY.load(Ordering::SeqCst) {
            continue;
        }
        let windows = taskbar_windows();
        let Some(&tray) = windows.first() else { continue };
        if PRIMARY_TRAY.swap(tray.0 as isize, Ordering::SeqCst) != tray.0 as isize {
            set_state(ABS_AUTOHIDE);
            on_explorer_restart();
        }
        for h in windows {
            unsafe {
                if IsWindowVisible(h).as_bool() {
                    let _ = ShowWindow(h, SW_HIDE);
                }
            }
        }
    });
}

unsafe extern "system" fn console_handler(_ctrl: u32) -> BOOL {
    restore();
    BOOL(0)
}

/// Ctrl+C / fechamento do console (`tauri dev`).
pub fn install_signal_guard() {
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(console_handler), true);
    }
}
