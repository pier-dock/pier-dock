//! Janela da dock: posicionamento na borda escolhida do monitor escolhido (o principal, por
//! padrão), reserva da área de trabalho, click-through fora da barra e ocultação automática.
//!
//! O estado (medidas vindas do frontend, borda da tela, modo de ocultar, visibilidade) é comum;
//! cada sistema tem um backend com `init`, `apply_layout` e `hit_rect_changed`.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Monitor, WebviewWindow};

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

pub use backend::init;
#[cfg(windows)]
pub use backend::{remove_appbar, reregister};

pub const DOCK_LABEL: &str = "dock";

/// Borda da tela onde a dock fica.
pub const BOTTOM: u8 = 0;
pub const TOP: u8 = 1;
pub const LEFT: u8 = 2;
pub const RIGHT: u8 = 3;

static POSITION: AtomicU8 = AtomicU8::new(BOTTOM);

/// A dock está à mostra (ou escondida pelo "ocultar automaticamente"). Começa escondida: ela entra
/// na tela quando a página já desenhou a barra (`start_monitor_watch`).
static VISIBLE: AtomicBool = AtomicBool::new(false);
/// A dock está fora da tela de propósito: abrindo o Pier ou trocando de monitor.
static MOVING: AtomicBool = AtomicBool::new(true);
/// Até quando a dock fica à mostra depois de chegar num monitor, mesmo com o "ocultar
/// automaticamente": sem isso, numa tela com um app em foco ela chegaria escondida e a troca
/// passaria despercebida.
static ARRIVED_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);
const ARRIVAL: Duration = Duration::from_millis(1500);
/// A saída da tela antiga, igual à transição do `.dock-hidden` no CSS.
const LEAVE: Duration = Duration::from_millis(260);
/// Para a janela assentar na tela nova antes de a dock entrar.
const SETTLE: Duration = Duration::from_millis(120);

const AUTO_HIDE_NEVER: u8 = 0;
const AUTO_HIDE_SMART: u8 = 1;
#[allow(dead_code)]
const AUTO_HIDE_ALWAYS: u8 = 2;
static AUTO_HIDE: AtomicU8 = AtomicU8::new(AUTO_HIDE_SMART);

/// Quanto tempo a dock continua à mostra depois que o mouse sai dela.
#[allow(dead_code)]
const HOVER_GRACE_MS: u64 = 500;

pub fn position() -> u8 {
    POSITION.load(Ordering::SeqCst)
}

/// Nome da borda, como o frontend e a extensão do GNOME o conhecem.
pub fn edge_name() -> &'static str {
    match position() {
        TOP => "top",
        LEFT => "left",
        RIGHT => "right",
        _ => "bottom",
    }
}

fn parse_position(position: &str) -> u8 {
    match position {
        "top" => TOP,
        "left" => LEFT,
        "right" => RIGHT,
        _ => BOTTOM,
    }
}

pub fn init_position(position: &str) {
    POSITION.store(parse_position(position), Ordering::SeqCst);
}

/// Troca a borda onde a dock fica. A janela é reposicionada na hora, com as medidas que já estão
/// aqui (o frontend manda as novas logo em seguida).
pub fn set_position(app: &AppHandle, position: &str) {
    let position = parse_position(position);
    if POSITION.swap(position, Ordering::SeqCst) != position {
        if let Some(window) = app.get_webview_window(DOCK_LABEL) {
            backend::apply_layout(&window);
        }
    }
}

/// Monitor onde a dock fica: `primary`, `cursor` ou o id de um monitor (`monitor_ids`).
static MONITOR: Mutex<String> = Mutex::new(String::new());
/// Seguindo o cursor: o monitor para onde a dock foi por último (`None`: o principal).
static CURSOR_MONITOR: Mutex<Option<String>> = Mutex::new(None);

const PRIMARY: &str = "primary";
const CURSOR: &str = "cursor";
/// Seguindo o cursor, a dock só troca de monitor depois que ele fica um tempo no outro: passar de
/// raspão por uma tela no caminho não a arrasta junto.
const FOLLOW_TICK: Duration = Duration::from_millis(200);
const FOLLOW_DWELL: Duration = Duration::from_millis(400);

pub fn init_monitor(monitor: &str) {
    *MONITOR.lock().unwrap() = monitor.to_string();
}

/// Troca o monitor da dock. O painel aberto é fechado: ele ficaria ancorado no monitor antigo.
pub fn set_monitor(app: &AppHandle, monitor: &str) {
    let changed = std::mem::replace(&mut *MONITOR.lock().unwrap(), monitor.to_string()) != monitor;
    if changed {
        move_dock(app);
    }
}

/// Ids estáveis para os monitores, na ordem recebida: o nome do sistema e, entre monitores com o
/// mesmo nome (dois do mesmo modelo), " 2", " 3"… da esquerda para a direita.
fn monitor_ids(monitors: &[(String, i32, i32)]) -> Vec<String> {
    monitors
        .iter()
        .map(|(name, x, y)| {
            let before = monitors.iter().filter(|(n, ox, oy)| n == name && (ox, oy) < (x, y)).count();
            if before == 0 {
                name.clone()
            } else {
                format!("{name} {}", before + 1)
            }
        })
        .collect()
}

fn ids_of(monitors: &[Monitor]) -> Vec<String> {
    let keys: Vec<_> = monitors
        .iter()
        .map(|m| (m.name().cloned().unwrap_or_else(|| "Monitor".into()), m.position().x, m.position().y))
        .collect();
    monitor_ids(&keys)
}

/// O monitor escolhido (ou, seguindo o cursor, o último para onde ela foi); se ele não estiver
/// conectado, o principal.
fn target_monitor(window: &WebviewWindow) -> Option<Monitor> {
    let mut wanted = MONITOR.lock().unwrap().clone();
    if wanted == CURSOR {
        wanted = CURSOR_MONITOR.lock().unwrap().clone().unwrap_or_default();
    }
    if !wanted.is_empty() && wanted != PRIMARY {
        if let Ok(monitors) = window.available_monitors() {
            if let Some(i) = ids_of(&monitors).iter().position(|id| *id == wanted) {
                return monitors.into_iter().nth(i);
            }
        }
    }
    window.primary_monitor().ok().flatten()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub primary: bool,
    /// Tamanho em pixels lógicos.
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    let monitors = app.available_monitors().unwrap_or_default();
    let primary = app.primary_monitor().ok().flatten().map(|m| *m.position());
    ids_of(&monitors)
        .into_iter()
        .zip(&monitors)
        .map(|(id, m)| {
            let size = m.size().to_logical::<u32>(m.scale_factor());
            MonitorInfo {
                id,
                name: m.name().cloned().unwrap_or_else(|| "Monitor".into()),
                primary: Some(*m.position()) == primary,
                width: size.width,
                height: size.height,
            }
        })
        .collect()
}

/// O ponto (no espaço do `backend::cursor`) está no monitor? No macOS cada monitor vem em pixels
/// da própria escala e o cursor em pontos: a comparação é em pontos.
fn contains(monitor: &Monitor, x: f64, y: f64) -> bool {
    let scale = if cfg!(target_os = "macos") { monitor.scale_factor() } else { 1.0 };
    let (pos, size) = (monitor.position(), monitor.size());
    let (left, top) = (pos.x as f64 / scale, pos.y as f64 / scale);
    let (width, height) = (size.width as f64 / scale, size.height as f64 / scale);
    x >= left && x < left + width && y >= top && y < top + height
}

/// Seguindo o cursor: o cursor ficou `FOLLOW_DWELL` em outro monitor, sem painel aberto (ele
/// fecharia no meio do uso)? Esse monitor vira o da dock. `pending` guarda o candidato e desde quando.
fn follow_cursor(app: &AppHandle, window: &WebviewWindow, pending: &mut Option<(String, Instant)>) {
    let (Some((x, y)), Ok(monitors)) = (backend::cursor(app), window.available_monitors()) else { return };
    let Some(index) = monitors.iter().position(|m| contains(m, x, y)) else { return };
    let current = target_monitor(window).map(|m| *m.position());
    if current == Some(*monitors[index].position()) || crate::panels::panel_open() {
        *pending = None;
        return;
    }
    let id = ids_of(&monitors).swap_remove(index);
    match pending {
        Some((candidate, since)) if *candidate == id => {
            if since.elapsed() >= FOLLOW_DWELL {
                *CURSOR_MONITOR.lock().unwrap() = Some(id);
                *pending = None;
            }
        }
        _ => *pending = Some((id, Instant::now())),
    }
}

/// Plugar ou desplugar um monitor, ou mudar a resolução, não move a janela sozinho: a cada segundo
/// confere se o monitor-alvo mudou e reposiciona a dock. Seguindo o cursor, confere mais vezes, e o
/// monitor-alvo muda junto com o cursor.
pub fn start_monitor_watch(app: AppHandle) {
    std::thread::spawn(move || {
        // Abrindo o Pier: a dock entra quando a página já mediu e desenhou a barra.
        let started = Instant::now();
        while hit_rect().width <= 0.0 && started.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(SETTLE);
        if let Some(window) = app.get_webview_window(DOCK_LABEL) {
            arrive(&window);
        }

        let mut pending = None;
        loop {
            let following = *MONITOR.lock().unwrap() == CURSOR;
            std::thread::sleep(if following { FOLLOW_TICK } else { Duration::from_secs(1) });
            // Uma troca em andamento já vai posicionar a dock no monitor-alvo mais recente.
            if MOVING.load(Ordering::SeqCst) {
                continue;
            }
            let Some(window) = app.get_webview_window(DOCK_LABEL) else { continue };
            if following {
                follow_cursor(&app, &window, &mut pending);
            }
            let Some(monitor) = target_monitor(&window) else { continue };
            // Compara com onde a dock foi posicionada de fato: uma troca pedida pelas configurações
            // já se moveu sozinha, e não pode ser animada de novo aqui.
            if *PLACED.lock().unwrap() != Some(monitor_key(&monitor)) {
                move_dock(&app);
            }
        }
    });
}

/// Medidas em pixels lógicos (CSS), informadas pelo frontend. As duas primeiras são medidas no eixo
/// da espessura da barra: a altura, nas bordas de cima e de baixo; a largura, nas laterais.
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    /// Espessura reservada na borda da tela (a área que janelas maximizadas não ocupam).
    pub reserved: f64,
    /// Tamanho total da janela, incluindo a folga para a magnificação, as dicas e as prévias.
    pub window_size: f64,
    /// Quanto a área clicável cresce para dentro da tela enquanto o mouse está na dock.
    pub hover_headroom: f64,
}

#[derive(Clone, Copy, Default, Deserialize)]
pub struct HitRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Raio dos cantos da barra, para o vidro acompanhar.
    #[serde(default)]
    pub radius: f64,
}

/// Retângulo em pixels de tela, ou em pixels da janela quando quem chama trabalha com o layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Band {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Band {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// Faixa reservada e limites do monitor da dock, em pixels físicos de tela — usado para ancorar
/// os painéis dos widgets e a bandeja.
pub struct Reserved {
    pub band: Band,
    pub monitor: Band,
    pub scale: f64,
}

static LAYOUT: Mutex<Layout> = Mutex::new(Layout { reserved: 72.0, window_size: 170.0, hover_headroom: 80.0 });
static HIT: Mutex<HitRect> = Mutex::new(HitRect { x: 0.0, y: 0.0, width: 0.0, height: 0.0, radius: 0.0 });
static PET: Mutex<Option<HitRect>> = Mutex::new(None);

fn layout() -> Layout {
    *LAYOUT.lock().unwrap()
}

fn hit_rect() -> HitRect {
    *HIT.lock().unwrap()
}

fn auto_hide() -> u8 {
    AUTO_HIDE.load(Ordering::SeqCst)
}

/// A dock deve estar à mostra? `hovering` inclui a folga depois que o mouse sai.
#[allow(dead_code)]
fn want_visible(hovering: bool, app_focused: impl FnOnce() -> bool) -> bool {
    if MOVING.load(Ordering::SeqCst) {
        return false;
    }
    if ARRIVED_UNTIL.lock().unwrap().is_some_and(|until| Instant::now() < until) {
        return true;
    }
    match auto_hide() {
        AUTO_HIDE_NEVER => true,
        AUTO_HIDE_SMART => hovering || !app_focused(),
        _ => hovering,
    }
}

/// Atualiza a visibilidade e avisa o frontend, que anima a dock para dentro/fora da tela; o vidro
/// nativo acompanha.
#[allow(dead_code)]
fn set_visible(window: &WebviewWindow, visible: bool) {
    use tauri::Emitter;
    if VISIBLE.swap(visible, Ordering::SeqCst) != visible {
        let _ = window.emit("dock-visibility", visible);
        crate::backdrop::set_hidden(window, !visible);
    }
}

/// Posição, tamanho e escala do monitor onde a dock foi posicionada por último.
type MonitorKey = (tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>, u64);
static PLACED: Mutex<Option<MonitorKey>> = Mutex::new(None);

fn monitor_key(monitor: &Monitor) -> MonitorKey {
    (*monitor.position(), *monitor.size(), monitor.scale_factor().to_bits())
}

/// A dock entra na tela (abrindo o Pier ou num monitor novo) e fica à mostra um instante.
fn arrive(window: &WebviewWindow) {
    *ARRIVED_UNTIL.lock().unwrap() = Some(Instant::now() + ARRIVAL);
    MOVING.store(false, Ordering::SeqCst);
    set_visible(window, true);
}

/// Troca a dock de monitor com animação: ela desliza para fora da tela antiga, a janela muda de
/// lugar e ela desliza para dentro da nova. Com uma troca já em andamento (ou o Pier abrindo), não
/// faz nada: a que está em andamento posiciona a dock no monitor-alvo do momento, e a thread de
/// monitores confere de novo depois.
fn move_dock(app: &AppHandle) {
    let Some(window) = app.get_webview_window(DOCK_LABEL) else { return };
    if MOVING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        crate::panels::close_open_panel(&app);
        if dock_visible() {
            set_visible(&window, false);
            std::thread::sleep(LEAVE);
        }
        backend::apply_layout(&window);
        std::thread::sleep(SETTLE);
        arrive(&window);
    });
}

fn parse_auto_hide(mode: &str) -> u8 {
    match mode {
        "never" => AUTO_HIDE_NEVER,
        "always" => AUTO_HIDE_ALWAYS,
        _ => AUTO_HIDE_SMART,
    }
}

/// Troca o modo de ocultar. Com ocultação ligada a dock não reserva área de trabalho, então
/// janelas maximizadas usam a tela inteira (como a taskbar em "ocultar automaticamente").
pub fn set_auto_hide(app: &AppHandle, mode: &str) {
    let mode = parse_auto_hide(mode);
    if AUTO_HIDE.swap(mode, Ordering::SeqCst) != mode {
        if let Some(window) = app.get_webview_window(DOCK_LABEL) {
            backend::apply_layout(&window);
        }
    }
}

pub fn init_auto_hide(mode: &str) {
    AUTO_HIDE.store(parse_auto_hide(mode), Ordering::SeqCst);
}

#[tauri::command]
pub fn dock_visible() -> bool {
    VISIBLE.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn set_dock_layout(window: WebviewWindow, layout: Layout) {
    *LAYOUT.lock().unwrap() = layout;
    if let Some(dock) = window.app_handle().get_webview_window(DOCK_LABEL) {
        backend::apply_layout(&dock);
    }
}

/// O mouse entrou ou saiu da área clicável da dock. Só o Linux usa: lá o click-through é feito
/// recortando a região de input da janela, e o hover vem do próprio WebView.
#[tauri::command]
pub fn set_dock_hover(hovering: bool) {
    backend::set_hover(hovering);
}

/// A barra mudou de tamanho (itens, magnificação): atualiza a área clicável e o vidro.
#[tauri::command]
pub fn set_hit_rect(app: AppHandle, rect: HitRect) {
    *HIT.lock().unwrap() = rect;
    if let Some(window) = app.get_webview_window(DOCK_LABEL) {
        let glass = crate::backdrop::Rect { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
        crate::backdrop::set_frame(&window, glass, rect.radius);
    }
    backend::hit_rect_changed(&app);
}

/// O Clawd (mascote das sessões do Claude Code) anda logo para fora da barra, então a área dele
/// também recebe clique, mesmo sem o cursor ter passado pela barra. `None` quando ele está
/// escondido ou não tem o que abrir.
#[tauri::command]
pub fn set_pet_rect(app: AppHandle, rect: Option<HitRect>) {
    *PET.lock().unwrap() = rect;
    backend::hit_rect_changed(&app);
}

pub fn pet_rect() -> Option<HitRect> {
    *PET.lock().unwrap()
}

/// O ponto (em pixels lógicos da janela) está sobre o Clawd?
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub fn over_pet(x: f64, y: f64) -> bool {
    pet_rect().is_some_and(|r| x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height)
}

/// A faixa de prévias das janelas abriu, mudou de tamanho (`Some`) ou fechou (`None`): o vidro
/// acompanha, como o da barra.
#[tauri::command]
pub fn set_preview_glass(app: AppHandle, rect: Option<HitRect>) {
    let Some(window) = app.get_webview_window(DOCK_LABEL) else { return };
    match rect {
        Some(r) => crate::backdrop::set_named_frame(&window, "preview", crate::backdrop::Rect { x: r.x, y: r.y, width: r.width, height: r.height }, r.radius),
        None => crate::backdrop::remove_named(&window, "preview"),
    }
}

/// Faixa de `reserved` pixels na borda escolhida do monitor. Na borda de cima a faixa começa
/// depois do que não pertence às janelas (a barra de menus do macOS).
fn reserved_band(monitor: Band, reserved: i32) -> Band {
    let top = monitor.top + backend::top_inset(monitor);
    match position() {
        TOP => Band::new(monitor.left, top, monitor.right, top + reserved),
        LEFT => Band::new(monitor.left, monitor.top, monitor.left + reserved, monitor.bottom),
        RIGHT => Band::new(monitor.right - reserved, monitor.top, monitor.right, monitor.bottom),
        _ => Band::new(monitor.left, monitor.bottom - reserved, monitor.right, monitor.bottom),
    }
}

/// Faixa reservada no monitor da dock, em pixels físicos de tela.
pub fn reserved_rect(window: &WebviewWindow) -> Option<Reserved> {
    let monitor = target_monitor(window)?;
    let scale = monitor.scale_factor();
    let pos = monitor.position();
    let size = monitor.size();
    let bounds = Band::new(pos.x, pos.y, pos.x + size.width as i32, pos.y + size.height as i32);
    let reserved = (layout().reserved * scale).round().max(0.0) as i32;
    Some(Reserved { band: reserved_band(bounds, reserved), monitor: bounds, scale })
}

/// Área clicável da dock, em pixels da janela: a barra mais a folga para dentro da tela, como
/// `(x0, y0, x1, y1)`. Fora dela a janela é click-through.
pub fn clickable(hit: HitRect, headroom: f64, window: (f64, f64)) -> (f64, f64, f64, f64) {
    let (x0, y0, x1, y1) = (hit.x, hit.y, hit.x + hit.width, hit.y + hit.height);
    match position() {
        TOP => (x0, 0.0, x1, y1 + headroom),
        LEFT => (0.0, y0, x1 + headroom, y1),
        RIGHT => (x0 - headroom, y0, window.0, y1),
        _ => (x0, y0 - headroom, x1, window.1),
    }
}

/// Enquanto o frontend não mediu a barra, a área clicável é a faixa reservada inteira.
fn reserved_hit(layout: Layout, window: (f64, f64)) -> HitRect {
    let (width, height) = window;
    match position() {
        TOP => HitRect { x: 0.0, y: 0.0, width, height: layout.reserved, radius: 0.0 },
        LEFT => HitRect { x: 0.0, y: 0.0, width: layout.reserved, height, radius: 0.0 },
        RIGHT => HitRect { x: width - layout.reserved, y: 0.0, width: layout.reserved, height, radius: 0.0 },
        _ => HitRect { x: 0.0, y: height - layout.reserved, width, height: layout.reserved, radius: 0.0 },
    }
}

/// Faixa fina na borda da tela que revela a dock escondida, em pixels da janela.
pub fn reveal_strip(thickness: f64, window: (f64, f64)) -> (f64, f64, f64, f64) {
    match position() {
        TOP => (0.0, 0.0, window.0, thickness),
        LEFT => (0.0, 0.0, thickness, window.1),
        RIGHT => (window.0 - thickness, 0.0, window.0, window.1),
        _ => (0.0, window.1 - thickness, window.0, window.1),
    }
}

/// Área clicável da dock na janela. `headroom` é a folga do hover, que só vale enquanto o cursor
/// está na dock; ela fica presa aos limites da janela.
pub fn clickable_area(window: (f64, f64), headroom: f64) -> (f64, f64, f64, f64) {
    let hit = hit_rect();
    let hit = if hit.width <= 0.0 { reserved_hit(layout(), window) } else { hit };
    let (x0, y0, x1, y1) = clickable(hit, headroom, window);
    (x0.max(0.0), y0.max(0.0), x1.min(window.0), y1.min(window.1))
}

/// O cursor encostou na borda da tela onde a dock vive? Revela a dock escondida.
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub fn at_dock_edge(cx: f64, cy: f64, window: Band, scale: f64) -> bool {
    let reach = 2.0 * scale;
    let (left, top) = (window.left as f64, window.top as f64);
    let (right, bottom) = (window.right as f64, window.bottom as f64);
    match position() {
        TOP => cy <= top + reach && cx >= left && cx < right,
        LEFT => cx <= left + reach && cy >= top && cy < bottom,
        RIGHT => cx >= right - reach && cy >= top && cy < bottom,
        _ => cy >= bottom - reach && cx >= left && cx < right,
    }
}

/// Onde a janela da dock fica, em pixels físicos de tela.
#[derive(Clone, Copy, Debug)]
// O macOS não reserva área de trabalho, então só posiciona a janela.
#[cfg_attr(target_os = "macos", allow(dead_code))]
struct Placement {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    /// Faixa que janelas maximizadas não devem ocupar.
    band: Band,
    /// Limites do monitor onde a janela ficou.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    monitor: Band,
}

/// Posiciona a janela na borda escolhida do monitor da dock, encostada na tela.
fn place_window(window: &WebviewWindow) -> Option<Placement> {
    #[cfg(not(target_os = "macos"))]
    use tauri::{PhysicalPosition, PhysicalSize};
    let layout = layout();
    let monitor = target_monitor(window)?;
    *PLACED.lock().unwrap() = Some(monitor_key(&monitor));
    let scale = monitor.scale_factor();
    let pos = monitor.position();
    let size = monitor.size();
    let thickness = (layout.window_size * scale).round().max(1.0) as i32;
    let reserved = (layout.reserved * scale).round().max(0.0) as i32;
    let bounds = Band::new(pos.x, pos.y, pos.x + size.width as i32, pos.y + size.height as i32);
    let (x, y, width, height) = match position() {
        TOP => (bounds.left, bounds.top + backend::top_inset(bounds), size.width, thickness as u32),
        LEFT => (bounds.left, bounds.top, thickness as u32, size.height),
        RIGHT => (bounds.right - thickness, bounds.top, thickness as u32, size.height),
        _ => (bounds.left, bounds.bottom - thickness, size.width, thickness as u32),
    };
    // No macOS o tao converte pixels físicos em pontos pela escala da tela onde a janela está
    // agora, não da tela de destino: indo de um Retina para um monitor 1x, a janela cairia no lugar
    // e no tamanho errados. Em pontos (pela escala do destino) ela chega certa.
    #[cfg(target_os = "macos")]
    {
        use tauri::{LogicalPosition, LogicalSize};
        let _ = window.set_size(LogicalSize::new(width as f64 / scale, height as f64 / scale));
        let _ = window.set_position(LogicalPosition::new(x as f64 / scale, y as f64 / scale));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_size(PhysicalSize::new(width, height));
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    Some(Placement { x, y, width, height, band: reserved_band(bounds, reserved), monitor: bounds })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `POSITION` é global e os testes rodam em paralelo: sem a trava, um teste troca a borda
    /// no meio de outro.
    static POSITION_LOCK: Mutex<()> = Mutex::new(());

    fn with_position<T>(position: &str, f: impl FnOnce() -> T) -> T {
        let _guard = POSITION_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = POSITION.swap(parse_position(position), Ordering::SeqCst);
        let result = f();
        POSITION.store(previous, Ordering::SeqCst);
        result
    }

    const MONITOR: Band = Band { left: 0, top: 0, right: 1920, bottom: 1080 };

    #[test]
    fn reserved_band_follows_the_edge() {
        assert_eq!(with_position("bottom", || reserved_band(MONITOR, 72)), Band::new(0, 1008, 1920, 1080));
        assert_eq!(with_position("top", || reserved_band(MONITOR, 72)), Band::new(0, 0, 1920, 72));
        assert_eq!(with_position("left", || reserved_band(MONITOR, 72)), Band::new(0, 0, 72, 1080));
        assert_eq!(with_position("right", || reserved_band(MONITOR, 72)), Band::new(1848, 0, 1920, 1080));
    }

    #[test]
    fn clickable_grows_towards_the_center_of_the_screen() {
        let hit = HitRect { x: 100.0, y: 1008.0, width: 400.0, height: 72.0, radius: 0.0 };
        let window = (1920.0, 1080.0);
        // Na base a área ganha a folga para cima e vai até o pé da janela.
        assert_eq!(with_position("bottom", || clickable(hit, 40.0, window)), (100.0, 968.0, 500.0, 1080.0));
        // Em cima ela ganha para baixo e começa no topo da janela.
        let top = HitRect { x: 100.0, y: 0.0, ..hit };
        assert_eq!(with_position("top", || clickable(top, 40.0, window)), (100.0, 0.0, 500.0, 112.0));
        // Nas laterais a folga é horizontal.
        let left = HitRect { x: 0.0, y: 300.0, width: 72.0, height: 400.0, radius: 0.0 };
        assert_eq!(with_position("left", || clickable(left, 40.0, window)), (0.0, 300.0, 112.0, 700.0));
        let right = HitRect { x: 1848.0, ..left };
        assert_eq!(with_position("right", || clickable(right, 40.0, window)), (1808.0, 300.0, 1920.0, 700.0));
    }

    #[test]
    fn without_a_measured_bar_the_whole_reserved_band_is_clickable() {
        let previous = layout();
        *LAYOUT.lock().unwrap() = Layout { reserved: 72.0, window_size: 170.0, hover_headroom: 80.0 };
        *HIT.lock().unwrap() = HitRect::default();
        let window = (1920.0, 1080.0);
        assert_eq!(with_position("right", || clickable_area(window, 0.0)), (1848.0, 0.0, 1920.0, 1080.0));
        assert_eq!(with_position("bottom", || clickable_area(window, 30.0)), (0.0, 978.0, 1920.0, 1080.0));
        *LAYOUT.lock().unwrap() = previous;
    }

    #[test]
    fn reveal_strip_stays_on_the_dock_edge() {
        let window = (1920.0, 1080.0);
        assert_eq!(with_position("bottom", || reveal_strip(2.0, window)), (0.0, 1078.0, 1920.0, 1080.0));
        assert_eq!(with_position("top", || reveal_strip(2.0, window)), (0.0, 0.0, 1920.0, 2.0));
        assert_eq!(with_position("left", || reveal_strip(2.0, window)), (0.0, 0.0, 2.0, 1080.0));
        assert_eq!(with_position("right", || reveal_strip(2.0, window)), (1918.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn dock_edge_only_reveals_near_the_dock() {
        let window = Band::new(0, 0, 1920, 1080);
        assert!(with_position("bottom", || at_dock_edge(900.0, 1079.0, window, 1.0)));
        assert!(!with_position("bottom", || at_dock_edge(900.0, 1000.0, window, 1.0)));
        assert!(with_position("top", || at_dock_edge(900.0, 1.0, window, 1.0)));
        assert!(with_position("left", || at_dock_edge(1.0, 500.0, window, 1.0)));
        assert!(with_position("right", || at_dock_edge(1919.0, 500.0, window, 1.0)));
        // A dock é uma faixa: encostar na borda fora do comprimento dela não revela nada.
        let band = Band::new(0, 0, 400, 300);
        assert!(!with_position("left", || at_dock_edge(1.0, 600.0, band, 1.0)));
    }

    #[test]
    fn monitors_with_the_same_name_get_a_number() {
        let ids = monitor_ids(&[
            ("Built-in Retina Display".into(), 0, 0),
            ("DELL U2720Q".into(), 3200, 0),
            ("DELL U2720Q".into(), 1440, 0),
        ]);
        // Da esquerda para a direita: o de x 1440 é o primeiro DELL, o de x 3200 o segundo.
        assert_eq!(ids, vec!["Built-in Retina Display", "DELL U2720Q 2", "DELL U2720Q"]);
    }

    #[test]
    fn layout_arrives_in_the_axis_of_the_edge() {
        let layout: Layout = serde_json::from_str(r#"{"reserved":80,"windowSize":260,"hoverHeadroom":180}"#).unwrap();
        assert_eq!((layout.reserved, layout.window_size, layout.hover_headroom), (80.0, 260.0, 180.0));
    }
}
