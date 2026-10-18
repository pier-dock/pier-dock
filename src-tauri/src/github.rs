//! Widgets do GitHub. Todas as requisições saem daqui: o token nunca chega ao WebView.
//!
//! O token vem de um PAT salvo no cofre de senhas do sistema (Credential Manager, Keychain ou
//! Secret Service), se o usuário configurou um, ou do `gh auth token`.

use std::sync::Mutex;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::util::ErrStr;

const KEYRING_SERVICE: &str = "pier";
const KEYRING_USER: &str = "github";

const QUERY: &str = r#"
query {
  viewer {
    login
    avatarUrl
    contributionsCollection {
      contributionCalendar {
        totalContributions
        weeks { contributionDays { date contributionCount } }
      }
    }
  }
  review: search(type: ISSUE, query: "is:open is:pr review-requested:@me archived:false sort:updated-desc", first: 20) {
    issueCount
    nodes { ...pr }
  }
  mine: search(type: ISSUE, query: "is:open is:pr author:@me archived:false sort:updated-desc", first: 20) {
    issueCount
    nodes { ...pr }
  }
}
fragment pr on PullRequest {
  number
  title
  url
  isDraft
  updatedAt
  reviewDecision
  repository { nameWithOwner }
  author { login avatarUrl }
  commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }
}
"#;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub number: i64,
    pub title: String,
    pub url: String,
    pub repo: String,
    pub author: String,
    pub author_avatar: String,
    pub is_draft: bool,
    pub updated_at: String,
    /// SUCCESS, FAILURE, ERROR, PENDING, EXPECTED ou None (sem checks).
    pub checks: Option<String>,
    pub review_decision: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub repo: String,
    pub url: String,
    pub date: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContributionDay {
    pub date: String,
    pub count: i64,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GithubData {
    pub login: String,
    pub avatar_url: String,
    pub review_count: i64,
    pub review_requests: Vec<PullRequest>,
    pub my_pr_count: i64,
    pub my_prs: Vec<PullRequest>,
    pub commits: Vec<Commit>,
    pub total_contributions: i64,
    pub weeks: Vec<Vec<ContributionDay>>,
    pub fetched_at: f64,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    /// "pat", "gh" ou None.
    pub source: Option<String>,
}

static CACHE: Mutex<Option<GithubData>> = Mutex::new(None);

fn stored_token() -> Option<String> {
    let read = |service: &str| keyring::Entry::new(service, KEYRING_USER).ok()?.get_password().ok().filter(|t| !t.is_empty());
    read(KEYRING_SERVICE).or_else(migrate_legacy_token)
}

/// O token salvo pelo WinDock vai para o serviço novo do cofre e sai do antigo.
fn migrate_legacy_token() -> Option<String> {
    let legacy = keyring::Entry::new(crate::util::LEGACY_NAME, KEYRING_USER).ok()?;
    let token = legacy.get_password().ok().filter(|t| !t.is_empty())?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?.set_password(&token).ok()?;
    let _ = legacy.delete_credential();
    Some(token)
}

/// Apps abertos pelo Finder/Dock não herdam o PATH do shell, então o `gh` do Homebrew não seria
/// achado só pelo nome.
fn gh_program() -> &'static str {
    #[cfg(unix)]
    for candidate in ["/opt/homebrew/bin/gh", "/usr/local/bin/gh", "/usr/bin/gh"] {
        if std::path::Path::new(candidate).exists() {
            return candidate;
        }
    }
    "gh"
}

fn gh_token() -> Option<String> {
    let mut command = std::process::Command::new(gh_program());
    command.args(["auth", "token"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

pub(crate) fn token() -> Option<(String, &'static str)> {
    stored_token().map(|t| (t, "pat")).or_else(|| gh_token().map(|t| (t, "gh")))
}

fn str_at(v: &Value, pointer: &str) -> String {
    v.pointer(pointer).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn parse_pr(node: &Value) -> Option<PullRequest> {
    let number = node.get("number")?.as_i64()?;
    Some(PullRequest {
        number,
        title: str_at(node, "/title"),
        url: str_at(node, "/url"),
        repo: str_at(node, "/repository/nameWithOwner"),
        author: str_at(node, "/author/login"),
        author_avatar: str_at(node, "/author/avatarUrl"),
        is_draft: node.get("isDraft").and_then(Value::as_bool).unwrap_or(false),
        updated_at: str_at(node, "/updatedAt"),
        checks: node
            .pointer("/commits/nodes/0/commit/statusCheckRollup/state")
            .and_then(Value::as_str)
            .map(String::from),
        review_decision: node.get("reviewDecision").and_then(Value::as_str).map(String::from),
    })
}

fn parse_prs(search: Option<&Value>) -> (i64, Vec<PullRequest>) {
    let Some(search) = search else { return (0, vec![]) };
    let count = search.get("issueCount").and_then(Value::as_i64).unwrap_or(0);
    let prs = search
        .get("nodes")
        .and_then(Value::as_array)
        .map(|nodes| nodes.iter().filter_map(parse_pr).collect())
        .unwrap_or_default();
    (count, prs)
}

/// Converte o `data` da resposta GraphQL (sem os commits, que vêm da API REST).
pub fn parse_graphql(data: &Value) -> GithubData {
    let (review_count, review_requests) = parse_prs(data.get("review"));
    let (my_pr_count, my_prs) = parse_prs(data.get("mine"));
    let calendar = data.pointer("/viewer/contributionsCollection/contributionCalendar");
    let weeks = calendar
        .and_then(|c| c.get("weeks"))
        .and_then(Value::as_array)
        .map(|weeks| {
            weeks
                .iter()
                .map(|w| {
                    w.get("contributionDays")
                        .and_then(Value::as_array)
                        .map(|days| {
                            days.iter()
                                .map(|d| ContributionDay {
                                    date: str_at(d, "/date"),
                                    count: d.get("contributionCount").and_then(Value::as_i64).unwrap_or(0),
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    GithubData {
        login: str_at(data, "/viewer/login"),
        avatar_url: str_at(data, "/viewer/avatarUrl"),
        review_count,
        review_requests,
        my_pr_count,
        my_prs,
        total_contributions: calendar
            .and_then(|c| c.get("totalContributions"))
            .and_then(Value::as_i64)
            .unwrap_or(0),
        weeks,
        ..Default::default()
    }
}

pub fn parse_commits(search: &Value) -> Vec<Commit> {
    search
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| Commit {
                    sha: str_at(item, "/sha"),
                    message: str_at(item, "/commit/message").lines().next().unwrap_or_default().to_string(),
                    repo: str_at(item, "/repository/full_name"),
                    url: str_at(item, "/html_url"),
                    date: str_at(item, "/commit/author/date"),
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn fetch(token: &str) -> Result<GithubData, String> {
    let client = reqwest::Client::builder().user_agent("pier").build().err_str()?;

    let response: Value = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .json(&json!({ "query": QUERY }))
        .send()
        .await
        .err_str()?
        .error_for_status()
        .err_str()?
        .json()
        .await
        .err_str()?;
    if let Some(message) = response.pointer("/errors/0/message").and_then(Value::as_str) {
        if response.get("data").map_or(true, Value::is_null) {
            return Err(message.to_string());
        }
    }
    let mut data = parse_graphql(response.get("data").unwrap_or(&Value::Null));

    // A busca de commits só cobre a branch padrão de cada repo, mas inclui repos privados aos quais o
    // token tem acesso e já traz a mensagem — o feed de eventos não traz mais os commits.
    let commits = client
        .get("https://api.github.com/search/commits")
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .query(&[
            ("q", format!("author:{}", data.login)),
            ("sort", "author-date".into()),
            ("order", "desc".into()),
            ("per_page", "20".into()),
        ])
        .send()
        .await;
    if let Ok(resp) = commits {
        if let Ok(json) = resp.json::<Value>().await {
            data.commits = parse_commits(&json);
        }
    }
    Ok(data)
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

#[tauri::command]
pub async fn github_fetch(app: AppHandle) -> Result<GithubData, String> {
    let token = tauri::async_runtime::spawn_blocking(token).await.err_str()?;
    let mut data = match token {
        None => GithubData {
            error: Some(
                crate::i18n::current(&app)
                    .t(
                        "GitHub não conectado: rode `gh auth login` ou cole um token nas configurações.",
                        "GitHub not connected: run `gh auth login` or paste a token in settings.",
                    )
                    .into(),
            ),
            ..Default::default()
        },
        Some((token, _)) => match fetch(&token).await {
            Ok(data) => data,
            // Mantém os últimos dados bons na tela e só anexa o erro.
            Err(e) => GithubData { error: Some(e), ..CACHE.lock().unwrap().clone().unwrap_or_default() },
        },
    };
    data.fetched_at = now_ms();
    *CACHE.lock().unwrap() = Some(data.clone());
    let _ = app.emit("github-updated", &data);
    Ok(data)
}

#[tauri::command]
pub fn github_cached() -> Option<GithubData> {
    CACHE.lock().unwrap().clone()
}

#[tauri::command]
pub async fn github_auth_status() -> Result<AuthStatus, String> {
    tauri::async_runtime::spawn_blocking(|| AuthStatus { source: token().map(|(_, source)| source.to_string()) })
        .await
        .err_str()
}

/// Salva um PAT no cofre de senhas do sistema; `None` ou vazio apaga e volta a usar o `gh`.
#[tauri::command]
pub fn github_set_token(token: Option<String>) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).err_str()?;
    match token.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(t) => entry.set_password(&t).err_str(),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_graphql_response() {
        let data = json!({
            "viewer": {
                "login": "octocat",
                "avatarUrl": "https://avatars/octocat",
                "contributionsCollection": { "contributionCalendar": {
                    "totalContributions": 42,
                    "weeks": [{ "contributionDays": [
                        { "date": "2026-10-11", "contributionCount": 0 },
                        { "date": "2026-10-12", "contributionCount": 5 }
                    ]}]
                }}
            },
            "review": { "issueCount": 1, "nodes": [{
                "number": 7, "title": "Fix bug", "url": "https://github.com/a/b/pull/7", "isDraft": false,
                "updatedAt": "2026-10-14T10:00:00Z", "reviewDecision": "REVIEW_REQUIRED",
                "repository": { "nameWithOwner": "a/b" },
                "author": { "login": "dev", "avatarUrl": "x" },
                "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "state": "FAILURE" } } }] }
            }]},
            "mine": { "issueCount": 0, "nodes": [{}] }
        });
        let parsed = parse_graphql(&data);
        assert_eq!(parsed.login, "octocat");
        assert_eq!(parsed.total_contributions, 42);
        assert_eq!(parsed.weeks[0][1].count, 5);
        assert_eq!(parsed.review_count, 1);
        assert_eq!(parsed.review_requests[0].repo, "a/b");
        assert_eq!(parsed.review_requests[0].checks.as_deref(), Some("FAILURE"));
        // Nós vazios (ex.: issues que não são PR) são ignorados.
        assert!(parsed.my_prs.is_empty());
    }

    #[test]
    fn parses_commit_search() {
        let json = json!({ "items": [{
            "sha": "abc123",
            "html_url": "https://github.com/a/b/commit/abc123",
            "commit": { "message": "feat: dock\n\ncorpo", "author": { "date": "2026-10-15T09:00:00Z" } },
            "repository": { "full_name": "a/b" }
        }]});
        let commits = parse_commits(&json);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "feat: dock");
        assert_eq!(commits[0].repo, "a/b");
    }
}

#[cfg(test)]
mod live_tests {
    #[test]
    #[ignore = "smoke test: chama a API real do GitHub com o token do gh"]
    fn fetches_from_github() {
        let (token, source) = super::token().expect("sem token");
        let rt = tauri::async_runtime::block_on(super::fetch(&token)).unwrap();
        println!(
            "fonte={source} login={} reviews={} meus={} commits={} contribuições={} semanas={}",
            rt.login,
            rt.review_count,
            rt.my_pr_count,
            rt.commits.len(),
            rt.total_contributions,
            rt.weeks.len()
        );
        for c in rt.commits.iter().take(3) {
            println!("  {} {} {}", c.repo, &c.sha[..7], c.message);
        }
        assert!(!rt.login.is_empty());
    }
}
