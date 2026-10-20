//! Janelas abertas (o que a taskbar mostraria), eventos ao vivo e ações de focar/minimizar.
//!
//! Cada sistema tem um backend com as mesmas funções: `start` (liga a fonte de eventos, que chama
//! [`mark_dirty`]), `snapshot`, `foreground`, `is_minimized`, `activate`, `minimize`, `close`,
//! `toggle_desktop` e `thumbnail` (a prévia de uma janela, quando o sistema permite capturá-la).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::util::ErrStr;

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
    /// Pid do processo dono da janela — usado para casar uma sessão do Claude Code com a janela de
    /// terminal/IDE que a lançou (ver `claude_agents`).
    pub pid: u32,
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

/// Mostra a área de trabalho minimizando todas as janelas; chamado de novo, restaura — o mesmo
/// "mostrar área de trabalho" do botão do canto da barra de tarefas.
#[tauri::command]
pub fn toggle_desktop() -> Result<(), String> {
    backend::toggle_desktop()?;
    mark_dirty();
    Ok(())
}

/// Miniatura de uma janela, para as prévias ao passar o mouse: PNG pronto num data URL, ou `None`
/// quando o sistema não consegue capturar (janela minimizada, ou Windows/macOS, que ainda não têm
/// captura). O frontend cai no ícone do app quando não vem nada.
#[tauri::command]
pub async fn window_thumbnail(id: i64) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || backend::thumbnail(id).map(|png| png_data_url(&png)))
        .await
        .err_str()
}

/// A miniatura vai direto num `<img src>` do WebView.
fn png_data_url(png: &[u8]) -> String {
    use base64::Engine as _;
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// Reduz ao tamanho de exibição (2x para telas densas) e codifica em PNG.
pub(crate) fn png_thumbnail(img: image::RgbaImage) -> Option<Vec<u8>> {
    const MAX_WIDTH: u32 = 336;
    let img = if img.width() > MAX_WIDTH {
        let height = (f64::from(img.height()) * f64::from(MAX_WIDTH) / f64::from(img.width())).round().max(1.0) as u32;
        image::imageops::resize(&img, MAX_WIDTH, height, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
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

    #[test]
    #[ignore = "smoke test: depende das janelas abertas na máquina"]
    fn captures_window_thumbnails() {
        let windows = super::backend::snapshot().windows;
        let mut captured = 0;
        for w in &windows {
            match super::backend::thumbnail(w.id) {
                Some(png) => {
                    captured += 1;
                    println!("{:>7} bytes | {:?} | {}", png.len(), w.id, w.title);
                }
                None => println!("    sem prévia | {:?} | {}", w.id, w.title),
            }
        }
        // Uma sessão sem janelas de apps não tem o que capturar.
        assert!(windows.is_empty() || captured > 0);
    }
}
