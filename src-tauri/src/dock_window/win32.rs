//! Dock no Windows: estilos Win32, registro como AppBar, click-through fora da barra (thread que
//! acompanha o cursor), ocultação automática e ocultação quando um app está em tela cheia.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId,
    IsZoomed, SetWindowLongPtrW, SetWindowPos, WindowFromPoint, GA_ROOT, GWL_EXSTYLE, GW_HWNDNEXT, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_USER, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

use super::{
    at_dock_edge, auto_hide, clickable_area, hit_rect, layout, place_window, position, set_visible, want_visible, Band, AUTO_HIDE_NEVER, BOTTOM,
    DOCK_LABEL, HOVER_GRACE_MS, LEFT, RIGHT, TOP,
};
use crate::util::wide_to_string;

static DOCK_HWND: AtomicIsize = AtomicIsize::new(0);
static APPBAR_REGISTERED: AtomicBool = AtomicBool::new(false);
static HIDDEN_FOR_FULLSCREEN: AtomicBool = AtomicBool::new(false);

fn dock_hwnd() -> HWND {
    HWND(DOCK_HWND.load(Ordering::SeqCst) as *mut _)
}

/// Estilos estendidos que a dock precisa. O tao recalcula os estilos da janela a partir das flags
/// dele em várias operações (show, resize…) e põe `WS_EX_APPWINDOW`, então a thread de ponteiro
/// reaplica isto continuamente; ela também é dona do bit de click-through (`WS_EX_TRANSPARENT`).
fn ensure_ex_style(hwnd: HWND, click_through: bool) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let mut desired = (current & !(WS_EX_APPWINDOW.0 | WS_EX_TRANSPARENT.0))
            | WS_EX_TOOLWINDOW.0
            | WS_EX_NOACTIVATE.0
            | WS_EX_LAYERED.0;
        if click_through {
            desired |= WS_EX_TRANSPARENT.0;
        }
        if desired != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired as isize);
        }
    }
}

/// De quanto em quanto tempo a thread de ponteiro confere se o hit-test do Windows ainda acerta a
/// barra (`hit_test_falls_through`).
const HIT_TEST_CHECK: Duration = Duration::from_millis(250);

/// Com a dock interativa e o cursor sobre a barra, o Windows às vezes entrega o mouse à janela de
/// baixo: a máscara de alfa que ele usa no hit-test da janela layered fica velha e marca a barra
/// como transparente (visto no Windows 11 passando o mouse pelos ícones). Só conta uma janela achada
/// abaixo da dock na ordem Z; uma por cima dela (o Iniciar, um popover) é legítima.
fn hit_test_falls_through(hwnd: HWND, cursor: POINT) -> bool {
    unsafe {
        let root = GetAncestor(WindowFromPoint(cursor), GA_ROOT);
        if root.is_invalid() || root == hwnd {
            return false;
        }
        let mut next = GetWindow(hwnd, GW_HWNDNEXT);
        while let Ok(below) = next {
            if below == root {
                return true;
            }
            next = GetWindow(below, GW_HWNDNEXT);
        }
        false
    }
}

/// Tirar e recolocar o `WS_EX_LAYERED` faz o Windows refazer a máscara de hit-test da janela; o
/// `SWP_FRAMECHANGED` sozinho não basta.
fn refresh_layered(hwnd: HWND) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_LAYERED.0 as isize));
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED;
        let _ = SetWindowPos(hwnd, HWND::default(), 0, 0, 0, 0, flags);
    }
}

pub fn init(window: &WebviewWindow) {
    if let Ok(h) = window.hwnd() {
        DOCK_HWND.store(h.0 as isize, Ordering::SeqCst);
    }
    apply_layout(window);
    // Faz o tao criar a janela já como LAYERED; daqui em diante o bit é gerenciado por nós.
    let _ = window.set_ignore_cursor_events(true);
    let _ = window.show();
    ensure_ex_style(dock_hwnd(), true);
    start_pointer_thread(window.clone());
    start_fullscreen_thread(window.app_handle().clone());
}

/// Posiciona a janela na borda escolhida do monitor primário e reserva a área de trabalho via
/// AppBar.
pub fn apply_layout(window: &WebviewWindow) {
    let Some(p) = place_window(window) else { return };
    if auto_hide() == AUTO_HIDE_NEVER {
        register_appbar(p.band);
    } else {
        remove_appbar();
    }
}

/// Borda da AppBar correspondente à borda da dock.
fn appbar_edge() -> u32 {
    match position() {
        TOP => ABE_TOP,
        LEFT => ABE_LEFT,
        RIGHT => ABE_RIGHT,
        _ => ABE_BOTTOM,
    }
}

fn appbar_data(rect: RECT) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: dock_hwnd(),
        uCallbackMessage: WM_USER + 0x4D,
        uEdge: appbar_edge(),
        rc: rect,
        lParam: LPARAM(0),
    }
}

/// Registra a faixa reservada como AppBar. O shell pode encolher a faixa ao responder
/// `ABM_QUERYPOS`; a espessura escolhida volta no `ABM_SETPOS`, encostada na borda da dock.
fn register_appbar(band: Band) {
    if dock_hwnd().is_invalid() {
        return;
    }
    let thickness = match position() {
        TOP | BOTTOM => band.height(),
        _ => band.width(),
    };
    let mut abd = appbar_data(RECT { left: band.left, top: band.top, right: band.right, bottom: band.bottom });
    unsafe {
        if !APPBAR_REGISTERED.swap(true, Ordering::SeqCst) {
            SHAppBarMessage(ABM_NEW, &mut abd);
        }
        SHAppBarMessage(ABM_QUERYPOS, &mut abd);
        match position() {
            TOP => abd.rc.bottom = abd.rc.top + thickness,
            LEFT => abd.rc.right = abd.rc.left + thickness,
            RIGHT => abd.rc.left = abd.rc.right - thickness,
            _ => abd.rc.top = abd.rc.bottom - thickness,
        }
        SHAppBarMessage(ABM_SETPOS, &mut abd);
    }
}

pub fn remove_appbar() {
    if APPBAR_REGISTERED.swap(false, Ordering::SeqCst) {
        let mut abd = appbar_data(RECT::default());
        unsafe {
            SHAppBarMessage(ABM_REMOVE, &mut abd);
        }
    }
}

/// Chamado quando o explorer.exe reinicia: o registro da AppBar se perdeu com o explorer antigo.
pub fn reregister(app: &AppHandle) {
    APPBAR_REGISTERED.store(false, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window(DOCK_LABEL) {
        apply_layout(&window);
    }
}

/// Algum app (Chrome, VS Code…) está em primeiro plano? Desktop, shell, Menu Iniciar e as
/// janelas da própria dock (painéis, configurações) não contam.
fn app_has_focus(own_pid: u32) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        pid != own_pid && !is_shell_window(fg)
    }
}

/// A janela é mais alta que a barra (folga para a magnificação), mas a área transparente não pode
/// bloquear cliques nas janelas de baixo. Esta thread liga o click-through sempre que o cursor
/// sai da barra e decide quando a dock aparece ou some (ocultar automaticamente).
fn start_pointer_thread(window: WebviewWindow) {
    std::thread::spawn(move || {
        let own_pid = std::process::id();
        let mut ignoring = true;
        let mut hover_until = Instant::now();
        let mut backdrop_checked = Instant::now();
        let mut hit_test_checked = Instant::now();
        let mut was_pressed = false;
        loop {
            std::thread::sleep(Duration::from_millis(if ignoring { 30 } else { 16 }));
            if backdrop_checked.elapsed() > Duration::from_secs(1) {
                backdrop_checked = Instant::now();
                crate::backdrop::refresh(&window);
            }
            let hwnd = dock_hwnd();
            ensure_ex_style(hwnd, ignoring);
            let mut cursor = POINT::default();
            let mut win = RECT::default();
            unsafe {
                if GetCursorPos(&mut cursor).is_err() || GetWindowRect(hwnd, &mut win).is_err() {
                    continue;
                }
            }
            let scale = unsafe { GetDpiForWindow(hwnd) } as f64 / 96.0;
            let hit = hit_rect();
            if hit.width <= 0.0 {
                continue;
            }
            let window_band = Band::new(win.left, win.top, win.right, win.bottom);
            let size = (window_band.width() as f64 / scale, window_band.height() as f64 / scale);
            let headroom = if ignoring { 0.0 } else { layout().hover_headroom };
            // A área clicável vai da barra até dentro da tela; a margem entre a barra e a borda
            // do monitor também conta como "na dock".
            let (x0, y0, x1, y1) = clickable_area(size, headroom);
            let (left, top) = (win.left as f64 + x0 * scale, win.top as f64 + y0 * scale);
            let (right, bottom) = (win.left as f64 + x1 * scale, win.top as f64 + y1 * scale);
            let (cx, cy) = (cursor.x as f64, cursor.y as f64);
            let over_dock = super::dock_visible() && cx >= left && cx <= right && cy >= top && cy <= bottom;
            let pressed = mouse_pressed();
            // Na dock quem decide é o próprio clique (o widget alterna o painel dele).
            if pressed && !was_pressed && !over_dock {
                crate::panels::close_if_outside(window.app_handle(), cursor.x, cursor.y);
            }
            was_pressed = pressed;
            // Encostar o mouse na borda da tela onde a dock vive revela a dock escondida.
            let at_edge = at_dock_edge(cx, cy, window_band, scale);

            let now = Instant::now();
            if over_dock || at_edge {
                hover_until = now + Duration::from_millis(HOVER_GRACE_MS);
            }
            let hovering = now < hover_until;
            set_visible(&window, want_visible(hovering, || app_has_focus(own_pid)));

            let interactive = over_dock;
            if interactive == ignoring {
                ignoring = !interactive;
                ensure_ex_style(hwnd, ignoring);
                if ignoring {
                    let _ = window.emit("dock-pointer-left", ());
                }
            }

            // Só sobre a própria barra, longe dos cantos arredondados: na folga acima dela e na
            // margem até a borda da tela os pixels são transparentes, e lá o mouse atravessa mesmo.
            let inset = (hit.radius.max(2.0) * scale, 2.0 * scale);
            let bar_left = win.left as f64 + hit.x * scale + inset.0;
            let bar_top = win.top as f64 + hit.y * scale + inset.1;
            let bar_right = win.left as f64 + (hit.x + hit.width) * scale - inset.0;
            let bar_bottom = win.top as f64 + (hit.y + hit.height) * scale - inset.1;
            let over_bar = cx > bar_left && cx < bar_right && cy > bar_top && cy < bar_bottom;
            if !ignoring && over_bar && hit_test_checked.elapsed() >= HIT_TEST_CHECK {
                hit_test_checked = Instant::now();
                if hit_test_falls_through(hwnd, cursor) {
                    log::warn!("o hit-test atravessou a barra da dock; refazendo o WS_EX_LAYERED");
                    refresh_layered(hwnd);
                }
            }
        }
    });
}

fn mouse_pressed() -> bool {
    [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON].iter().any(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } < 0)
}

fn is_shell_window(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) } as usize;
    let class = wide_to_string(&buf[..len]);
    matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
}

/// Esconde a dock enquanto um app ocupa a tela inteira do monitor primário (vídeos, jogos).
fn start_fullscreen_thread(app: AppHandle) {
    std::thread::spawn(move || {
        let own_pid = std::process::id();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let Some(window) = app.get_webview_window(DOCK_LABEL) else { continue };
            let fullscreen = unsafe {
                let fg = GetForegroundWindow();
                let mut pid = 0u32;
                GetWindowThreadProcessId(fg, Some(&mut pid));
                if fg.is_invalid() || pid == own_pid || is_shell_window(fg) {
                    None
                } else {
                    let mut rect = RECT::default();
                    let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
                    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                    let ok = GetWindowRect(fg, &mut rect).is_ok() && GetMonitorInfoW(monitor, &mut info).as_bool();
                    // Sem área reservada (ocultar automaticamente), uma janela maximizada também cobre
                    // o monitor inteiro, mas não é tela cheia: a dock tem que poder aparecer por cima.
                    let maximized = IsZoomed(fg).as_bool();
                    // dwFlags & 1 = MONITORINFOF_PRIMARY: a dock só vive no monitor primário.
                    Some(
                        ok && !maximized
                            && info.dwFlags & 1 != 0
                            && rect.left <= info.rcMonitor.left
                            && rect.top <= info.rcMonitor.top
                            && rect.right >= info.rcMonitor.right
                            && rect.bottom >= info.rcMonitor.bottom,
                    )
                }
            };
            // `None`: a própria dock ou o desktop estão em foco; mantém o estado atual.
            let Some(fullscreen) = fullscreen else { continue };
            if fullscreen != HIDDEN_FOR_FULLSCREEN.load(Ordering::SeqCst) {
                HIDDEN_FOR_FULLSCREEN.store(fullscreen, Ordering::SeqCst);
                let _ = if fullscreen { window.hide() } else { window.show() };
                crate::backdrop::set_hidden(&window, fullscreen || !super::dock_visible());
            }
        }
    });
}

pub fn hit_rect_changed(_app: &AppHandle) {}

/// Nada fica fora do alcance das janelas na borda de cima no Windows.
pub fn top_inset() -> i32 {
    0
}

/// O hover vem da thread que acompanha o cursor.
pub fn set_hover(_hovering: bool) {}
