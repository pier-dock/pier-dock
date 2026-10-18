//! "Reportar um problema" e "Sugerir um recurso" nas configurações. A issue é criada pela API com a
//! conta do GitHub conectada ao Pier; sem conta (ou se a API recusar), a página `issues/new` abre no
//! navegador já preenchida pelo template do repositório.

use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::util::ErrStr;

const REPO: &str = "pier-dock/pier-dock";
const LOG_NAME: &str = "pier";
const RECENT_LINES: usize = 200;
/// O GitHub recusa URLs muito longas; no fallback os logs perdem as linhas mais antigas até caber.
const MAX_URL: usize = 8000;

pub fn log_plugin() -> TauriPlugin<Wry> {
    tauri_plugin_log::Builder::new()
        .targets([Target::new(TargetKind::Stdout), Target::new(TargetKind::LogDir { file_name: Some(LOG_NAME.into()) })])
        .level(log::LevelFilter::Info)
        .max_file_size(1_000_000)
        // Guarda o arquivo anterior: com KeepOne o log some inteiro na hora de girar.
        .rotation_strategy(RotationStrategy::KeepSome(2))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Bug,
    Feature,
}

impl Kind {
    fn parse(kind: &str) -> Result<Self, String> {
        match kind {
            "bug" => Ok(Kind::Bug),
            "feature" => Ok(Kind::Feature),
            _ => Err(format!("tipo de relato desconhecido: {kind}")),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Bug => "bug",
            Kind::Feature => "enhancement",
        }
    }

    fn template(self) -> &'static str {
        match self {
            Kind::Bug => "bug_report.yml",
            Kind::Feature => "feature_request.yml",
        }
    }

    /// Só quem tem push no repositório pode definir labels ao criar uma issue (a API descarta as
    /// outras sem erro). A action `label-reports.yml` lê esta marca e põe a label.
    fn marker(self) -> String {
        format!("<!-- pier-report: {} -->", self.label())
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub version: String,
    pub os: String,
    pub arch: String,
    /// Só no Linux: área de trabalho, compositor e tipo de sessão.
    pub desktop: Option<String>,
}

impl Environment {
    fn markdown(&self) -> String {
        let mut lines = vec![format!("- Pier: {}", self.version), format!("- OS: {} ({})", self.os, self.arch)];
        if let Some(desktop) = &self.desktop {
            lines.push(format!("- Desktop: {desktop}"));
        }
        lines.join("\n")
    }
}

fn environment(app: &AppHandle) -> Environment {
    Environment {
        version: app.package_info().version.to_string(),
        os: sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
        arch: std::env::consts::ARCH.into(),
        desktop: desktop(),
    }
}

#[cfg(target_os = "linux")]
fn desktop() -> Option<String> {
    let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    let current = var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let has = |name: &str| current.split(':').any(|d| d.eq_ignore_ascii_case(name));
    let version = if has("GNOME") {
        crate::util::run("gnome-shell", &["--version"])
    } else if has("KDE") {
        crate::util::run("plasmashell", &["--version"])
    } else {
        None
    };
    Some(describe_desktop(&var, version.map(|v| v.trim().to_string())))
}

#[cfg(not(target_os = "linux"))]
fn desktop() -> Option<String> {
    None
}

/// "ubuntu:GNOME · GNOME Shell 46.0 · Mutter · Wayland". O tipo de sessão vem do ambiente, não do
/// GDK: o Pier força `GDK_BACKEND=x11` no Wayland (ver `prepare_env`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn describe_desktop(var: &dyn Fn(&str) -> Option<String>, version: Option<String>) -> String {
    let current = var("XDG_CURRENT_DESKTOP").unwrap_or_else(|| "unknown".into());
    let session = match var("XDG_SESSION_TYPE").as_deref() {
        Some("wayland") => "Wayland".to_string(),
        Some("x11") => "X11".to_string(),
        Some(other) => other.to_string(),
        None if var("WAYLAND_DISPLAY").is_some() => "Wayland".into(),
        None if var("DISPLAY").is_some() => "X11".into(),
        None => "unknown session".into(),
    };
    let mut parts = vec![current.clone()];
    parts.extend(version);
    parts.extend(compositor(&current, var).map(String::from));
    parts.push(session);
    parts.join(" · ")
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn compositor(current: &str, var: &dyn Fn(&str) -> Option<String>) -> Option<&'static str> {
    for (env, name) in [("HYPRLAND_INSTANCE_SIGNATURE", "Hyprland"), ("SWAYSOCK", "Sway"), ("NIRI_SOCKET", "niri")] {
        if var(env).is_some() {
            return Some(name);
        }
    }
    let known = [("KDE", "KWin"), ("GNOME", "Mutter"), ("X-Cinnamon", "Muffin"), ("MATE", "Marco"), ("XFCE", "xfwm4"), ("COSMIC", "cosmic-comp")];
    known
        .into_iter()
        .find(|(desktop, _)| current.split(':').any(|d| d.eq_ignore_ascii_case(desktop)))
        .map(|(_, name)| name)
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Os logs podem ter caminhos com o nome do usuário; a pessoa ainda revisa tudo antes de enviar.
fn scrub_home(text: &str, home: Option<std::path::PathBuf>) -> String {
    match home.map(|h| h.display().to_string()).filter(|h| h.len() > 1) {
        Some(home) => text.replace(&home, "~"),
        None => text.to_string(),
    }
}

/// Um bloco de código que não fecha antes da hora se os logs tiverem crases.
fn code_block(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}shell\n{}\n{fence}", text.trim_end())
}

/// Mesmos títulos que o GitHub usa ao renderizar os campos dos templates.
fn body(kind: Kind, description: &str, env: Option<&Environment>, logs: Option<&str>) -> String {
    let mut out = format!("### Description\n\n{}\n", description.trim());
    if let Some(env) = env {
        out += &format!("\n### Environment\n\n{}\n", env.markdown());
    }
    if let Some(logs) = logs {
        out += &format!("\n### Logs\n\n{}\n", code_block(logs));
    }
    out += &format!("\n{}\n", kind.marker());
    out
}

/// Tamanho de `text` depois de codificado como em `application/x-www-form-urlencoded`.
fn encoded_len(text: &str) -> usize {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || b" -._*".contains(&b) { 1 } else { 3 }).sum()
}

fn new_issue_url(kind: Kind, title: &str, description: &str, env: Option<&Environment>, logs: Option<&str>) -> String {
    let mut url = reqwest::Url::parse(&format!("https://github.com/{REPO}/issues/new")).expect("URL fixa");
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("template", kind.template()).append_pair("title", title).append_pair("description", description.trim());
        if let Some(env) = env {
            query.append_pair("environment", &env.markdown());
        }
    }
    if let Some(logs) = logs {
        let room = MAX_URL.saturating_sub(url.as_str().len() + "&logs=".len());
        let mut lines: Vec<&str> = logs.lines().collect();
        while !lines.is_empty() && encoded_len(&lines.join("\n")) > room {
            lines.remove(0);
        }
        if !lines.is_empty() {
            url.query_pairs_mut().append_pair("logs", &lines.join("\n"));
        }
    }
    url.into()
}

async fn create(token: &str, kind: Kind, title: &str, body: &str) -> Result<String, String> {
    let client = reqwest::Client::builder().user_agent("pier").timeout(Duration::from_secs(15)).build().err_str()?;
    let response = client
        .post(format!("https://api.github.com/repos/{REPO}/issues"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .json(&json!({ "title": title, "body": body, "labels": [kind.label()] }))
        .send()
        .await
        .err_str()?;
    let status = response.status();
    let json: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let message = json.get("message").and_then(Value::as_str).unwrap_or_default();
        return Err(format!("{} {message}", status.as_u16()).trim().to_string());
    }
    json.get("html_url").and_then(Value::as_str).map(String::from).ok_or_else(|| "resposta do GitHub sem html_url".into())
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ReportResult {
    Created { url: String },
    Fallback { url: String, reason: Option<String> },
}

#[tauri::command]
pub fn report_environment(app: AppHandle) -> Environment {
    environment(&app)
}

#[tauri::command]
pub fn recent_logs(app: AppHandle) -> Result<String, String> {
    let path = app.path().app_log_dir().err_str()?.join(format!("{LOG_NAME}.log"));
    let text = std::fs::read(&path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default();
    Ok(scrub_home(&tail(&text, RECENT_LINES), dirs::home_dir()))
}

#[tauri::command]
pub async fn report_issue(app: AppHandle, kind: String, title: String, description: String, logs: Option<String>) -> Result<ReportResult, String> {
    let kind = Kind::parse(&kind)?;
    let title = title.trim();
    if title.is_empty() {
        return Err("o título está vazio".into());
    }
    let env = (kind == Kind::Bug).then(|| environment(&app));
    let logs = logs.as_deref().filter(|l| !l.trim().is_empty());
    // `token()` pode rodar o `gh auth token`.
    let token = tauri::async_runtime::spawn_blocking(crate::github::token).await.err_str()?;
    let mut reason = None;
    if let Some((token, _)) = token {
        match create(&token, kind, title, &body(kind, &description, env.as_ref(), logs)).await {
            Ok(url) => return Ok(ReportResult::Created { url }),
            Err(e) => {
                log::warn!("não deu para criar a issue pela API: {e}");
                reason = Some(e);
            }
        }
    }
    Ok(ReportResult::Fallback { url: new_issue_url(kind, title, &description, env.as_ref(), logs), reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
    }

    fn sample_env() -> Environment {
        Environment { version: "0.6.1".into(), os: "Linux (Ubuntu 24.04)".into(), arch: "x86_64".into(), desktop: Some("GNOME · X11".into()) }
    }

    #[test]
    fn describes_linux_desktops() {
        let gnome = env_of(&[("XDG_CURRENT_DESKTOP", "ubuntu:GNOME"), ("XDG_SESSION_TYPE", "wayland")]);
        assert_eq!(describe_desktop(&gnome, Some("GNOME Shell 46.0".into())), "ubuntu:GNOME · GNOME Shell 46.0 · Mutter · Wayland");
        let kde = env_of(&[("XDG_CURRENT_DESKTOP", "KDE"), ("XDG_SESSION_TYPE", "x11")]);
        assert_eq!(describe_desktop(&kde, None), "KDE · KWin · X11");
        let hyprland = env_of(&[("XDG_CURRENT_DESKTOP", "Hyprland"), ("HYPRLAND_INSTANCE_SIGNATURE", "abc"), ("WAYLAND_DISPLAY", "wayland-1")]);
        assert_eq!(describe_desktop(&hyprland, None), "Hyprland · Hyprland · Wayland");
        let bare = env_of(&[("DISPLAY", ":0")]);
        assert_eq!(describe_desktop(&bare, None), "unknown · X11");
    }

    #[test]
    fn body_has_template_sections_and_marker() {
        let body = body(Kind::Bug, " It crashed \n", Some(&sample_env()), Some("line with ``` inside"));
        assert!(body.starts_with("### Description\n\nIt crashed\n"));
        assert!(body.contains("### Environment\n\n- Pier: 0.6.1\n- OS: Linux (Ubuntu 24.04) (x86_64)\n- Desktop: GNOME · X11"));
        assert!(body.contains("````shell\nline with ``` inside\n````"));
        assert!(body.trim_end().ends_with("<!-- pier-report: bug -->"));
        let feature = super::body(Kind::Feature, "Idea", None, None);
        assert!(!feature.contains("### Environment") && !feature.contains("### Logs"));
        assert!(feature.contains("<!-- pier-report: enhancement -->"));
    }

    #[test]
    fn fallback_url_fits_and_keeps_newest_logs() {
        let logs: Vec<String> = (0..2000).map(|i| format!("[INFO] line {i} with some text")).collect();
        let url = new_issue_url(Kind::Bug, "Dock froze", "Details", Some(&sample_env()), Some(&logs.join("\n")));
        assert!(url.len() <= MAX_URL, "{}", url.len());
        assert!(url.starts_with("https://github.com/pier-dock/pier-dock/issues/new?template=bug_report.yml&title=Dock+froze"));
        assert!(url.contains("line+1999+with"));
        assert!(!url.contains("line+0+with"));
        let short = new_issue_url(Kind::Feature, "Idea", "Text", None, None);
        assert_eq!(short, "https://github.com/pier-dock/pier-dock/issues/new?template=feature_request.yml&title=Idea&description=Text");
    }

    #[test]
    fn logs_tail_and_scrub() {
        assert_eq!(tail("a\nb\nc", 2), "b\nc");
        assert_eq!(tail("a", 5), "a");
        let home = Some(std::path::PathBuf::from("/home/ana"));
        assert_eq!(scrub_home("open /home/ana/.config/pier/config.json", home), "open ~/.config/pier/config.json");
    }

    #[test]
    fn parses_kind() {
        assert_eq!(Kind::parse("bug"), Ok(Kind::Bug));
        assert_eq!(Kind::parse("feature").map(Kind::label), Ok("enhancement"));
        assert!(Kind::parse("other").is_err());
    }
}
