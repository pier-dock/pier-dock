//! Windows 11: o Acrylic do DWM vale para a janela inteira, e a janela da dock é uma faixa
//! transparente da largura da tela (folga para a magnificação e os tooltips). Por isso o vidro é
//! uma janela à parte, sem conteúdo, com o Acrylic recortado no formato da barra e mantida logo
//! abaixo da janela do WebView na ordem Z.
//!
//! O Acrylic vem do `SetWindowCompositionAttribute` (não documentado, mas estável desde o Windows
//! 10 1803): ao contrário do `DWMWA_SYSTEMBACKDROP_TYPE`, ele respeita a região da janela, que dá
//! os cantos arredondados. A região do GDI não tem antialias, então os cantos ficam serrilhados
//! por baixo da borda do CSS.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tauri::WebviewWindow;
use windows::core::{s, w};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_DONOTROUND,
};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, RegisterClassW, SetWindowPos, ShowWindow, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SW_HIDE, WM_NCHITTEST, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::Rect;

/// Primeira versão do Windows 11. No 10 o Acrylic por composição engasga ao mover janelas.
const WINDOWS_11_BUILD: u32 = 22000;
/// Duração da animação de mostrar da barra (`.dock-bar` no CSS): o vidro só aparece no fim dela.
const REVEAL_DELAY: Duration = Duration::from_millis(300);

struct Backdrop {
    hwnd: isize,
    rect: Rect,
    radius: f64,
    hidden: bool,
}

static BACKDROPS: Mutex<Option<HashMap<String, Backdrop>>> = Mutex::new(None);
/// Muda a cada mostrar/esconder, para um "mostrar" atrasado não desfazer um "esconder" posterior.
static VISIBILITY_SEQ: AtomicU64 = AtomicU64::new(0);

fn with_backdrops<T>(f: impl FnOnce(&mut HashMap<String, Backdrop>) -> T) -> T {
    f(BACKDROPS.lock().unwrap().get_or_insert_with(HashMap::new))
}

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service_pack: [u16; 128],
}

/// Build do Windows pelo `RtlGetVersion`: o `GetVersionEx` mente para apps sem manifesto.
fn windows_build() -> u32 {
    static BUILD: OnceLock<u32> = OnceLock::new();
    *BUILD.get_or_init(|| unsafe {
        let Ok(ntdll) = GetModuleHandleW(w!("ntdll.dll")) else { return 0 };
        let Some(proc) = GetProcAddress(ntdll, s!("RtlGetVersion")) else { return 0 };
        let rtl_get_version: unsafe extern "system" fn(*mut OsVersionInfo) -> i32 = std::mem::transmute(proc);
        let mut info = OsVersionInfo { size: std::mem::size_of::<OsVersionInfo>() as u32, major: 0, minor: 0, build: 0, platform: 0, service_pack: [0; 128] };
        if rtl_get_version(&mut info) == 0 {
            info.build
        } else {
            0
        }
    })
}

pub fn supported() -> bool {
    windows_build() >= WINDOWS_11_BUILD
}

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct CompositionAttributeData {
    attribute: u32,
    data: *mut c_void,
    size: usize,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

unsafe fn enable_acrylic(hwnd: HWND) {
    let Ok(user32) = GetModuleHandleW(w!("user32.dll")) else { return };
    let Some(proc) = GetProcAddress(user32, s!("SetWindowCompositionAttribute")) else { return };
    let set: unsafe extern "system" fn(HWND, *mut CompositionAttributeData) -> i32 = std::mem::transmute(proc);
    // A cor é ABGR: `--glass-tint` (28 28 34) com alfa ~0,4. Sem tinta o Acrylic do Windows 11 fica
    // quase branco, e a tinta do CSS por cima não basta para escurecê-lo.
    let mut policy = AccentPolicy { state: ACCENT_ENABLE_ACRYLICBLURBEHIND, flags: 0, gradient_color: 0x6622_1C1C, animation_id: 0 };
    let mut data = CompositionAttributeData {
        attribute: WCA_ACCENT_POLICY,
        data: &mut policy as *mut _ as *mut c_void,
        size: std::mem::size_of::<AccentPolicy>(),
    };
    set(hwnd, &mut data);
}

/// O vidro nunca recebe o mouse: os cliques vão para a janela de cima ou de baixo.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    const HTTRANSPARENT: isize = -1;
    if msg == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Cria a janela do vidro. Tem que rodar na thread principal, dona do loop de mensagens.
fn create_window() -> Option<isize> {
    unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None).ok()?.0);
        static REGISTERED: OnceLock<()> = OnceLock::new();
        REGISTERED.get_or_init(|| {
            let class = WNDCLASSW { lpfnWndProc: Some(wnd_proc), hInstance: instance, lpszClassName: w!("PierBackdrop"), ..Default::default() };
            RegisterClassW(&class);
        });
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            w!("PierBackdrop"),
            w!(""),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        )
        .ok()?;
        let corners = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corners as *const _ as *const c_void, std::mem::size_of_val(&corners) as u32);
        let dark = BOOL::from(true);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const c_void, std::mem::size_of_val(&dark) as u32);
        // A sombra que o DWM desenha em volta da janela ignora a região arredondada e aparece como
        // um retângulo em volta da barra e dos painéis.
        let policy = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_NCRENDERING_POLICY, &policy as *const _ as *const c_void, std::mem::size_of_val(&policy) as u32);
        enable_acrylic(hwnd);
        Some(hwnd.0 as isize)
    }
}

/// Posiciona o vidro sob o retângulo da página, logo abaixo da janela dona, ou o esconde.
fn apply(owner: HWND, b: &Backdrop) {
    let hwnd = HWND(b.hwnd as *mut c_void);
    unsafe {
        let mut win = RECT::default();
        if b.hidden || !super::active() || b.rect.width <= 0.0 || GetWindowRect(owner, &mut win).is_err() {
            let _ = ShowWindow(hwnd, SW_HIDE);
            return;
        }
        let scale = GetDpiForWindow(owner) as f64 / 96.0;
        let x = win.left + (b.rect.x * scale).round() as i32;
        let y = win.top + (b.rect.y * scale).round() as i32;
        let (w, h) = ((b.rect.width * scale).round() as i32, (b.rect.height * scale).round() as i32);
        let d = (b.radius * 2.0 * scale).round() as i32;
        // "Depois" da janela dona na ordem Z é logo abaixo dela.
        let _ = SetWindowPos(hwnd, owner, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        SetWindowRgn(hwnd, CreateRoundRectRgn(0, 0, w + 1, h + 1, d, d), true);
    }
}

/// O `HWND` do Tauri vem de outra versão do crate `windows`.
fn owner_of(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|h| HWND(h.0))
}

/// Chave do vidro principal de uma janela (`label`) ou de um vidro com nome (`label#nome`).
fn key(label: &str, name: Option<&str>) -> String {
    name.map_or_else(|| label.to_string(), |name| format!("{label}#{name}"))
}

fn owner(key: &str) -> &str {
    key.split('#').next().unwrap_or(key)
}

/// Aplica `change` ao vidro `only` da janela (criando-o na primeira vez, se `create`) ou, sem
/// `only`, a todos os vidros dela.
fn update(window: &WebviewWindow, only: Option<String>, create: bool, change: impl Fn(&mut Backdrop) + Send + 'static) {
    let label = window.label().to_string();
    let w = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Some(owner_hwnd) = owner_of(&w) else { return };
        with_backdrops(|backdrops| {
            if let Some(key) = only.as_ref().filter(|key| create && !backdrops.contains_key(*key)) {
                let Some(hwnd) = create_window() else { return };
                backdrops.insert(key.clone(), Backdrop { hwnd, rect: Rect::default(), radius: 0.0, hidden: false });
            }
            for (_, b) in backdrops.iter_mut().filter(|(k, _)| only.as_ref().map_or(owner(k) == label, |key| *k == key)) {
                change(b);
                apply(owner_hwnd, b);
            }
        });
    });
}

pub fn set_frame(window: &WebviewWindow, rect: Rect, radius: f64) {
    set_frame_key(window, key(window.label(), None), rect, radius);
}

pub fn set_named_frame(window: &WebviewWindow, name: &str, rect: Rect, radius: f64) {
    set_frame_key(window, key(window.label(), Some(name)), rect, radius);
}

fn set_frame_key(window: &WebviewWindow, key: String, rect: Rect, radius: f64) {
    update(window, Some(key), true, move |b| {
        b.rect = rect;
        b.radius = radius;
    });
}

pub fn remove_named(window: &WebviewWindow, name: &str) {
    let key = key(window.label(), Some(name));
    let _ = window.run_on_main_thread(move || {
        if let Some(b) = with_backdrops(|backdrops| backdrops.remove(&key)) {
            unsafe {
                let _ = DestroyWindow(HWND(b.hwnd as *mut c_void));
            }
        }
    });
}

pub fn set_hidden(window: &WebviewWindow, hidden: bool) {
    let seq = VISIBILITY_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    if hidden {
        update(window, None, false, |b| b.hidden = true);
        return;
    }
    let window = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(REVEAL_DELAY);
        if VISIBILITY_SEQ.load(Ordering::SeqCst) == seq {
            update(&window, None, false, |b| b.hidden = false);
        }
    });
}

/// Reaplica o material e a posição: também devolve o vidro para baixo da janela dona se outra
/// janela "sempre visível" entrou entre as duas.
pub fn refresh(window: &WebviewWindow) {
    update(window, None, false, |_| {});
}

/// Destrói os vidros de uma janela destruída. Chamado na thread principal (eventos de janela).
pub fn forget(label: &str) {
    let removed: Vec<Backdrop> = with_backdrops(|backdrops| {
        let keys: Vec<String> = backdrops.keys().filter(|k| owner(k) == label).cloned().collect();
        keys.iter().filter_map(|k| backdrops.remove(k)).collect()
    });
    for b in removed {
        unsafe {
            let _ = DestroyWindow(HWND(b.hwnd as *mut c_void));
        }
    }
}
