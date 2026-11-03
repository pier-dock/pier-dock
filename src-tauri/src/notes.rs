//! Widget de lembretes (post-it). Cada lembrete é um arquivo `.md` em `Documentos/Pier/Lembretes`,
//! então dá para editá-los fora do app. A cor fica num frontmatter mínimo (`color: yellow`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::util::{write_atomic, ErrStr};

/// Limite de caracteres (não bytes) de um lembrete. Espelhado em `MAX_NOTE_CHARS` no `ipc.ts`.
pub const MAX_NOTE_CHARS: usize = 280;
const MAX_NOTES: usize = 50;
const COLORS: [&str; 4] = ["yellow", "pink", "blue", "green"];

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// Nome do arquivo, sem a extensão.
    pub id: String,
    pub text: String,
    pub color: String,
    pub modified_at: f64,
}

fn notes_dir() -> PathBuf {
    let base = dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(|| ".".into());
    base.join("Pier").join("Lembretes")
}

fn invalid() -> String {
    crate::i18n::tr("Lembrete inválido.", "Invalid reminder.").to_string()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn clamp_text(text: &str) -> String {
    text.trim().chars().take(MAX_NOTE_CHARS).collect()
}

fn normalize_color(color: &str) -> &'static str {
    COLORS.iter().find(|c| **c == color).copied().unwrap_or(COLORS[0])
}

fn serialize(text: &str, color: &str) -> String {
    format!("---\ncolor: {color}\n---\n{text}\n")
}

/// Lê o frontmatter se houver; arquivos escritos à mão, sem ele, viram lembretes amarelos.
fn parse(raw: &str) -> (String, String) {
    let raw = raw.replace("\r\n", "\n");
    if let Some(rest) = raw.strip_prefix("---\n") {
        if let Some((head, body)) = rest.split_once("\n---\n") {
            let color = head
                .lines()
                .find_map(|l| l.strip_prefix("color:"))
                .map(|c| normalize_color(c.trim()))
                .unwrap_or(COLORS[0]);
            return (clamp_text(body), color.to_string());
        }
    }
    (clamp_text(&raw), COLORS[0].to_string())
}

fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

fn list_in(dir: &Path) -> Vec<Note> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut notes: Vec<Note> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "md" {
                return None;
            }
            let id = path.file_stem()?.to_str()?.to_string();
            if !valid_id(&id) {
                return None;
            }
            let (text, color) = parse(&fs::read_to_string(&path).ok()?);
            let modified_at = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0);
            Some(Note { id, text, color, modified_at })
        })
        .filter(|n| !n.text.is_empty())
        .collect();
    notes.sort_by(|a, b| b.modified_at.total_cmp(&a.modified_at).then_with(|| a.id.cmp(&b.id)));
    notes.truncate(MAX_NOTES);
    notes
}

fn save_in(dir: &Path, id: Option<&str>, text: &str, color: &str) -> Result<String, String> {
    let text = clamp_text(text);
    if text.is_empty() {
        return Err(crate::i18n::tr("O lembrete está vazio.", "The reminder is empty.").to_string());
    }
    fs::create_dir_all(dir).err_str()?;
    let id = match id {
        Some(id) if valid_id(id) => id.to_string(),
        Some(_) => return Err(invalid()),
        None => {
            let mut stamp = now_ms() as u64;
            while dir.join(format!("lembrete-{stamp}.md")).exists() {
                stamp += 1;
            }
            format!("lembrete-{stamp}")
        }
    };
    write_atomic(&dir.join(format!("{id}.md")), serialize(&text, normalize_color(color))).err_str()?;
    Ok(id)
}

fn delete_in(dir: &Path, id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err(invalid());
    }
    match fs::remove_file(dir.join(format!("{id}.md"))) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

fn publish(app: &AppHandle) -> Vec<Note> {
    let notes = list_in(&notes_dir());
    let _ = app.emit("notes-updated", &notes);
    notes
}

#[tauri::command]
pub async fn notes_list() -> Result<Vec<Note>, String> {
    tauri::async_runtime::spawn_blocking(|| list_in(&notes_dir())).await.err_str()
}

#[tauri::command]
pub async fn notes_save(app: AppHandle, id: Option<String>, text: String, color: String) -> Result<Vec<Note>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        save_in(&notes_dir(), id.as_deref(), &text, &color)?;
        Ok(publish(&app))
    })
    .await
    .err_str()?
}

#[tauri::command]
pub async fn notes_delete(app: AppHandle, id: String) -> Result<Vec<Note>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        delete_in(&notes_dir(), &id)?;
        Ok(publish(&app))
    })
    .await
    .err_str()?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pier-notes-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn saves_lists_and_deletes_notes() {
        let dir = temp_dir("roundtrip");
        let id = save_in(&dir, None, "  comprar pão  ", "pink").unwrap();
        let notes = list_in(&dir);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, id);
        assert_eq!((notes[0].text.as_str(), notes[0].color.as_str()), ("comprar pão", "pink"));
        save_in(&dir, Some(&id), "comprar leite", "blue").unwrap();
        assert_eq!(list_in(&dir)[0].text, "comprar leite");
        delete_in(&dir, &id).unwrap();
        assert!(list_in(&dir).is_empty());
        delete_in(&dir, &id).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_is_limited_by_characters_not_bytes() {
        let long = "é😀".repeat(MAX_NOTE_CHARS);
        assert_eq!(clamp_text(&long).chars().count(), MAX_NOTE_CHARS);
        let dir = temp_dir("limit");
        save_in(&dir, None, &long, "yellow").unwrap();
        assert_eq!(list_in(&dir)[0].text.chars().count(), MAX_NOTE_CHARS);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_empty_text_and_unsafe_ids() {
        let dir = temp_dir("unsafe");
        assert!(save_in(&dir, None, "   ", "yellow").is_err());
        assert!(save_in(&dir, Some("../evil"), "x", "yellow").is_err());
        assert!(delete_in(&dir, "../evil").is_err());
        assert!(delete_in(&dir, "a/b").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_frontmatter_and_plain_files() {
        assert_eq!(parse("---\ncolor: green\n---\nolá\n"), ("olá".into(), "green".into()));
        assert_eq!(parse("---\ncolor: roxo\n---\nolá"), ("olá".into(), "yellow".into()));
        assert_eq!(parse("só texto\n"), ("só texto".into(), "yellow".into()));
        assert_eq!(parse("---\r\ncolor: blue\r\n---\r\nx"), ("x".into(), "blue".into()));
    }
}
