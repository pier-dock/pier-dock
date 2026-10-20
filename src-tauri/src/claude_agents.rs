//! Status ao vivo de sessões do Claude Code — rodando, pedindo permissão, com erro ou concluída —
//! lido dos arquivos que os hooks do Claude Code escrevem (`resources/claude-hooks/pier-status.mjs`,
//! instalado em `~/.claude/settings.json` por `install_claude_hooks`).
//!
//! O Claude Code não guarda esse estado em lugar nenhum por conta própria: só existe enquanto o
//! hook está rodando. O script grava um JSON pequeno por sessão em
//! `<diretório do Claude Code>/pier-dock/agents/<session_id>.json`; esta thread só lê esses
//! arquivos e casa o `pidChain` de cada um (a cadeia de processo-pai no momento do hook) contra as
//! janelas abertas, para saber qual terminal ou IDE focar no clique.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::ai_usage::claude::config_root;
use crate::ai_usage::now_ms;
use crate::util::ErrStr;
use crate::windows_list;

/// O script de hook, embutido no binário — como os ícones da bandeja (`include_bytes!` em
/// `main.rs`). `install_claude_hooks` o escreve em disco na primeira ativação.
const HOOK_SCRIPT: &str = include_str!("../resources/claude-hooks/pier-status.mjs");

/// Precisa bater com o nome do arquivo acima: é por ele que `is_pier_entry` reconhece uma entrada
/// de hook que o Pier adicionou, para não mexer nas do usuário.
const HOOK_FILENAME: &str = "pier-status.mjs";

/// Eventos do Claude Code que alimentam o status. `PostToolUse` sem `matcher` dispara para
/// qualquer ferramenta — é o próprio script que decide se o resultado foi um erro.
const HOOK_EVENTS: [&str; 6] = ["SessionStart", "UserPromptSubmit", "Notification", "PostToolUse", "Stop", "SubagentStop"];

/// `done`/`error` ficam visíveis por isso antes de sumir — tempo de mostrar o resultado.
const FINISHED_TTL_SECS: f64 = 20.0;
/// `running`/`permission` sem atualização por isso são tratados como sessão morta (o hook `Stop`
/// nunca disparou, por exemplo um processo encerrado à força) e removidos.
const STALE_RUNNING_SECS: f64 = 600.0;

fn pier_dock_dir() -> PathBuf {
    config_root().join("pier-dock")
}

fn agents_dir() -> PathBuf {
    pier_dock_dir().join("agents")
}

fn hook_script_path() -> PathBuf {
    pier_dock_dir().join(HOOK_FILENAME)
}

fn settings_json_path() -> PathBuf {
    config_root().join("settings.json")
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct RawStatus {
    session_id: String,
    status: String,
    cwd: String,
    project: String,
    pid_chain: Vec<u32>,
    updated_at: f64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub session_id: String,
    /// `"running" | "permission" | "error" | "done"`.
    pub status: String,
    pub cwd: String,
    pub project: String,
    pub updated_at: f64,
    /// Janela resolvida pelo `pidChain`, quando alguma janela aberta casa com ela.
    pub window_id: Option<i64>,
}

static CACHE: Mutex<Vec<AgentStatus>> = Mutex::new(Vec::new());

fn read_all() -> Vec<(PathBuf, RawStatus)> {
    let Ok(entries) = fs::read_dir(agents_dir()) else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|e| {
            let text = fs::read_to_string(e.path()).ok()?;
            serde_json::from_str(&text).ok().map(|raw| (e.path(), raw))
        })
        .collect()
}

/// A primeira janela aberta cujo dono está na cadeia de processo-pai capturada pelo hook — o
/// terminal ou IDE de onde a sessão foi lançada, não só "o app está aberto em algum lugar".
fn resolve_window(pid_chain: &[u32]) -> Option<i64> {
    windows_list::get_windows().windows.iter().find(|w| pid_chain.contains(&w.pid)).map(|w| w.id)
}

/// `done`/`error` usam `FINISHED_TTL_SECS`; `running`/`permission`, o teto bem mais alto de
/// `STALE_RUNNING_SECS` (sessão morta, hook `Stop` que nunca chegou a disparar).
fn is_expired(status: &str, age_secs: f64) -> bool {
    match status {
        "done" | "error" => age_secs > FINISHED_TTL_SECS,
        _ => age_secs > STALE_RUNNING_SECS,
    }
}

fn collect() -> Vec<AgentStatus> {
    let now = now_ms();
    let mut result: Vec<AgentStatus> = read_all()
        .into_iter()
        .filter_map(|(path, raw)| {
            let age_secs = (now - raw.updated_at) / 1000.0;
            if is_expired(&raw.status, age_secs) {
                let _ = fs::remove_file(&path);
                return None;
            }
            Some(AgentStatus {
                session_id: raw.session_id,
                status: raw.status,
                cwd: raw.cwd,
                project: raw.project,
                updated_at: raw.updated_at,
                window_id: resolve_window(&raw.pid_chain),
            })
        })
        .collect();
    result.sort_by(|a, b| a.updated_at.partial_cmp(&b.updated_at).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Varre a pasta de status a cada segundo (poucos arquivos pequenos: não precisa de um watcher de
/// arquivos) e só emite quando a lista muda, no mesmo molde de `windows_list::start`.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: Option<Vec<AgentStatus>> = None;
        loop {
            let list = collect();
            if last.as_ref() != Some(&list) {
                *CACHE.lock().unwrap() = list.clone();
                let _ = app.emit("claude-agents-changed", &list);
                last = Some(list);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[tauri::command]
pub fn get_claude_agents() -> Vec<AgentStatus> {
    CACHE.lock().unwrap().clone()
}

/// Foca a janela da sessão, se alguma tiver sido resolvida.
#[tauri::command]
pub fn focus_claude_agent(session_id: String) -> Result<(), String> {
    let window_id = CACHE
        .lock()
        .unwrap()
        .iter()
        .find(|a| a.session_id == session_id)
        .ok_or("sessão não encontrada")?
        .window_id
        .ok_or("não achei a janela desta sessão")?;
    windows_list::activate_window(window_id);
    Ok(())
}

// --- instalação dos hooks em ~/.claude/settings.json ------------------------

#[tauri::command]
pub fn claude_hooks_installed() -> bool {
    hook_script_path().is_file()
}

/// Uma entrada de `hooks.<Evento>` é do Pier quando algum comando dela aponta para o script
/// instalado por ele — reconhecido pelo nome do arquivo, não pelo caminho inteiro (que muda de
/// máquina para máquina).
fn is_pier_entry(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|hooks| hooks.iter().any(|h| h.get("command").and_then(|c| c.as_str()).is_some_and(|c| c.contains(HOOK_FILENAME))))
}

fn hook_command() -> String {
    format!("node \"{}\"", hook_script_path().display())
}

fn read_settings() -> Result<Value, String> {
    let path = settings_json_path();
    if !path.is_file() {
        return Ok(json!({}));
    }
    serde_json::from_str(&fs::read_to_string(&path).err_str()?).err_str()
}

/// Grava com um backup do arquivo anterior do lado, por segurança — é uma edição de um arquivo de
/// configuração de outra ferramenta, feita fora do controle do usuário.
fn write_settings(settings: &Value) -> Result<(), String> {
    let path = settings_json_path();
    if path.is_file() {
        let backup = pier_dock_dir().join(format!("settings.json.bak-{}", now_ms() as i64));
        let _ = fs::copy(&path, backup);
    }
    fs::write(&path, serde_json::to_string_pretty(settings).err_str()?).err_str()
}

/// Acrescenta as entradas de hook do Pier (uma por evento de `HOOK_EVENTS`) em `settings`, sem
/// tocar nas que já existirem — nem as de outra ferramenta, nem uma do Pier já presente.
fn add_pier_hooks(settings: &mut Value, command: &str) -> Result<(), String> {
    let obj = settings.as_object_mut().ok_or("settings.json: formato inesperado (não é um objeto)")?;
    let hooks = obj.entry("hooks").or_insert_with(|| json!({})).as_object_mut().ok_or("settings.json: \"hooks\" não é um objeto")?;
    for event in HOOK_EVENTS {
        let list = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| format!("settings.json: hooks.{event} não é uma lista"))?;
        if !list.iter().any(is_pier_entry) {
            list.push(json!({ "hooks": [{ "type": "command", "command": command }] }));
        }
    }
    Ok(())
}

/// Remove só as entradas de hook do Pier (as de `is_pier_entry`); o resto de `settings` (inclusive
/// hooks de outra ferramenta nos mesmos eventos) fica como estava.
fn remove_pier_hooks(settings: &mut Value) {
    let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) else { return };
    for event in HOOK_EVENTS {
        if let Some(list) = hooks.get_mut(event).and_then(|l| l.as_array_mut()) {
            list.retain(|entry| !is_pier_entry(entry));
        }
    }
}

/// Mescla as entradas de hook do Pier em `~/.claude/settings.json`, sem tocar nas que já existirem.
/// Instala o script embutido do lado do arquivo de configuração.
#[tauri::command]
pub fn install_claude_hooks() -> Result<(), String> {
    fs::create_dir_all(pier_dock_dir()).err_str()?;
    fs::create_dir_all(agents_dir()).err_str()?;
    fs::write(hook_script_path(), HOOK_SCRIPT).err_str()?;

    let mut settings = read_settings()?;
    add_pier_hooks(&mut settings, &hook_command())?;
    write_settings(&settings)
}

/// Remove só as entradas de hook do Pier e o script instalado; o resto de `settings.json` fica
/// como estava.
#[tauri::command]
pub fn uninstall_claude_hooks() -> Result<(), String> {
    let path = settings_json_path();
    if path.is_file() {
        let mut settings = read_settings()?;
        remove_pier_hooks(&mut settings);
        write_settings(&settings)?;
    }
    let _ = fs::remove_file(hook_script_path());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_expires_only_after_the_long_ttl() {
        assert!(!is_expired("running", STALE_RUNNING_SECS - 1.0));
        assert!(is_expired("running", STALE_RUNNING_SECS + 1.0));
        assert!(!is_expired("permission", STALE_RUNNING_SECS - 1.0));
    }

    #[test]
    fn finished_states_expire_after_the_short_ttl() {
        assert!(!is_expired("done", FINISHED_TTL_SECS - 1.0));
        assert!(is_expired("done", FINISHED_TTL_SECS + 1.0));
        assert!(is_expired("error", FINISHED_TTL_SECS + 1.0));
    }

    #[test]
    fn recognizes_only_its_own_hook_command() {
        let mine = json!({ "hooks": [{ "type": "command", "command": "node \"/home/x/.claude/pier-dock/pier-status.mjs\"" }] });
        let other = json!({ "hooks": [{ "type": "command", "command": "echo hi" }] });
        assert!(is_pier_entry(&mine));
        assert!(!is_pier_entry(&other));
    }

    #[test]
    fn adding_hooks_is_idempotent_and_keeps_other_tools_entries() {
        let mut settings = json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }],
            },
            "otherSetting": true,
        });
        add_pier_hooks(&mut settings, "node \"/x/pier-status.mjs\"").unwrap();
        // De novo: não deve duplicar a entrada do Pier nem mexer na do usuário.
        add_pier_hooks(&mut settings, "node \"/x/pier-status.mjs\"").unwrap();

        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2, "{stop:?}");
        assert!(stop.iter().any(|e| !is_pier_entry(e)), "a entrada de outra ferramenta sumiu");
        assert_eq!(stop.iter().filter(|e| is_pier_entry(e)).count(), 1, "duplicou a entrada do Pier");
        assert_eq!(settings["otherSetting"], true);
        for event in HOOK_EVENTS {
            assert!(settings["hooks"][event].as_array().is_some(), "faltou hooks.{event}");
        }
    }

    #[test]
    fn removing_hooks_keeps_other_tools_entries() {
        let mut settings = json!({
            "hooks": {
                "Stop": [
                    { "hooks": [{ "type": "command", "command": "echo done" }] },
                    { "hooks": [{ "type": "command", "command": "node \"/x/pier-status.mjs\"" }] },
                ],
            },
        });
        remove_pier_hooks(&mut settings);
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
        assert!(!is_pier_entry(&stop[0]));
    }
}
