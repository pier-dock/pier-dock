//! Status ao vivo de sessões de CLIs de IA — rodando, pedindo permissão, com erro ou concluída —
//! lido dos arquivos que os hooks dos próprios CLIs escrevem (`resources/agent-hooks/pier-status.mjs`
//! e o plugin do opencode). Cada integração é instalada em `install_agent_integration`.
//!
//! As ferramentas não guardam esse estado em lugar nenhum por conta própria: só existe enquanto o
//! hook está rodando. O script grava um JSON pequeno por sessão em
//! `<pasta de dados do Pier>/pier-dock/agents/<provider>-<session_id>.json`; esta thread só lê esses
//! arquivos e casa o `pidChain` de cada um (a cadeia de processo-pai no momento do hook) contra as
//! janelas abertas, para saber qual terminal ou IDE focar no clique.
//!
//! Nada do conteúdo das conversas é lido ou guardado: só provider, status, cwd, projeto e horário.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::ai_usage::claude::config_root as claude_config_root;
use crate::ai_usage::now_ms;
use crate::util::{data_dir, ErrStr};
use crate::windows_list;

/// O script de hook, embutido no binário — como os ícones da bandeja (`include_bytes!` em
/// `main.rs`). A instalação o escreve em disco.
const HOOK_SCRIPT: &str = include_str!("../resources/agent-hooks/pier-status.mjs");

/// O plugin do opencode, também embutido. Ele lê os eventos da sessão e grava o mesmo formato.
const OPENCODE_PLUGIN: &str = include_str!("../resources/agent-hooks/opencode.js");

/// Precisa bater com o nome do arquivo acima: é por ele que `is_pier_entry` reconhece uma entrada
/// de hook que o Pier adicionou, para não mexer nas do usuário.
const HOOK_FILENAME: &str = "pier-status.mjs";
const OPENCODE_PLUGIN_FILENAME: &str = "pier-status.js";

/// `done`/`error` ficam visíveis por isso antes de sumir — tempo de mostrar o resultado.
const FINISHED_TTL_SECS: f64 = 20.0;
/// `running`/`permission` sem atualização por isso são tratados como sessão morta (o hook `Stop`
/// nunca disparou, por exemplo um processo encerrado à força) e removidos.
const STALE_RUNNING_SECS: f64 = 600.0;

pub const PROVIDERS: [&str; 4] = ["claude", "codex", "opencode", "gemini"];

/// Eventos de cada CLI que alimentam o status. O script é o mesmo; só a lista de eventos muda.
const CLAUDE_EVENTS: [&str; 7] = [
    "SessionStart",
    "UserPromptSubmit",
    "Notification",
    "PostToolUse",
    "Stop",
    "SubagentStop",
    "SessionEnd",
];
const CODEX_EVENTS: [&str; 7] = [
    "SessionStart",
    "UserPromptSubmit",
    "PermissionRequest",
    "PostToolUse",
    "Stop",
    "SubagentStop",
    "SessionEnd",
];
const GEMINI_EVENTS: [&str; 6] = ["SessionStart", "BeforeAgent", "Notification", "AfterTool", "AfterAgent", "SessionEnd"];

fn pier_dock_dir() -> PathBuf {
    data_dir().join("pier-dock")
}

fn agents_dir() -> PathBuf {
    pier_dock_dir().join("agents")
}

/// Durante a transição, a versão anterior escrevia em `~/.claude/pier-dock/agents`.
fn legacy_agents_dir() -> PathBuf {
    claude_config_root().join("pier-dock").join("agents")
}

fn hook_script_path() -> PathBuf {
    pier_dock_dir().join(HOOK_FILENAME)
}

fn legacy_hook_script_path() -> PathBuf {
    claude_config_root().join("pier-dock").join(HOOK_FILENAME)
}

fn claude_settings_path() -> PathBuf {
    claude_config_root().join("settings.json")
}

fn codex_root() -> PathBuf {
    match std::env::var_os("CODEX_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::home_dir().unwrap_or_else(|| ".".into()).join(".codex"),
    }
}

fn codex_hooks_path() -> PathBuf {
    codex_root().join("hooks.json")
}

fn gemini_root() -> PathBuf {
    match std::env::var_os("GEMINI_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::home_dir().unwrap_or_else(|| ".".into()).join(".gemini"),
    }
}

fn gemini_settings_path() -> PathBuf {
    gemini_root().join("settings.json")
}

/// O opencode segue os diretórios XDG em todas as plataformas (no Windows também é `~/.config`).
fn opencode_config_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("OPENCODE_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| ".".into()).join(".config"));
    base.join("opencode")
}

fn opencode_plugin_path() -> PathBuf {
    opencode_config_root().join("plugins").join(OPENCODE_PLUGIN_FILENAME)
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct RawStatus {
    #[serde(default = "default_provider")]
    provider: String,
    session_id: String,
    status: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    project: String,
    #[serde(default)]
    pid_chain: Vec<u32>,
    #[serde(default)]
    updated_at: f64,
}

fn default_provider() -> String {
    "claude".into()
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    /// `"claude" | "codex" | "opencode" | "gemini"`.
    pub provider: String,
    pub session_id: String,
    /// `"running" | "permission" | "error" | "done"`.
    pub status: String,
    pub cwd: String,
    pub project: String,
    pub updated_at: f64,
    /// Janela resolvida pelo `pidChain`, quando alguma janela aberta casa com ela.
    pub window_id: Option<i64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentIntegration {
    pub id: String,
    /// Se a CLI/pasta de configuração existe nesta máquina.
    pub detected: bool,
    /// Se os hooks (ou o plugin) do Pier estão instalados.
    pub installed: bool,
}

static CACHE: Mutex<Vec<AgentSession>> = Mutex::new(Vec::new());

fn read_dir_sessions(dir: &Path, forced_provider: Option<&str>) -> Vec<(PathBuf, RawStatus)> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|e| {
            let text = fs::read_to_string(e.path()).ok()?;
            let mut raw: RawStatus = serde_json::from_str(&text).ok()?;
            if let Some(provider) = forced_provider {
                raw.provider = provider.into();
            }
            if !PROVIDERS.contains(&raw.provider.as_str()) || raw.session_id.is_empty() {
                return None;
            }
            Some((e.path(), raw))
        })
        .collect()
}

/// Lê o diretório novo e o antigo do Claude. Se a mesma sessão aparecer nos dois, fica a leitura
/// mais recente e o arquivo perdedor é removido.
fn read_all() -> Vec<(PathBuf, RawStatus)> {
    let mut merged: std::collections::HashMap<(String, String), (PathBuf, RawStatus)> = std::collections::HashMap::new();
    for (path, raw) in read_dir_sessions(&legacy_agents_dir(), Some("claude")).into_iter().chain(read_dir_sessions(&agents_dir(), None)) {
        let key = (raw.provider.clone(), raw.session_id.clone());
        match merged.get(&key) {
            Some((_, current)) if current.updated_at > raw.updated_at => {
                let _ = fs::remove_file(path);
            }
            Some((old_path, _)) => {
                let old_path = old_path.clone();
                merged.insert(key, (path, raw));
                let _ = fs::remove_file(old_path);
            }
            None => {
                merged.insert(key, (path, raw));
            }
        }
    }
    merged.into_values().collect()
}

/// A janela do terminal/IDE de onde a sessão foi lançada, não só "o app está aberto em algum
/// lugar". Um mesmo processo pode ter várias janelas (o VS Code, por exemplo, abre todas no mesmo
/// processo, e a cadeia de pids só chega até ele): nesse caso o nome do projeto no título desempata.
/// Sem título que bata, fica a primeira, que está no topo da ordem Z.
fn pick_window(windows: &[windows_list::WindowInfo], pid_chain: &[u32], project: &str) -> Option<i64> {
    let mut fallback = None;
    let needle = project.trim().to_lowercase();
    for window in windows.iter().filter(|w| pid_chain.contains(&w.pid)) {
        if !needle.is_empty() && window.title.to_lowercase().contains(&needle) {
            return Some(window.id);
        }
        fallback.get_or_insert(window.id);
    }
    fallback
}

fn resolve_window(pid_chain: &[u32], project: &str) -> Option<i64> {
    pick_window(&windows_list::get_windows().windows, pid_chain, project)
}

/// `done`/`error` usam `FINISHED_TTL_SECS`; `running`/`permission`, o teto bem mais alto de
/// `STALE_RUNNING_SECS` (sessão morta, hook `Stop` que nunca chegou a disparar).
fn is_expired(status: &str, age_secs: f64) -> bool {
    match status {
        "done" | "error" => age_secs > FINISHED_TTL_SECS,
        _ => age_secs > STALE_RUNNING_SECS,
    }
}

fn collect() -> Vec<AgentSession> {
    let now = now_ms();
    let mut result: Vec<AgentSession> = read_all()
        .into_iter()
        .filter_map(|(path, raw)| {
            let age_secs = (now - raw.updated_at) / 1000.0;
            if is_expired(&raw.status, age_secs) {
                let _ = fs::remove_file(&path);
                return None;
            }
            let window_id = resolve_window(&raw.pid_chain, &raw.project);
            Some(AgentSession {
                provider: raw.provider,
                session_id: raw.session_id,
                status: raw.status,
                cwd: raw.cwd,
                project: raw.project,
                updated_at: raw.updated_at,
                window_id,
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
        let mut last: Option<Vec<AgentSession>> = None;
        loop {
            let list = collect();
            if last.as_ref() != Some(&list) {
                *CACHE.lock().unwrap() = list.clone();
                let _ = app.emit("agent-sessions-changed", &list);
                last = Some(list);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[tauri::command]
pub fn get_agent_sessions() -> Vec<AgentSession> {
    CACHE.lock().unwrap().clone()
}

/// Foca a janela da sessão, se alguma tiver sido resolvida.
#[tauri::command]
pub fn focus_agent_session(provider: String, session_id: String) -> Result<(), String> {
    let window_id = CACHE
        .lock()
        .unwrap()
        .iter()
        .find(|a| a.provider == provider && a.session_id == session_id)
        .ok_or("sessão não encontrada")?
        .window_id
        .ok_or("não achei a janela desta sessão")?;
    windows_list::activate_window(window_id);
    Ok(())
}

// --- instalação das integrações ---------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum HookFormat {
    Claude,
    Codex,
    Gemini,
}

impl HookFormat {
    fn events(self) -> &'static [&'static str] {
        match self {
            Self::Claude => &CLAUDE_EVENTS,
            Self::Codex => &CODEX_EVENTS,
            Self::Gemini => &GEMINI_EVENTS,
        }
    }

    fn settings_path(self) -> PathBuf {
        match self {
            Self::Claude => claude_settings_path(),
            Self::Codex => codex_hooks_path(),
            Self::Gemini => gemini_settings_path(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Claude => "settings.json do Claude Code",
            Self::Codex => "hooks.json do Codex",
            Self::Gemini => "settings.json do Gemini",
        }
    }
}

fn hook_command(provider: &str) -> String {
    format!("node \"{}\" --provider {provider} --dir \"{}\"", hook_script_path().display(), agents_dir().display())
}

/// Uma entrada de `hooks.<Evento>` é do Pier quando algum comando dela aponta para o script
/// instalado por ele — reconhecido pelo nome do arquivo, não pelo caminho inteiro (que muda de
/// máquina para máquina e mudou quando o diretório passou a ser compartilhado).
fn is_pier_entry(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|hooks| hooks.iter().any(|h| h.get("command").and_then(|c| c.as_str()).is_some_and(|c| c.contains(HOOK_FILENAME))))
}

fn read_json(path: &Path) -> Result<Value, String> {
    if !path.is_file() {
        return Ok(json!({}));
    }
    serde_json::from_str(&fs::read_to_string(path).err_str()?).err_str()
}

/// Grava com um backup do arquivo anterior do lado, por segurança — é uma edição de um arquivo de
/// configuração de outra ferramenta, feita fora do controle do usuário.
fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).err_str()?;
    }
    if path.is_file() {
        let backup = path.with_extension(format!("json.bak-{}", now_ms() as i64));
        let _ = fs::copy(path, backup);
    }
    fs::write(path, serde_json::to_string_pretty(value).err_str()?).err_str()
}

/// Acrescenta as entradas de hook do Pier, uma por evento, substituindo entradas antigas do Pier
/// (para atualizar o comando) sem tocar nas de outras ferramentas.
fn add_pier_hooks(settings: &mut Value, format: HookFormat, command: &str) -> Result<(), String> {
    let label = format.label();
    let obj = settings.as_object_mut().ok_or_else(|| format!("{label}: formato inesperado (não é um objeto)"))?;
    let hooks = obj.entry("hooks").or_insert_with(|| json!({})).as_object_mut().ok_or_else(|| format!("{label}: \"hooks\" não é um objeto"))?;
    for event in format.events() {
        let list = hooks
            .entry(*event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| format!("{label}: hooks.{event} não é uma lista"))?;
        list.retain(|entry| !is_pier_entry(entry));
        let mut hook = json!({ "type": "command", "command": command });
        if format == HookFormat::Gemini {
            hook["name"] = "Pier status".into();
        }
        list.push(json!({ "hooks": [hook] }));
    }
    if format == HookFormat::Gemini {
        let hooks_config = obj.entry("hooksConfig").or_insert_with(|| json!({})).as_object_mut().ok_or("settings.json do Gemini: \"hooksConfig\" não é um objeto")?;
        hooks_config.insert("enabled".into(), true.into());
    }
    Ok(())
}

fn remove_pier_hooks(settings: &mut Value, format: HookFormat) {
    let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) else { return };
    for event in format.events() {
        if let Some(list) = hooks.get_mut(*event).and_then(|l| l.as_array_mut()) {
            list.retain(|entry| !is_pier_entry(entry));
        }
    }
}

fn hooks_installed(format: HookFormat) -> bool {
    read_json(&format.settings_path())
        .ok()
        .and_then(|settings| settings.get("hooks").and_then(|h| h.as_object()).cloned())
        .is_some_and(|hooks| {
            format.events().iter().any(|event| {
                hooks
                    .get(*event)
                    .and_then(|l| l.as_array())
                    .is_some_and(|list| list.iter().any(is_pier_entry))
            })
        })
}

fn opencode_installed() -> bool {
    opencode_plugin_path().is_file()
}

fn detected(provider: &str) -> bool {
    match provider {
        "claude" => claude_config_root().is_dir(),
        "codex" => codex_root().is_dir(),
        "opencode" => opencode_config_root().is_dir(),
        "gemini" => gemini_root().is_dir(),
        _ => false,
    }
}

fn installed(provider: &str) -> bool {
    match provider {
        "claude" => hooks_installed(HookFormat::Claude),
        "codex" => hooks_installed(HookFormat::Codex),
        "gemini" => hooks_installed(HookFormat::Gemini),
        "opencode" => opencode_installed(),
        _ => false,
    }
}

#[tauri::command]
pub fn agent_integrations() -> Vec<AgentIntegration> {
    PROVIDERS
        .iter()
        .map(|id| AgentIntegration { id: (*id).into(), detected: detected(id), installed: installed(id) })
        .collect()
}

fn install_hooks(format: HookFormat, provider: &str) -> Result<(), String> {
    fs::create_dir_all(agents_dir()).err_str()?;
    fs::write(hook_script_path(), HOOK_SCRIPT).err_str()?;
    let path = format.settings_path();
    let mut settings = read_json(&path)?;
    add_pier_hooks(&mut settings, format, &hook_command(provider))?;
    write_json(&path, &settings)
}

fn remove_hooks(format: HookFormat) -> Result<(), String> {
    let path = format.settings_path();
    if path.is_file() {
        let mut settings = read_json(&path)?;
        remove_pier_hooks(&mut settings, format);
        write_json(&path, &settings)?;
    }
    Ok(())
}

fn remove_shared_script_if_unused() {
    if PROVIDERS.iter().any(|provider| installed(provider)) {
        return;
    }
    let _ = fs::remove_file(hook_script_path());
    let _ = fs::remove_file(legacy_hook_script_path());
}

#[tauri::command]
pub fn install_agent_integration(provider: String) -> Result<(), String> {
    match provider.as_str() {
        "claude" => install_hooks(HookFormat::Claude, "claude"),
        "codex" => install_hooks(HookFormat::Codex, "codex"),
        "gemini" => install_hooks(HookFormat::Gemini, "gemini"),
        "opencode" => {
            let path = opencode_plugin_path();
            fs::create_dir_all(path.parent().ok_or("caminho de plugin inválido")?).err_str()?;
            fs::write(path, OPENCODE_PLUGIN).err_str()
        }
        _ => Err(format!("integração desconhecida: {provider}")),
    }
}

#[tauri::command]
pub fn uninstall_agent_integration(provider: String) -> Result<(), String> {
    match provider.as_str() {
        "claude" => remove_hooks(HookFormat::Claude)?,
        "codex" => remove_hooks(HookFormat::Codex)?,
        "gemini" => remove_hooks(HookFormat::Gemini)?,
        "opencode" => {
            let _ = fs::remove_file(opencode_plugin_path());
        }
        _ => return Err(format!("integração desconhecida: {provider}")),
    }
    remove_shared_script_if_unused();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(id: i64, pid: u32, title: &str) -> windows_list::WindowInfo {
        windows_list::WindowInfo { id, title: title.into(), exe: String::new(), aumid: None, minimized: false, pid, badge: 0 }
    }

    #[test]
    fn several_windows_of_the_same_process_match_the_project_title() {
        let windows = vec![
            window(1, 3832, "new-cemiterio - Visual Studio Code"),
            window(2, 3832, "win-dock - Visual Studio Code"),
        ];
        assert_eq!(pick_window(&windows, &[3832], "win-dock"), Some(2));
        assert_eq!(pick_window(&windows, &[3832], "new-cemiterio"), Some(1));
    }

    #[test]
    fn without_a_matching_title_the_first_window_of_the_chain_wins() {
        let windows = vec![
            window(1, 10, "Terminal"),
            window(2, 20, "outro - Visual Studio Code"),
            window(3, 20, "mais outro - Visual Studio Code"),
        ];
        assert_eq!(pick_window(&windows, &[20, 10], "sem titulo"), Some(1));
        assert_eq!(pick_window(&windows, &[30], "sem titulo"), None);
    }

    #[test]
    fn windows_outside_the_chain_are_ignored_even_with_the_project_title() {
        let windows = vec![window(1, 999, "pier - Visual Studio Code"), window(2, 20, "outro - Visual Studio Code")];
        assert_eq!(pick_window(&windows, &[20], "pier"), Some(2));
    }

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
    fn old_status_files_default_to_claude() {
        let raw: RawStatus = serde_json::from_str(r#"{"sessionId":"a1","status":"running","cwd":"/x","project":"x","pidChain":[],"updatedAt":1}"#).unwrap();
        assert_eq!(raw.provider, "claude");
    }

    #[test]
    fn recognizes_only_its_own_hook_command() {
        let mine = json!({ "hooks": [{ "type": "command", "command": "node \"/home/x/.config/pier/pier-dock/pier-status.mjs\" --provider claude" }] });
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
        add_pier_hooks(&mut settings, HookFormat::Claude, "node \"/x/pier-status.mjs\" --provider claude").unwrap();
        // De novo: não deve duplicar a entrada do Pier nem mexer na do usuário.
        add_pier_hooks(&mut settings, HookFormat::Claude, "node \"/x/pier-status.mjs\" --provider claude").unwrap();

        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2, "{stop:?}");
        assert!(stop.iter().any(|e| !is_pier_entry(e)), "a entrada de outra ferramenta sumiu");
        assert_eq!(stop.iter().filter(|e| is_pier_entry(e)).count(), 1, "duplicou a entrada do Pier");
        assert_eq!(settings["otherSetting"], true);
        for event in CLAUDE_EVENTS {
            assert!(settings["hooks"][event].as_array().is_some(), "faltou hooks.{event}");
        }
    }

    #[test]
    fn reinstall_updates_the_command() {
        let mut settings = json!({});
        add_pier_hooks(&mut settings, HookFormat::Claude, "node \"/old/pier-status.mjs\"").unwrap();
        add_pier_hooks(&mut settings, HookFormat::Claude, "node \"/new/pier-status.mjs\" --provider claude").unwrap();
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
        assert!(stop[0]["hooks"][0]["command"].as_str().unwrap().contains("/new/"));
    }

    #[test]
    fn gemini_hooks_enable_the_feature_and_get_a_name() {
        let mut settings = json!({});
        add_pier_hooks(&mut settings, HookFormat::Gemini, "node \"/x/pier-status.mjs\" --provider gemini").unwrap();
        assert_eq!(settings["hooksConfig"]["enabled"], true);
        assert_eq!(settings["hooks"]["SessionStart"][0]["hooks"][0]["name"], "Pier status");
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
        remove_pier_hooks(&mut settings, HookFormat::Claude);
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
        assert!(!is_pier_entry(&stop[0]));
    }
}
