//! Janelas auxiliares: painéis dos widgets (ancorados acima da dock) e a janela de configurações.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

use crate::dock_window::{position, reserved_rect, DOCK_LABEL, LEFT, RIGHT, TOP};
use crate::util::ErrStr;

pub const SETTINGS_LABEL: &str = "settings";
pub const WHATS_NEW_LABEL: &str = "whats-new";

/// Margem transparente da janela em volta do painel e raio dos cantos, iguais a `--panel-inset` e
/// a `--radius-window` no CSS.
const PANEL_INSET: f64 = 18.0;
const PANEL_RADIUS: f64 = 18.0;

/// (label, kind) do painel aberto. Cada painel ganha um label novo porque fechar uma janela é
/// assíncrono e reutilizar o label logo em seguida falharia.
static OPEN_PANEL: Mutex<Option<(String, String)>> = Mutex::new(None);
static PANEL_COUNTER: AtomicU32 = AtomicU32::new(0);

/// O frontend decide o que renderizar por esta variável global, injetada antes da página carregar.
fn view_script(view: &str, kind: &str) -> String {
    format!(
        "window.__PIER_VIEW__ = {};",
        serde_json::json!({ "view": view, "kind": kind })
    )
}

pub fn panel_open() -> bool {
    OPEN_PANEL.lock().unwrap().is_some()
}

pub fn close_open_panel(app: &AppHandle) -> Option<String> {
    let (label, kind) = OPEN_PANEL.lock().unwrap().take()?;
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.destroy();
    }
    Some(kind)
}

/// Abre o painel de um widget ao lado dele; clicar de novo no mesmo widget fecha. `anchor_x` e
/// `anchor_y` são o centro do widget em pixels lógicos da janela da dock.
#[tauri::command]
pub async fn open_panel(app: AppHandle, kind: String, anchor_x: f64, anchor_y: f64, width: f64, height: f64) -> Result<(), String> {
    if close_open_panel(&app).as_deref() == Some(kind.as_str()) {
        return Ok(());
    }
    let dock = app.get_webview_window(DOCK_LABEL).ok_or("dock não encontrada")?;
    let reserved = reserved_rect(&dock).ok_or("monitor não encontrado")?;
    let dock_pos = dock.outer_position().err_str()?;

    let scale = reserved.scale;
    let monitor = reserved.monitor;
    let margin = 10.0 * scale;
    let (w, h) = (width * scale, height * scale);
    // O painel abre para dentro da tela, encostado na faixa reservada e centrado no widget.
    let anchor = (
        dock_pos.x as f64 + anchor_x * scale,
        dock_pos.y as f64 + anchor_y * scale,
    );
    let centered_x = clamp(anchor.0 - w / 2.0, monitor.left as f64 + margin, monitor.right as f64 - w - margin);
    let centered_y = clamp(anchor.1 - h / 2.0, monitor.top as f64 + margin, monitor.bottom as f64 - h - margin);
    let (x, y) = match position() {
        TOP => (centered_x, reserved.band.bottom as f64),
        LEFT => (reserved.band.right as f64, centered_y),
        RIGHT => (reserved.band.left as f64 - w, centered_y),
        // Acima da barra: a margem transparente em volta do painel (sombra) já serve de espaço
        // entre ele e a dock.
        _ => (centered_x, reserved.band.top as f64 - h),
    };

    let label = format!("panel-{}", PANEL_COUNTER.fetch_add(1, Ordering::SeqCst));
    let builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
        .initialization_script(&view_script("panel", &kind))
        .title("Pier")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .inner_size(width, height)
        .position(x / scale, y / scale)
        .focused(true);
    // No Linux a janela aparece preta até o WebKitGTK desenhar a primeira vez: só mostra depois
    // que a página carregou.
    #[cfg(target_os = "linux")]
    let builder = builder.visible(false).on_page_load(|window, payload| {
        if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
            let _ = window.show();
            let _ = window.set_focus();
        }
    });
    #[cfg(target_os = "macos")]
    let builder = builder.accept_first_mouse(true);
    let window = builder.build().err_str()?;
    #[cfg(target_os = "macos")]
    crate::platform::macos::pointer::forward(&window);
    let rect = crate::backdrop::Rect { x: PANEL_INSET, y: PANEL_INSET, width: width - 2.0 * PANEL_INSET, height: height - 2.0 * PANEL_INSET };
    crate::backdrop::set_frame(&window, rect, PANEL_RADIUS);

    // Fecha ao perder o foco, mas só depois de ter tido foco: no X11 a janela recém-criada recebe
    // um "sem foco" antes de ser focada, o que a fechava na hora.
    let handle = window.clone();
    let was_focused = AtomicBool::new(false);
    window.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            crate::backdrop::forget(handle.label());
        }
        if let WindowEvent::Focused(focused) = event {
            if *focused {
                was_focused.store(true, Ordering::SeqCst);
                return;
            }
            if !was_focused.load(Ordering::SeqCst) {
                return;
            }
            let mut open = OPEN_PANEL.lock().unwrap();
            if open.as_ref().map(|(l, _)| l == handle.label()).unwrap_or(false) {
                *open = None;
            }
            let _ = handle.destroy();
        }
    });
    *OPEN_PANEL.lock().unwrap() = Some((label, kind));
    Ok(())
}

/// Prende `value` entre `min` e `max`. Uma janela maior que o intervalo fica no começo dele, em
/// vez de estourar o `clamp` do `f64`.
fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max.max(min))
}

/// Fecha o painel aberto se o clique (em pixels físicos de tela) caiu fora dele. No Windows o
/// painel quase nunca ganha foco: o clique na dock vai para o processo do WebView2, então o
/// Windows recusa trazer a janela nova para a frente e o `Focused(false)` nunca chega.
#[cfg(windows)]
pub fn close_if_outside(app: &AppHandle, x: i32, y: i32) {
    let Some(label) = OPEN_PANEL.lock().unwrap().as_ref().map(|(l, _)| l.clone()) else { return };
    let Some(window) = app.get_webview_window(&label) else { return };
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else { return };
    let inside = x >= pos.x && x < pos.x + size.width as i32 && y >= pos.y && y < pos.y + size.height as i32;
    if !inside {
        close_open_panel(app);
    }
}

/// O painel não costuma receber o teclado (no macOS o app nunca fica ativo, no Windows o clique
/// vai para o processo do WebView2). Chamado antes de digitar, como na senha do Wi-Fi: o
/// `set_focus` do tao ativa o app no macOS e força o primeiro plano no Windows.
#[tauri::command]
pub fn focus_panel(window: WebviewWindow) {
    let _ = window.set_focus();
}

#[tauri::command]
pub fn close_panel(app: AppHandle) {
    close_open_panel(&app);
}

pub fn open_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let title = crate::i18n::settings_title(crate::i18n::current(app));
    app_window(app, SETTINGS_LABEL, "", title, (1040.0, 720.0), (880.0, 600.0))
}

/// Novidades das versões depois de `since` (vazio: só a atual), lidas do changelog pelo frontend.
pub fn open_whats_new_window(app: &AppHandle, since: &str) -> tauri::Result<WebviewWindow> {
    let title = crate::i18n::current(app).t("Novidades do Pier", "What's new in Pier");
    let window = app_window(app, WHATS_NEW_LABEL, since, title, (600.0, 680.0), (480.0, 480.0))?;
    // Aberta no boot, sem interação do usuário, o sistema não traz a janela para a frente (o app
    // não está ativo no macOS, o Windows bloqueia o primeiro plano). Fica por cima até a primeira
    // mudança de foco e depois volta a ser uma janela comum.
    let _ = window.set_always_on_top(true);
    let handle = window.clone();
    let released = AtomicBool::new(false);
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(_) = event {
            if !released.swap(true, Ordering::SeqCst) {
                let _ = handle.set_always_on_top(false);
            }
        }
    });
    let _ = window.set_focus();
    Ok(window)
}

/// Janelas normais do app (configurações e novidades), com o material do sistema; o label também é
/// a view que o frontend renderiza. Se já estiver aberta, só traz para a frente.
fn app_window(app: &AppHandle, label: &str, kind: &str, title: &str, size: (f64, f64), min_size: (f64, f64)) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(window);
    }
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .initialization_script(&view_script(label, kind))
        .title(title)
        .inner_size(size.0, size.1)
        .min_inner_size(min_size.0, min_size.1)
        .theme(Some(tauri::Theme::Dark))
        .center()
        .focused(true);
    // O cabeçalho da página faz as vezes da barra de título, com os semáforos por cima.
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    // O material da janela inteira vem do sistema; no Linux não há, e a página pinta o sólido.
    #[cfg(not(target_os = "linux"))]
    let builder = builder.transparent(true).effects(settings_effects());
    builder.build()
}

/// Vibrancy escura no macOS, Mica no Windows 11; sem efeito com o material sólido.
#[cfg(not(target_os = "linux"))]
fn settings_effects() -> tauri::utils::config::WindowEffectsConfig {
    use tauri::window::{Effect, EffectState, EffectsBuilder};
    if !crate::backdrop::active() {
        return EffectsBuilder::new().build();
    }
    let effect = if cfg!(target_os = "macos") { Effect::HudWindow } else { Effect::MicaDark };
    EffectsBuilder::new().effect(effect).state(EffectState::Active).build()
}

/// Troca o material das janelas de configurações e de novidades abertas (a dock e os painéis são
/// do `backdrop`).
pub fn apply_settings_material(app: &AppHandle) {
    #[cfg(not(target_os = "linux"))]
    for window in [SETTINGS_LABEL, WHATS_NEW_LABEL].into_iter().filter_map(|label| app.get_webview_window(label)) {
        let _ = window.set_effects(settings_effects());
    }
    #[cfg(target_os = "linux")]
    let _ = app;
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app).map(|_| ()).err_str()
}

#[tauri::command]
pub async fn open_whats_new(app: AppHandle) -> Result<(), String> {
    open_whats_new_window(&app, "").map(|_| ()).err_str()
}
