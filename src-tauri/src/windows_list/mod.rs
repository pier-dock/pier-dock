//! Janelas abertas (o que a taskbar mostraria), eventos ao vivo e ações de focar/minimizar.
//!
//! Cada sistema tem um backend com as mesmas funções: `start` (liga a fonte de eventos, que chama
//! [`mark_dirty`]), `snapshot`, `foreground`, `is_minimized`, `activate`, `minimize` e `close`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

#[cfg(not(windows))]
pub use backend::app_has_focus;
#[cfg(windows)]
pub(crate) use backend::process_path;

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// Identificador opaco da janela: HWND, id do Mutter/X11 ou CGWindowID.
    pub id: i64,
    pub title: String,
    pub exe: String,
    pub aumid: Option<String>,
    pub minimized: bool,
}

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowsSnapshot {
    /// Em ordem Z: a primeira janela de cada app é a usada mais recentemente.
    pub windows: Vec<WindowInfo>,
    pub foreground: i64,
}

static DIRTY: AtomicBool = AtomicBool::new(true);

/// Algo mudou: a próxima volta da thread de eventos tira um snapshot novo.
pub fn mark_dirty() {
    DIRTY.store(true, Ordering::SeqCst);
}

/// Liga os eventos do sistema + uma thread que, quando algo mudou, tira um snapshot e emite
/// `windows-changed` só se ele for diferente do anterior.
pub fn start(app: AppHandle) {
    backend::start(app.clone());
    std::thread::spawn(move || {
        let mut last: Option<WindowsSnapshot> = None;
        loop {
            std::thread::sleep(Duration::from_millis(120));
            if !DIRTY.swap(false, Ordering::SeqCst) {
                continue;
            }
            let snap = backend::snapshot();
            if last.as_ref() != Some(&snap) {
                let _ = app.emit("windows-changed", &snap);
                last = Some(snap);
            }
        }
    });
}

#[tauri::command]
pub fn get_windows() -> WindowsSnapshot {
    backend::snapshot()
}

/// Clique num app aberto: foca a janela mais recente; se o app já está ativo, minimiza (uma
/// janela) ou alterna para a próxima (várias janelas).
#[tauri::command]
pub fn toggle_app_windows(ids: Vec<i64>) {
    let Some(&first) = ids.first() else { return };
    let fg = backend::foreground();
    let active = ids.contains(&fg) && !backend::is_minimized(fg);
    if !active {
        backend::activate(first);
    } else if ids.len() == 1 {
        backend::minimize(fg);
    } else {
        // A lista vem em ordem Z, então a última é a usada há mais tempo: isso faz o ciclo
        // passar por todas as janelas em vez de pular entre as duas mais recentes.
        backend::activate(*ids.last().unwrap());
    }
    mark_dirty();
}

#[tauri::command]
pub fn activate_window(id: i64) {
    backend::activate(id);
    mark_dirty();
}

#[tauri::command]
pub fn close_windows(ids: Vec<i64>) {
    ids.into_iter().for_each(backend::close);
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "smoke test: depende das janelas abertas na máquina"]
    fn lists_open_windows() {
        let snap = super::backend::snapshot();
        for w in &snap.windows {
            println!("{:?} | {} | {:?} | {}", w.id, w.exe, w.aumid, w.title);
        }
        assert!(snap.windows.iter().all(|w| !w.title.is_empty()));
    }
}
