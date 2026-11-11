//! Hover nas janelas do Pier. O WKWebView só recebe o movimento do mouse quando a janela é a
//! janela-chave, e a dock e os painéis nunca são: o app é `Accessory` e não toma o foco de quem
//! está em uso. Então uma thread lê o cursor e manda a posição para a página (`pointer-bridge`),
//! que gera os eventos de mouse e o estado de hover por conta própria (`src/lib/pointerBridge.ts`).

use std::ffi::c_void;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: *const c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *const c_void) -> CGPoint;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// O cursor em pontos, a partir do canto de cima à esquerda da tela principal. Com telas de escalas
/// diferentes (um Retina e um monitor externo) os "pixels físicos" de cada uma não se comparam; os
/// pontos, sim.
///
/// Lê direto do Quartz, que vale de qualquer thread. O `cursor_position` do tao faz uma ida à thread
/// principal por leitura, e as threads de hover leem a cada 16 ms.
pub fn cursor_points() -> Option<(f64, f64)> {
    // SAFETY: `CGEventCreate` aceita uma fonte nula e devolve um evento que este código possui (e
    // solta aqui) ou nulo; `CGEventGetLocation` só lê dele.
    unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return None;
        }
        let location = CGEventGetLocation(event);
        CFRelease(event);
        Some((location.x, location.y))
    }
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
        let mut frame: Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> = None;
        let mut frame_read = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(16));
            let Some(window) = app.get_webview_window(&label) else { return };
            // Posição, tamanho e telas só mudam com o layout; cada leitura é uma ida à thread principal.
            if frame.is_none() || frame_read.elapsed() > Duration::from_millis(500) {
                frame = match (window.outer_position(), window.outer_size(), window.scale_factor()) {
                    (Ok(pos), Ok(size), Ok(scale)) => Some((pos, size, scale)),
                    _ => None,
                };
                frame_read = Instant::now();
            }
            let Some((pos, size, scale)) = frame else { continue };
            let Some((cx, cy)) = cursor_points() else { continue };
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
