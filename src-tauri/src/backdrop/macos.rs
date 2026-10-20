//! Vidro nativo atrás do WebView: o Liquid Glass (`NSGlassEffectView`, macOS 26) ou, antes dele,
//! o material translúcido do sistema (`NSVisualEffectView`). O WebView é transparente, então o
//! vidro aparece onde a página só tinge.

use std::cell::RefCell;
use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSGlassEffectView, NSGlassEffectViewStyle, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use tauri::WebviewWindow;

use super::Rect;

struct Backdrop {
    view: Retained<NSView>,
    rect: Rect,
    radius: f64,
    hidden: bool,
}

thread_local! {
    // As views são `MainThreadOnly`; todo acesso vem de `with_webview`, que roda na thread principal.
    static BACKDROPS: RefCell<HashMap<String, Backdrop>> = RefCell::new(HashMap::new());
}

fn make_view(mtm: MainThreadMarker) -> Retained<NSView> {
    // A classe só existe a partir do macOS 26; chamar `NSGlassEffectView::new` antes disso aborta.
    if AnyClass::get(c"NSGlassEffectView").is_some() {
        let glass = NSGlassEffectView::new(mtm);
        glass.setStyle(NSGlassEffectViewStyle::Regular);
        return Retained::into_super(glass);
    }
    let effect = NSVisualEffectView::new(mtm);
    effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
    effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    effect.setState(NSVisualEffectState::Active);
    effect.setWantsLayer(true);
    Retained::into_super(effect)
}

fn set_radius(view: &NSView, radius: f64) {
    if let Some(glass) = view.downcast_ref::<NSGlassEffectView>() {
        glass.setCornerRadius(radius);
    } else if let Some(layer) = view.layer() {
        layer.setCornerRadius(radius);
        layer.setMasksToBounds(true);
    }
}

/// Converte para as coordenadas da superview, que no AppKit começam embaixo (a menos que ela seja
/// "flipped"). Escondido, o vidro desce para fora da tela junto com a barra (`.dock-hidden`).
fn frame(parent: &NSView, webview: &NSView, b: &Backdrop) -> NSRect {
    let web = webview.frame();
    let shift = if b.hidden { b.rect.height + 24.0 } else { 0.0 };
    let y = if parent.isFlipped() {
        web.origin.y + b.rect.y + shift
    } else {
        web.origin.y + web.size.height - b.rect.y - b.rect.height - shift
    };
    NSRect::new(NSPoint::new(web.origin.x + b.rect.x, y), NSSize::new(b.rect.width, b.rect.height))
}

pub fn supported() -> bool {
    true
}

/// Chave do vidro principal de uma janela (`label`) ou de um vidro com nome (`label#nome`).
fn key(label: &str, name: Option<&str>) -> String {
    name.map_or_else(|| label.to_string(), |name| format!("{label}#{name}"))
}

fn owner(key: &str) -> &str {
    key.split('#').next().unwrap_or(key)
}

/// Aplica `change` ao vidro `only` da janela (criando-o atrás do WebView na primeira vez, se
/// `create`) ou, sem `only`, a todos os vidros dela.
fn update(window: &WebviewWindow, only: Option<String>, create: bool, change: impl Fn(&mut Backdrop) -> bool + Send + 'static) {
    let label = window.label().to_string();
    let _ = window.with_webview(move |platform| {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let webview = unsafe { &*(platform.inner() as *const NSView) };
        let Some(parent) = (unsafe { webview.superview() }) else { return };
        BACKDROPS.with_borrow_mut(|backdrops| {
            if let Some(key) = only.as_ref().filter(|key| create && !backdrops.contains_key(*key)) {
                let view = make_view(mtm);
                parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, Some(webview));
                backdrops.insert(key.clone(), Backdrop { view, rect: Rect::default(), radius: 0.0, hidden: super::starts_hidden(key) });
            }
            let targets = backdrops.iter_mut().filter(|(k, _)| only.as_ref().map_or(owner(k) == label, |key| *k == key));
            for (_, b) in targets {
                let animate = change(b);
                set_radius(&b.view, b.radius);
                let target = frame(&parent, webview, b);
                let alpha = if b.hidden || !super::active() { 0.0 } else { 1.0 };
                if animate {
                    // O `animator` usa a duração padrão do AppKit (0,25 s), a mesma da transição do CSS.
                    let animator = b.view.animator();
                    animator.setFrame(target);
                    animator.setAlphaValue(alpha);
                } else {
                    b.view.setFrame(target);
                    b.view.setAlphaValue(alpha);
                }
            }
        });
    });
}

pub fn set_frame(window: &WebviewWindow, rect: Rect, radius: f64) {
    set_named_frame_key(window, key(window.label(), None), rect, radius);
}

pub fn set_named_frame(window: &WebviewWindow, name: &str, rect: Rect, radius: f64) {
    set_named_frame_key(window, key(window.label(), Some(name)), rect, radius);
}

fn set_named_frame_key(window: &WebviewWindow, key: String, rect: Rect, radius: f64) {
    update(window, Some(key), true, move |b| {
        b.rect = rect;
        b.radius = radius;
        false
    });
}

pub fn remove_named(window: &WebviewWindow, name: &str) {
    let key = key(window.label(), Some(name));
    let _ = window.with_webview(move |_| {
        if let Some(b) = BACKDROPS.with_borrow_mut(|backdrops| backdrops.remove(&key)) {
            b.view.removeFromSuperview();
        }
    });
}

pub fn set_hidden(window: &WebviewWindow, hidden: bool) {
    update(window, None, false, move |b| {
        b.hidden = hidden;
        true
    });
}

/// Reaplica o material escolhido (vidro ou sólido) aos vidros que a janela já tem.
pub fn refresh(window: &WebviewWindow) {
    update(window, None, false, |_| true);
}

/// Esquece os vidros de uma janela destruída. Chamado na thread principal (eventos de janela).
pub fn forget(label: &str) {
    BACKDROPS.with_borrow_mut(|backdrops| backdrops.retain(|key, _| owner(key) != label));
}
