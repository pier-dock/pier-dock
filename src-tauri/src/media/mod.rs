//! Player de mídia: o que está tocando e os controles. Cada sistema tem um backend com `run`
//! (laço que lê o estado e chama `publish`) e `control`.
//!
//! - Windows: System Media Transport Controls (Spotify, YouTube Music no navegador ou PWA…).
//! - Linux: MPRIS no session bus (os mesmos apps publicam lá).
//! - macOS: AppleScript para Spotify e Música.

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::util::ErrStr;

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MediaSession {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position_ms: f64,
    pub duration_ms: f64,
    /// Momento (Unix ms) em que `position_ms` foi medida — o frontend interpola a partir daí.
    pub updated_at_ms: f64,
    pub thumbnail_key: Option<String>,
    pub can_next: bool,
    pub can_prev: bool,
    pub can_seek: bool,
}

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub sessions: Vec<MediaSession>,
    pub current: Option<String>,
}

static STATE: Mutex<Option<MediaState>> = Mutex::new(None);
static THUMBNAILS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Guarda a capa da faixa (`id|título|artista`) e devolve a chave que o frontend usa para pedi-la.
/// `fetch` só roda se a capa ainda não está no cache; sem capa, tenta de novo no próximo ciclo
/// (navegadores às vezes publicam a capa um pouco depois do título).
fn thumbnail_key(id: &str, title: &str, artist: &str, fetch: impl FnOnce() -> Option<String>) -> Option<String> {
    let key = format!("{id}|{title}|{artist}");
    let mut cache = THUMBNAILS.lock().unwrap();
    let cache = cache.get_or_insert_with(HashMap::new);
    if !cache.contains_key(&key) {
        if let Some(data) = fetch() {
            if cache.len() > 50 {
                cache.clear();
            }
            cache.insert(key.clone(), data);
        }
    }
    cache.contains_key(&key).then_some(key)
}

/// O content type vem do app que publica a sessão de mídia e acaba dentro de um `url()` CSS no
/// frontend: só tipos de imagem conhecidos passam.
fn safe_image_mime(content_type: &str) -> &'static str {
    match content_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "image/jpeg",
        "image/gif" => "image/gif",
        "image/webp" => "image/webp",
        "image/bmp" => "image/bmp",
        _ => "image/png",
    }
}

fn image_data_url(content_type: &str, bytes: &[u8]) -> String {
    use base64::Engine;
    format!("data:{};base64,{}", safe_image_mime(content_type), base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[cfg(unix)]
const MAX_ART_BYTES: usize = 5 * 1024 * 1024;

/// Capa da faixa por HTTPS (o Spotify usa i.scdn.co), como data URL.
#[cfg(unix)]
fn fetch_image(url: &str) -> Option<String> {
    if !url.starts_with("https://") {
        return None;
    }
    let url = url.to_string();
    tauri::async_runtime::block_on(async move {
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build().ok()?;
        let response = client.get(&url).send().await.ok()?.error_for_status().ok()?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = response.bytes().await.ok().filter(|b| b.len() <= MAX_ART_BYTES)?;
        Some(image_data_url(&content_type, &bytes))
    })
}

#[allow(dead_code)]
fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

/// Quanto a posição lida pode se afastar da extrapolada antes de contar como um seek.
const SEEK_TOLERANCE_MS: f64 = 1500.0;

/// `new` é só a reprodução seguindo em frente: nada mudou além de `position_ms`/`updated_at_ms`, e a
/// posição está onde a extrapolação de `last` previa. O frontend interpola a posição sozinho, então
/// emitir a cada leitura só acordaria todas as janelas por nada.
fn is_continuation(last: &MediaState, new: &MediaState) -> bool {
    let without_position = |s: &MediaSession| MediaSession { position_ms: 0.0, updated_at_ms: 0.0, ..s.clone() };
    last.current == new.current
        && last.sessions.len() == new.sessions.len()
        && last.sessions.iter().zip(&new.sessions).all(|(a, b)| {
            let expected = if a.playing { a.position_ms + (b.updated_at_ms - a.updated_at_ms) } else { a.position_ms };
            without_position(a) == without_position(b) && (b.position_ms - expected).abs() <= SEEK_TOLERANCE_MS
        })
}

/// Emite `media-changed` se o estado mudou. `STATE` guarda o último estado emitido, para a posição e
/// o `updated_at_ms` continuarem formando o par que o frontend usa para interpolar.
fn publish(app: &AppHandle, state: MediaState) {
    let mut last = STATE.lock().unwrap();
    if last.as_ref().is_some_and(|last| is_continuation(last, &state)) {
        return;
    }
    let _ = app.emit("media-changed", &state);
    *last = Some(state);
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || backend::run(&app));
}

#[tauri::command]
pub fn media_state() -> MediaState {
    STATE.lock().unwrap().clone().unwrap_or_default()
}

#[tauri::command]
pub fn media_thumbnail(key: String) -> Option<String> {
    THUMBNAILS.lock().unwrap().as_ref().and_then(|c| c.get(&key).cloned())
}

#[tauri::command]
pub async fn media_control(id: String, action: String, position_ms: Option<f64>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::control(&id, &action, position_ms)).await.err_str()?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_mime_is_whitelisted() {
        assert_eq!(safe_image_mime("image/jpeg"), "image/jpeg");
        assert_eq!(safe_image_mime(" IMAGE/WEBP "), "image/webp");
        assert_eq!(safe_image_mime(""), "image/png");
        assert_eq!(safe_image_mime("x),url(https://evil.example/a"), "image/png");
    }

    fn playing(position_ms: f64, updated_at_ms: f64) -> MediaState {
        let session = MediaSession {
            id: "spotify".into(),
            title: "Song".into(),
            playing: true,
            duration_ms: 200_000.0,
            position_ms,
            updated_at_ms,
            ..Default::default()
        };
        MediaState { sessions: vec![session], current: Some("spotify".into()) }
    }

    #[test]
    fn steady_playback_is_not_republished() {
        assert!(is_continuation(&playing(10_000.0, 1_000_000.0), &playing(11_000.0, 1_001_000.0)));
        assert!(is_continuation(&playing(10_000.0, 1_000_000.0), &playing(11_400.0, 1_001_000.0)));
    }

    #[test]
    fn seeks_and_stalls_are_republished() {
        assert!(!is_continuation(&playing(10_000.0, 1_000_000.0), &playing(90_000.0, 1_001_000.0)));
        assert!(!is_continuation(&playing(10_000.0, 1_000_000.0), &playing(2_000.0, 1_001_000.0)));
        assert!(!is_continuation(&playing(10_000.0, 1_000_000.0), &playing(10_000.0, 1_005_000.0)));
    }

    #[test]
    fn track_and_state_changes_are_republished() {
        let last = playing(10_000.0, 1_000_000.0);
        let mut paused = playing(11_000.0, 1_001_000.0);
        paused.sessions[0].playing = false;
        assert!(!is_continuation(&last, &paused));
        let mut next = playing(11_000.0, 1_001_000.0);
        next.sessions[0].title = "Other".into();
        assert!(!is_continuation(&last, &next));
        assert!(!is_continuation(&last, &MediaState::default()));
    }

    #[test]
    fn a_paused_player_stays_quiet_until_the_position_moves() {
        let mut last = playing(10_000.0, 1_000_000.0);
        last.sessions[0].playing = false;
        let mut same = playing(10_000.0, 1_001_000.0);
        same.sessions[0].playing = false;
        assert!(is_continuation(&last, &same));
        let mut moved = playing(60_000.0, 1_002_000.0);
        moved.sessions[0].playing = false;
        assert!(!is_continuation(&last, &moved));
    }
}
