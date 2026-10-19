//! Janelas no Windows: enumeração com as regras da taskbar e hooks de eventos do sistema.

use std::sync::atomic::{AtomicIsize, Ordering};

use tauri::AppHandle;
use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, HWND, LPARAM, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::Shell::{IShellDispatch4, Shell};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, DispatchMessageW, EnumWindows, GetAncestor, GetForegroundWindow, GetMessageW, GetWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    PostMessageW, SetForegroundWindow, ShowWindow, TranslateMessage, CHILDID_SELF, EVENT_OBJECT_CLOAKED,
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_UNCLOAKED,
    EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GA_ROOTOWNER, GWL_EXSTYLE,
    GW_OWNER, MSG, OBJID_WINDOW, SW_MINIMIZE, SW_RESTORE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_CLOSE,
    WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, PW_RENDERFULLCONTENT,
};

use super::{mark_dirty, WindowInfo, WindowsSnapshot};
use crate::util::{com_init, wide_to_string, ErrStr};

/// Última janela de outro processo que esteve em primeiro plano. Clicar na dock pode mover o foco
/// para o WebView, então "qual app está ativo" é lembrado aqui.
static LAST_FOREGROUND: AtomicIsize = AtomicIsize::new(0);

fn hwnd(value: i64) -> HWND {
    HWND(value as isize as *mut _)
}

/// Mesmas regras de "aparece na taskbar" que o explorer usa, mais o filtro de janelas cloaked
/// (outras áreas de trabalho virtuais, apps UWP suspensos).
fn is_app_window(h: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(h).as_bool() {
            return false;
        }
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        let app_window = ex & WS_EX_APPWINDOW.0 != 0;
        if !app_window {
            if ex & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) != 0 {
                return false;
            }
            if GetWindow(h, GW_OWNER).map(|o| !o.is_invalid()).unwrap_or(false) {
                return false;
            }
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
        if cloaked != 0 {
            return false;
        }
        GetWindowTextLengthW(h) > 0
    }
}

fn window_pid(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(h, Some(&mut pid));
    }
    pid
}

pub(crate) fn process_path(pid: u32) -> String {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        if ok.is_ok() {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

fn window_aumid(h: HWND) -> Option<String> {
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(h).ok()?;
        let value = store.GetValue(&PKEY_AppUserModel_ID).ok()?;
        let pwstr = PropVariantToStringAlloc(&value).ok()?;
        let s = pwstr.to_string().ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.0 as *const _));
        s.filter(|s| !s.is_empty())
    }
}

fn window_title(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(h, &mut buf) } as usize;
    wide_to_string(&buf[..len])
}

unsafe extern "system" fn collect(h: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<HWND>);
    list.push(h);
    TRUE
}

pub fn snapshot() -> WindowsSnapshot {
    com_init();
    let mut handles: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut handles as *mut _ as isize));
    }
    let own_pid = std::process::id();
    let windows = handles
        .into_iter()
        .filter(|&h| is_app_window(h))
        .filter_map(|h| {
            let pid = window_pid(h);
            if pid == own_pid {
                return None;
            }
            Some(WindowInfo {
                id: h.0 as isize as i64,
                title: window_title(h),
                exe: process_path(pid),
                aumid: window_aumid(h),
                minimized: unsafe { IsIconic(h).as_bool() },
            })
        })
        .collect();
    WindowsSnapshot { windows, foreground: foreground() }
}

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    h: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if id_object != OBJID_WINDOW.0 || id_child != CHILDID_SELF as i32 {
        return;
    }
    if event == EVENT_SYSTEM_FOREGROUND && !h.is_invalid() && window_pid(h) != std::process::id() {
        let root = GetAncestor(h, GA_ROOTOWNER);
        let root = if root.is_invalid() { h } else { root };
        LAST_FOREGROUND.store(root.0 as isize, Ordering::SeqCst);
    }
    mark_dirty();
}

/// Hooks de eventos do sistema, numa thread com message loop.
pub fn start(_app: AppHandle) {
    unsafe {
        let fg = GetForegroundWindow();
        if !fg.is_invalid() && window_pid(fg) != std::process::id() {
            LAST_FOREGROUND.store(fg.0 as isize, Ordering::SeqCst);
        }
    }

    std::thread::spawn(|| unsafe {
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let ranges = [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
            (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
            (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
        ];
        for (min, max) in ranges {
            SetWinEventHook(min, max, None, Some(on_win_event), 0, 0, flags);
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

pub fn foreground() -> i64 {
    LAST_FOREGROUND.load(Ordering::SeqCst) as i64
}

pub fn is_minimized(id: i64) -> bool {
    unsafe { IsIconic(hwnd(id)).as_bool() }
}

pub fn activate(id: i64) {
    let h = hwnd(id);
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        // Anexar a fila de input à da janela em foco libera o SetForegroundWindow.
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let own_thread = GetCurrentThreadId();
        let attached = fg_thread != own_thread && AttachThreadInput(own_thread, fg_thread, true).as_bool();
        let _ = SetForegroundWindow(h);
        let _ = BringWindowToTop(h);
        if attached {
            let _ = AttachThreadInput(own_thread, fg_thread, false);
        }
    }
    LAST_FOREGROUND.store(h.0 as isize, Ordering::SeqCst);
}

pub fn minimize(id: i64) {
    unsafe {
        let _ = ShowWindow(hwnd(id), SW_MINIMIZE);
    }
}

pub fn close(id: i64) {
    unsafe {
        let _ = PostMessageW(hwnd(id), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

/// O mesmo "mostrar área de trabalho" do botão do canto da barra de tarefas: o shell minimiza
/// todas as janelas e, chamado de novo, as restaura.
pub fn toggle_desktop() -> Result<(), String> {
    com_init();
    unsafe {
        let shell: IShellDispatch4 = CoCreateInstance(&Shell, None, CLSCTX_ALL).err_str()?;
        shell.ToggleDesktop().err_str()
    }
}

/// `PrintWindow` com `PW_RENDERFULLCONTENT` desenha num bitmap nosso também as janelas aceleradas
/// (navegadores, apps UWP), sem elas precisarem estar à vista. Minimizada não tem o que desenhar, e
/// o frontend mostra o ícone do app.
pub fn thumbnail(id: i64) -> Option<Vec<u8>> {
    let window = hwnd(id);
    unsafe {
        if IsIconic(window).as_bool() {
            return None;
        }
        let mut rect = RECT::default();
        GetWindowRect(window, &mut rect).ok()?;
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        if width <= 0 || height <= 0 {
            return None;
        }
        let screen = GetDC(HWND::default());
        let dc = CreateCompatibleDC(screen);
        ReleaseDC(HWND::default(), screen);
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Altura negativa: linhas de cima para baixo, como o `RgbaImage`.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let info = BITMAPINFO { bmiHeader: header, ..Default::default() };
        let mut bits = std::ptr::null_mut();
        let pixels = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, HANDLE::default(), 0).ok().and_then(|bitmap| {
            let previous = SelectObject(dc, bitmap);
            let drawn = PrintWindow(window, dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)).as_bool();
            let pixels = drawn.then(|| std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec());
            SelectObject(dc, previous);
            let _ = DeleteObject(bitmap);
            pixels
        });
        let _ = DeleteDC(dc);
        let mut pixels = pixels?;
        // BGRA do GDI, com o alfa zerado na maior parte das janelas.
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        let image = image::RgbaImage::from_raw(width as u32, height as u32, pixels)?;
        super::png_thumbnail(crop_to_visible_frame(window, &rect, image))
    }
}

/// O `GetWindowRect` inclui a moldura invisível do DWM (a borda de redimensionar, uns 7 px em
/// volta); a parte visível é a `DWMWA_EXTENDED_FRAME_BOUNDS`.
fn crop_to_visible_frame(window: HWND, rect: &RECT, image: image::RgbaImage) -> image::RgbaImage {
    let mut visible = RECT::default();
    let size = std::mem::size_of::<RECT>() as u32;
    if unsafe { DwmGetWindowAttribute(window, DWMWA_EXTENDED_FRAME_BOUNDS, &mut visible as *mut RECT as *mut _, size) }.is_err() {
        return image;
    }
    let x = (visible.left - rect.left).clamp(0, image.width() as i32) as u32;
    let y = (visible.top - rect.top).clamp(0, image.height() as i32) as u32;
    let width = ((visible.right - visible.left).max(1) as u32).min(image.width() - x);
    let height = ((visible.bottom - visible.top).max(1) as u32).min(image.height() - y);
    if width == 0 || height == 0 {
        return image;
    }
    image::imageops::crop_imm(&image, x, y, width, height).to_image()
}
