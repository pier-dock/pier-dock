//! Janelas extras da dock: uma por monitor adicional selecionado pelo usuário.
//! Cada janela extra carrega o mesmo frontend da dock principal, é posicionada no monitor
//! escolhido e registra sua própria AppBar. Visibilidade e layout são herdados da janela principal.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(windows)]
use std::sync::Arc;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(windows)]
use crate::dock_window::Band;

struct ExtraEntry {
    label: String,
    monitor_id: String,
    #[cfg(windows)]
    appbar_registered: AtomicBool,
    #[cfg(windows)]
    appbar_monitor: Mutex<Option<Band>>,
}

static ENTRIES: Mutex<Vec<ExtraEntry>> = Mutex::new(Vec::new());
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Sincroniza as janelas extras com base na configuração de monitor da dock.
/// "all" → cria janelas em todos os monitores não-primários.
/// Qualquer outro valor → fecha todas as janelas extras.
pub fn sync_from_setting(app: &AppHandle, setting: &str) {
    if setting == crate::dock_window::ALL {
        let ids: Vec<String> = crate::dock_window::list_monitors(app.clone())
            .into_iter()
            .filter(|m| !m.primary)
            .map(|m| m.id)
            .collect();
        sync_monitors(app, &ids);
    } else {
        sync_monitors(app, &[]);
    }
}

/// Sincroniza as janelas extras com a lista de ids de monitores. Cria janelas para
/// monitores novos, fecha as de monitores removidos e re-aplica o layout nos que ficaram.
fn sync_monitors(app: &AppHandle, monitor_ids: &[String]) {
    // Fase 1: fechar janelas de monitores que saíram da lista (lock breve).
    {
        let mut entries = ENTRIES.lock().unwrap();
        entries.retain(|e| {
            if monitor_ids.contains(&e.monitor_id) {
                true
            } else {
                if let Some(w) = app.get_webview_window(&e.label) {
                    let _ = w.destroy();
                }
                false
            }
        });
    }

    // Fase 2: descobrir quais monitores precisam de janela nova (lock breve).
    let to_create: Vec<String> = {
        let entries = ENTRIES.lock().unwrap();
        monitor_ids
            .iter()
            .filter(|id| !entries.iter().any(|e| &e.monitor_id == *id))
            .cloned()
            .collect()
    };

    // Fase 3: criar janelas SEM segurar o lock — build() despacha para o event loop
    // e qualquer evento disparado durante a criação pode precisar de ENTRIES.
    let mut new_entries = Vec::new();
    for id in &to_create {
        if let Some(entry) = create_extra(app, id) {
            new_entries.push(entry);
        }
    }

    // Fase 4: adicionar entradas e re-aplicar layout em todas (lock breve).
    {
        let mut entries = ENTRIES.lock().unwrap();
        entries.extend(new_entries);
        for entry in entries.iter() {
            apply_entry_layout(app, entry);
        }
    }
}

/// Cria uma janela extra para o monitor com o id dado e a inicializa.
fn create_extra(app: &AppHandle, monitor_id: &str) -> Option<ExtraEntry> {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let label = format!("dock-extra-{n}");

    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("Pier")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .build()
        .ok()?;

    #[cfg(windows)]
    let stop = Arc::new(AtomicBool::new(false));

    // Ao destruir a janela extra (troca de config, monitor desconectado): encerra a thread de
    // ponteiro e esquece o vidro e a geometria da barra desta janela.
    {
        let cleanup_label = label.clone();
        #[cfg(windows)]
        let cleanup_stop = stop.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                #[cfg(windows)]
                cleanup_stop.store(true, Ordering::SeqCst);
                crate::backdrop::forget(&cleanup_label);
                crate::dock_window::forget_hit_rect(&cleanup_label);
            }
        });
    }

    let entry = ExtraEntry {
        label: label.clone(),
        monitor_id: monitor_id.to_string(),
        #[cfg(windows)]
        appbar_registered: AtomicBool::new(false),
        #[cfg(windows)]
        appbar_monitor: Mutex::new(None),
    };

    #[cfg(windows)]
    crate::dock_window::init_extra(
        &window,
        monitor_id,
        &entry.appbar_registered,
        &entry.appbar_monitor,
        stop,
    );

    // Em outros sistemas operacionais: apenas posicionar.
    #[cfg(not(windows))]
    {
        let _ = crate::dock_window::place_window_on(&window, monitor_id);
        let _ = window.show();
    }

    Some(entry)
}

/// Notifica todas as janelas extras sobre uma mudança de visibilidade da dock principal.
/// Chamado por `dock_window::set_visible`.
pub fn on_visibility_changed(app: &AppHandle, visible: bool) {
    let entries = ENTRIES.lock().unwrap();
    for entry in entries.iter() {
        if let Some(window) = app.get_webview_window(&entry.label) {
            let _ = window.emit_to(&entry.label, "dock-visibility", visible);
            crate::backdrop::set_hidden(&window, !visible);
        }
    }
}

/// Re-aplica o layout em todas as janelas extras. Chamado quando posição, auto-hide ou tamanho
/// do layout mudam.
pub fn apply_all_layouts(app: &AppHandle) {
    let entries = ENTRIES.lock().unwrap();
    for entry in entries.iter() {
        apply_entry_layout(app, entry);
    }
}

fn apply_entry_layout(app: &AppHandle, entry: &ExtraEntry) {
    let Some(window) = app.get_webview_window(&entry.label) else { return };
    #[cfg(windows)]
    crate::dock_window::apply_layout_extra(
        &window,
        &entry.monitor_id,
        &entry.appbar_registered,
        &entry.appbar_monitor,
    );
    #[cfg(not(windows))]
    {
        let _ = crate::dock_window::place_window_on(&window, &entry.monitor_id);
    }
}

/// Chamado quando o explorer.exe reinicia: todos os registros de AppBar se perderam.
/// Re-registra cada janela extra.
#[cfg(windows)]
pub fn reregister_all(app: &AppHandle) {
    let entries = ENTRIES.lock().unwrap();
    for entry in entries.iter() {
        entry.appbar_registered.store(false, Ordering::SeqCst);
    }
    drop(entries);
    apply_all_layouts(app);
}

// Stub para sistemas não-Windows.
#[cfg(not(windows))]
pub fn reregister_all(_app: &AppHandle) {}
