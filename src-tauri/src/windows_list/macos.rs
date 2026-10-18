//! Apps e janelas no macOS, no modelo do Dock nativo: todo app regular aberto aparece, com ou sem
//! janelas. Com a permissão de Acessibilidade cada janela vira uma entrada (id = CGWindowID). Um
//! app sem janelas, ou qualquer app sem a permissão, vira uma entrada só com id = -pid, e as ações
//! nela valem para o app inteiro (mostrar, esconder com ⌘H, encerrar).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use accessibility_sys::{
    kAXCloseButtonAttribute, kAXFocusedWindowAttribute, kAXMainAttribute, kAXMinimizedAttribute, kAXPressAction,
    kAXRaiseAction, kAXStandardWindowSubrole, kAXSubroleAttribute, kAXTitleAttribute, kAXWindowsAttribute,
};
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use tauri::AppHandle;

use super::{mark_dirty, WindowInfo, WindowsSnapshot};
use crate::platform::macos::ax::{self, Element};
use crate::platform::macos::workspace;

const FINDER: &str = "com.apple.finder";

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// SPI do Dock que dispara as ações do Mission Control (não há API pública para elas); o
    /// mesmo caminho que o app do Mission Control usa.
    fn CoreDockSendNotification(notification: CFStringRef, unknown: i32) -> i32;
}

/// Janela (ou app, se negativo) em foco no último app da frente que não era a própria dock.
static LAST_FOREGROUND: AtomicI64 = AtomicI64::new(0);
/// Um app (não a área de trabalho nem a própria dock) está em foco.
static APP_FOCUSED: AtomicBool = AtomicBool::new(false);
/// Janelas do último snapshot, para as ações não precisarem procurar de novo.
static WINDOWS: Mutex<Option<HashMap<i64, Element>>> = Mutex::new(None);

/// O pid da frente é barato de ler e muda a cada troca de app; o resto (janelas abertas,
/// fechadas, títulos) é pego numa varredura completa a cada segundo.
pub fn start(_app: AppHandle) {
    std::thread::spawn(|| {
        let mut last_front = 0;
        let mut ticks = 0;
        loop {
            std::thread::sleep(Duration::from_millis(150));
            let front = workspace::frontmost().map(|a| a.pid).unwrap_or(0);
            ticks += 1;
            if front != last_front || ticks >= 7 {
                last_front = front;
                ticks = 0;
                mark_dirty();
            }
        }
    });
}

fn placeholder_id(pid: i32) -> i64 {
    -(pid as i64)
}

fn cached(id: i64) -> Option<Element> {
    WINDOWS.lock().unwrap().as_ref()?.get(&id).cloned()
}

/// Uma janela minimizada passa a ter subrole `AXDialog` (macOS 26) e sumiria da lista; sem ela, o
/// clique na dock só ativava o app e a janela continuava no Dock nativo.
fn standard_windows(app: &Element) -> impl Iterator<Item = (u32, Element)> {
    app.elements(kAXWindowsAttribute)
        .into_iter()
        .filter(|w| w.string(kAXSubroleAttribute).as_deref() == Some(kAXStandardWindowSubrole) || w.bool(kAXMinimizedAttribute))
        .filter_map(|w| Some((w.window_id()?, w)))
}

fn note_focus(trusted: bool) {
    let own = std::process::id() as i32;
    let Some(front) = workspace::frontmost() else {
        APP_FOCUSED.store(false, Ordering::SeqCst);
        return;
    };
    if front.pid == own {
        APP_FOCUSED.store(false, Ordering::SeqCst);
        return;
    }
    let focused = trusted
        .then(|| Element::application(front.pid)?.element(kAXFocusedWindowAttribute)?.window_id())
        .flatten();
    LAST_FOREGROUND.store(focused.map(i64::from).unwrap_or(placeholder_id(front.pid)), Ordering::SeqCst);
    // O Finder em foco sem nenhuma janela é a área de trabalho.
    let desktop = front.bundle_id.as_deref() == Some(FINDER) && focused.is_none();
    APP_FOCUSED.store(!desktop, Ordering::SeqCst);
}

pub fn snapshot() -> WindowsSnapshot {
    let trusted = ax::is_trusted();
    let mut windows = Vec::new();
    let mut elements = HashMap::new();
    for app in workspace::running_apps() {
        let exe = app.path.clone().unwrap_or_default();
        let before = windows.len();
        if let Some(element) = trusted.then(|| Element::application(app.pid)).flatten() {
            for (id, window) in standard_windows(&element) {
                let title = window.string(kAXTitleAttribute).filter(|t| !t.is_empty()).unwrap_or_else(|| app.name.clone());
                windows.push(WindowInfo {
                    id: id as i64,
                    title,
                    exe: exe.clone(),
                    aumid: app.bundle_id.clone(),
                    minimized: app.hidden || window.bool(kAXMinimizedAttribute),
                });
                elements.insert(id as i64, window);
            }
        }
        if windows.len() == before {
            windows.push(WindowInfo {
                id: placeholder_id(app.pid),
                title: app.name.clone(),
                exe,
                aumid: app.bundle_id.clone(),
                minimized: app.hidden,
            });
        }
    }
    *WINDOWS.lock().unwrap() = Some(elements);
    note_focus(trusted);
    WindowsSnapshot { windows, foreground: LAST_FOREGROUND.load(Ordering::SeqCst) }
}

pub fn app_has_focus() -> bool {
    APP_FOCUSED.load(Ordering::SeqCst)
}

pub fn foreground() -> i64 {
    LAST_FOREGROUND.load(Ordering::SeqCst)
}

pub fn is_minimized(id: i64) -> bool {
    if id < 0 {
        return workspace::is_hidden(-id as i32);
    }
    cached(id).is_some_and(|w| w.bool(kAXMinimizedAttribute) || w.pid().is_some_and(workspace::is_hidden))
}

pub fn activate(id: i64) {
    if id < 0 {
        workspace::activate(-id as i32);
    } else if let Some(window) = cached(id) {
        window.set_bool(kAXMinimizedAttribute, false);
        if let Some(pid) = window.pid() {
            workspace::activate(pid);
        }
        window.perform(kAXRaiseAction);
        window.set_bool(kAXMainAttribute, true);
    }
    LAST_FOREGROUND.store(id, Ordering::SeqCst);
}

pub fn minimize(id: i64) {
    if id < 0 {
        workspace::hide(-id as i32);
    } else if let Some(window) = cached(id) {
        window.set_bool(kAXMinimizedAttribute, true);
    }
}

pub fn close(id: i64) {
    if id < 0 {
        workspace::terminate(-id as i32);
    } else if let Some(button) = cached(id).and_then(|w| w.element(kAXCloseButtonAttribute)) {
        button.perform(kAXPressAction);
    }
}

/// A ação "Mostrar Área de Trabalho" do Mission Control: minimiza todas as janelas e, chamada de
/// novo, o Dock as restaura.
pub fn toggle_desktop() -> Result<(), String> {
    let notification = CFString::from_static_string("com.apple.showdesktop.awake");
    let error = unsafe { CoreDockSendNotification(notification.as_concrete_TypeRef(), 0) };
    if error == 0 {
        Ok(())
    } else {
        Err(format!("CoreDockSendNotification falhou ({error})"))
    }
}

/// Sem captura no macOS por enquanto (o frontend cai no ícone do app). O caminho é o
/// `SCScreenshotManager`/`CGWindowListCreateImage` do ScreenCaptureKit, que pede a permissão de
/// gravação de tela.
pub fn thumbnail(_id: i64) -> Option<Vec<u8>> {
    None
}
