//! Hover nas janelas do Pier. O WKWebView só recebe o movimento do mouse quando a janela é a
//! janela-chave, e a dock e os painéis nunca são: o app é `Accessory` e não toma o foco de quem
//! está em uso. Então uma thread lê o cursor e manda a posição para a página (`pointer-bridge`),
//! que gera os eventos de mouse e o estado de hover por conta própria (`src/lib/pointerBridge.ts`).

use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// Escala da tela principal: a que o `cursor_position` do tao usa.
pub fn primary_scale(app: &AppHandle) -> f64 {
    app.primary_monitor().ok().flatten().map_or(1.0, |m| m.scale_factor())
}

/// O cursor em pontos, a partir do canto de cima à esquerda da tela principal. O `cursor_position`
/// do tao multiplica os pontos pela escala da tela principal, mas a posição de uma janela ou de um
/// monitor usa a escala da própria tela: com telas de escalas diferentes (um Retina e um monitor
/// externo) os "pixels físicos" de cada um não se comparam. Os pontos, sim.
pub fn cursor_points(app: &AppHandle, primary_scale: f64) -> Option<(f64, f64)> {
    let cursor = app.cursor_position().ok()?;
    Some((cursor.x / primary_scale, cursor.y / primary_scale))
}

/// Posição em pixels CSS, relativa ao topo esquerdo da janela.
#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
struct Pointer {
    x: f64,
    y: f64,
    inside: bool,
}

/// Roda enquanto a janela existir.
pub fn forward(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    let label = window.label().to_string();
    std::thread::spawn(move || {
        let mut last: Option<Pointer> = None;
        let mut frame: Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64, f64)> = None;
        let mut frame_read = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(16));
            let Some(window) = app.get_webview_window(&label) else { return };
            // Posição, tamanho e telas só mudam com o layout; cada leitura é uma ida à thread principal.
            if frame.is_none() || frame_read.elapsed() > Duration::from_millis(500) {
                frame = match (window.outer_position(), window.outer_size(), window.scale_factor()) {
                    (Ok(pos), Ok(size), Ok(scale)) => Some((pos, size, scale, primary_scale(&app))),
                    _ => None,
                };
                frame_read = Instant::now();
            }
            let Some((pos, size, scale, primary)) = frame else { continue };
            let Some((cx, cy)) = cursor_points(&app, primary) else { continue };
            // Pontos da tela são pixels CSS da página.
            let x = cx - pos.x as f64 / scale;
            let y = cy - pos.y as f64 / scale;
            let inside = x >= 0.0 && y >= 0.0 && x < size.width as f64 / scale && y < size.height as f64 / scale;
            let pointer = Pointer { x, y, inside };
            // Fora da janela, só a primeira posição interessa (para a página encerrar o hover).
            let unchanged = last == Some(pointer) || (!inside && last.is_some_and(|l| !l.inside));
            if unchanged {
                continue;
            }
            last = Some(pointer);
            let _ = app.emit_to(label.as_str(), "pointer-bridge", pointer);
        }
    });
}
