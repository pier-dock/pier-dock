//! Mídia no macOS via AppleScript: Spotify e Música. (O MediaRemote, que veria qualquer player,
//! é privado e desde o macOS 15.4 só responde a apps da Apple.) O id da sessão é o bundle id.

use std::time::Duration;

use objc2::rc::autoreleasepool;
use tauri::AppHandle;

use super::{fetch_image, image_data_url, now_ms, publish, thumbnail_key, MediaSession, MediaState, MAX_ART_BYTES};
use crate::platform::macos::{osascript, workspace};

struct Player {
    id: &'static str,
    app: &'static str,
    /// Duração da faixa em ms (o Spotify já dá em ms; o Música, em segundos).
    duration_ms: &'static str,
    /// URL HTTPS da capa; o Música não tem, a capa vem de `raw data`.
    artwork_url: &'static str,
}

const PLAYERS: [Player; 2] = [
    Player { id: "com.spotify.client", app: "Spotify", duration_ms: "duration of t", artwork_url: "artwork url of t" },
    Player { id: "com.apple.Music", app: "Music", duration_ms: "(duration of t * 1000) as integer", artwork_url: "\"\"" },
];

fn player(id: &str) -> Option<&'static Player> {
    PLAYERS.iter().find(|p| p.id == id)
}

/// `tell application` abriria o app se ele estivesse fechado.
fn running(player: &Player) -> bool {
    workspace::pid_of(player.id).is_some()
}

/// Uma linha por faixa, com os campos separados pelo caractere 31 (unit separator), que não
/// aparece em títulos. A posição vai como inteiro em ms para não depender do separador decimal
/// do idioma do sistema.
fn track_script(p: &Player) -> String {
    format!(
        r#"tell application "{app}"
	if player state is stopped then return ""
	set sep to character id 31
	set t to current track
	return (name of t) & sep & (artist of t) & sep & (album of t) & sep & (player state as text) & sep & ((player position * 1000) as integer) & sep & ({duration}) & sep & ({artwork})
end tell"#,
        app = p.app,
        duration = p.duration_ms,
        artwork = p.artwork_url,
    )
}

#[derive(Debug, PartialEq)]
struct Track {
    title: String,
    artist: String,
    album: String,
    playing: bool,
    position_ms: f64,
    duration_ms: f64,
    artwork_url: String,
}

fn parse_track(output: &str) -> Option<Track> {
    let fields: Vec<&str> = output.split('\u{1f}').collect();
    let [title, artist, album, state, position, duration, artwork] = fields[..] else { return None };
    if title.is_empty() {
        return None;
    }
    Some(Track {
        title: title.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        playing: state == "playing",
        position_ms: position.trim().parse().unwrap_or(0.0),
        duration_ms: duration.trim().parse().unwrap_or(0.0),
        artwork_url: artwork.trim().to_string(),
    })
}

/// Bytes de um `«data JPEG FFD8…»`, que é como o AppleScript imprime dados binários.
fn parse_raw_data(output: &str) -> Option<Vec<u8>> {
    let start = output.find("«data ")? + "«data ".len();
    let rest = output.get(start..)?;
    let hex = rest.get(4..)?.split('»').next()?.trim();
    if hex.len() % 2 != 0 || hex.len() > MAX_ART_BYTES * 2 {
        return None;
    }
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok()).collect()
}

fn music_artwork() -> Option<String> {
    let output = osascript(r#"tell application "Music" to get raw data of artwork 1 of current track"#)?;
    let bytes = parse_raw_data(&output)?;
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
    Some(image_data_url(mime, &bytes))
}

fn read_player(p: &Player) -> Option<MediaSession> {
    if !running(p) {
        return None;
    }
    let track = parse_track(&osascript(&track_script(p))?)?;
    let artwork_url = track.artwork_url.clone();
    let fetch = || if artwork_url.is_empty() { music_artwork() } else { fetch_image(&artwork_url) };
    Some(MediaSession {
        id: p.id.to_string(),
        thumbnail_key: thumbnail_key(p.id, &track.title, &track.artist, fetch),
        title: track.title,
        artist: track.artist,
        album: track.album,
        playing: track.playing,
        position_ms: track.position_ms,
        duration_ms: track.duration_ms,
        updated_at_ms: now_ms(),
        can_next: true,
        can_prev: true,
        can_seek: true,
    })
}

fn read_state() -> MediaState {
    let sessions: Vec<MediaSession> = PLAYERS.iter().filter_map(read_player).collect();
    let current = sessions.iter().find(|s| s.playing).or(sessions.first()).map(|s| s.id.clone());
    MediaState { sessions, current }
}

pub fn run(app: &AppHandle) {
    loop {
        publish(app, autoreleasepool(|_| read_state()));
        std::thread::sleep(Duration::from_millis(1000));
    }
}

pub fn control(id: &str, action: &str, position_ms: Option<f64>) -> Result<(), String> {
    // Só os players conhecidos: o id vem do frontend e acaba dentro de um script.
    let p = player(id).filter(|p| running(p)).ok_or(crate::i18n::tr("player não encontrado", "player not found"))?;
    let command = match action {
        "toggle" => "playpause".to_string(),
        "next" => "next track".to_string(),
        "prev" => "previous track".to_string(),
        "seek" => format!("set player position to {:.3}", position_ms.unwrap_or(0.0).max(0.0) / 1000.0),
        _ => return Ok(()),
    };
    osascript(&format!(r#"tell application "{}" to {command}"#, p.app)).map(|_| ()).ok_or_else(|| format!("falha ao controlar o {}", p.app))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_track_fields() {
        let out = "Song\u{1f}Artist\u{1f}Album\u{1f}playing\u{1f}12345\u{1f}200000\u{1f}https://i.scdn.co/image/x";
        let track = parse_track(out).unwrap();
        assert_eq!(track.title, "Song");
        assert!(track.playing);
        assert_eq!(track.position_ms, 12345.0);
        assert_eq!(track.duration_ms, 200000.0);
        assert_eq!(track.artwork_url, "https://i.scdn.co/image/x");
        assert_eq!(parse_track(""), None);
        assert_eq!(parse_track("\u{1f}a\u{1f}b\u{1f}paused\u{1f}0\u{1f}0\u{1f}"), None);
    }

    #[test]
    fn parses_applescript_raw_data() {
        assert_eq!(parse_raw_data("«data JPEGFFD8FF00»"), Some(vec![0xFF, 0xD8, 0xFF, 0x00]));
        assert_eq!(parse_raw_data("«data PNGf89504E47»"), Some(vec![0x89, 0x50, 0x4E, 0x47]));
        assert_eq!(parse_raw_data("«data JPEGFFD»"), None);
        assert_eq!(parse_raw_data("missing value"), None);
    }

    #[test]
    #[ignore = "smoke test: lê o Spotify/Música reais"]
    fn reads_media_sessions() {
        println!("{:#?}", read_state());
    }
}
