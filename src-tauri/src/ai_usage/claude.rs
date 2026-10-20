//! Uso do Claude Code: percentuais reais de limite + contagem de tokens.
//!
//! Duas fontes locais, com papéis diferentes:
//!
//! 1. `~/.claude.json`, campo `cachedUsageUtilization` — os percentuais REAIS da sessão de 5h e da
//!    semana. É a mesma fonte do comando `/usage`: o próprio Claude Code busca da API e guarda ali.
//!    Só é renovado enquanto ele está aberto, então a idade do dado importa (ver `STALE_MINUTES`).
//! 2. `~/.claude/projects/**/*.jsonl` — os transcripts, de onde saem as contagens de tokens. Eles
//!    não têm os limites (o `rateLimits` deles vem sempre nulo); tokens e percentual de limite são
//!    coisas separadas e não se convertem uma na outra.
//!
//! Nada sai para a rede. Do transcript são lidos apenas os campos numéricos de `usage`, o nome do
//! modelo e o timestamp — nunca o conteúdo das conversas. Do `.claude.json`, que também guarda
//! e-mail, uuids da conta e os caminhos de todos os projetos, só `cachedUsageUtilization` é tocado.

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use serde_json::Value;

use super::{active_block, now_ms, recent_files, today_totals, AiUsage, AiWindow, Event, BLOCK_HOURS, STALE_MINUTES};

/// A janela que o Claude Code chama de "sessão", em minutos.
const SESSION_MINUTES: i64 = BLOCK_HOURS * 60;

/// A janela semanal do `/usage`.
const WEEK_MINUTES: i64 = 7 * 24 * 60;

// --- caminhos ---------------------------------------------------------------

/// Pasta de dados do Claude Code: `$CLAUDE_CONFIG_DIR` ou `~/.claude`.
pub(crate) fn config_root() -> PathBuf {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::home_dir().unwrap_or_else(|| ".".into()).join(".claude"),
    }
}

/// O `.claude.json` fica ao lado da pasta `.claude`, mas acompanha o `CLAUDE_CONFIG_DIR` quando ele
/// existe. Devolve o primeiro candidato que existir, para não depender de qual deles o CLI criou.
fn claude_json() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| ".".into());
    let candidates = [config_root().join(".claude.json"), home.join(".claude.json")];
    candidates.iter().find(|p| p.is_file()).cloned().unwrap_or_else(|| candidates[1].clone())
}

fn projects_dir() -> PathBuf {
    config_root().join("projects")
}

/// Se o Claude Code já rodou nesta máquina.
pub fn detected() -> bool {
    claude_json().is_file() || config_root().is_dir()
}

// --- limites reais (~/.claude.json) -----------------------------------------

#[derive(Debug, Default, PartialEq)]
pub struct Limits {
    pub windows: Vec<AiWindow>,
    pub age_min: Option<i64>,
    pub stale: bool,
}

/// Minutos daqui até um timestamp ISO. Negativo se já passou.
fn minutes_until(iso: Option<&str>, now: DateTime<Utc>) -> Option<i64> {
    let when = DateTime::parse_from_rfc3339(iso?).ok()?;
    Some((when.with_timezone(&Utc) - now).num_minutes())
}

/// Converte o `cachedUsageUtilization` nas duas janelas do `/usage`.
///
/// `limits` é a lista que alimenta o `/usage`: o `kind` "session" é a janela de 5h, "weekly_all" é
/// a de 7 dias. A ordem é sempre sessão e depois semana, mesmo que só uma das duas exista.
pub fn parse_limits(cache: &Value, now: DateTime<Utc>) -> Limits {
    let mut out = Limits { stale: true, ..Default::default() };

    if let Some(fetched) = cache.get("fetchedAtMs").and_then(Value::as_f64) {
        let age = ((now.timestamp_millis() as f64 - fetched) / 60_000.0) as i64;
        out.age_min = Some(age.max(0));
        out.stale = age > STALE_MINUTES;
    }

    let window = |id: &str, minutes: i64, percent: Option<i64>, resets_in: Option<i64>| AiWindow {
        id: id.into(),
        minutes: Some(minutes),
        percent,
        resets_in,
    };
    let (mut session, mut weekly) = (None, None);
    for entry in cache.pointer("/utilization/limits").and_then(Value::as_array).into_iter().flatten() {
        let percent = entry.get("percent").and_then(Value::as_i64);
        let resets_in = minutes_until(entry.get("resets_at").and_then(Value::as_str), now);
        match entry.get("kind").and_then(Value::as_str) {
            Some("session") => session = Some(window("session", SESSION_MINUTES, percent, resets_in)),
            Some("weekly_all") => weekly = Some(window("weekly", WEEK_MINUTES, percent, resets_in)),
            _ => continue,
        }
    }
    out.windows = [session, weekly].into_iter().flatten().collect();

    // Se a janela já virou, o cache descreve um período que acabou. O percentual provavelmente
    // zerou, mas não dá para afirmar sem o Claude Code buscar de novo — então devolve desconhecido
    // em vez de um número errado.
    for window in &mut out.windows {
        if window.resets_in.is_some_and(|m| m <= 0) {
            window.percent = None;
        }
    }
    out
}

fn read_limits(now: DateTime<Utc>) -> Limits {
    let unknown = || Limits { stale: true, ..Default::default() };
    let Ok(text) = std::fs::read_to_string(claude_json()) else { return unknown() };
    let Ok(json) = serde_json::from_str::<Value>(&text) else { return unknown() };
    json.get("cachedUsageUtilization").map_or_else(unknown, |cache| parse_limits(cache, now))
}

// --- tokens (transcripts) ---------------------------------------------------

/// Converte uma linha do transcript num evento, com a chave de deduplicação.
///
/// Os transcripts repetem a mesma mensagem várias vezes, uma por iteração do stream, então sem
/// deduplicar por `(message.id, requestId)` os totais saem inflados em 2-3x.
pub fn parse_event(line: &str) -> Option<((String, String), Event)> {
    let record: Value = serde_json::from_str(line).ok()?;
    if record.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    let usage = record.pointer("/message/usage")?;
    let timestamp = record.get("timestamp").and_then(Value::as_str)?;
    // Os timestamps vêm em UTC; converte para o fuso local, senão o contador de "hoje" viraria o
    // dia às 21h.
    let when = DateTime::parse_from_rfc3339(timestamp).ok()?.with_timezone(&Local);

    let field = |name| usage.get(name).and_then(Value::as_i64).unwrap_or(0);
    let tokens = field("input_tokens")
        + field("output_tokens")
        + field("cache_creation_input_tokens")
        + field("cache_read_input_tokens");

    let text = |pointer| record.pointer(pointer).and_then(Value::as_str).unwrap_or_default().to_string();
    let key = (text("/message/id"), text("/requestId"));
    let model = record.pointer("/message/model").and_then(Value::as_str).map(String::from);
    Some((key, Event { when, tokens, model }))
}

/// Eventos dos transcripts recentes, deduplicados e ordenados por tempo.
fn collect_events() -> Vec<Event> {
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    for path in recent_files(projects_dir()) {
        let Ok(file) = std::fs::File::open(&path) else { continue };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            // Filtro barato antes de pagar o custo do parse de JSON.
            if !line.contains("\"usage\"") {
                continue;
            }
            if let Some((key, event)) = parse_event(&line) {
                if seen.insert(key) {
                    events.push(event);
                }
            }
        }
    }
    events.sort_by_key(|e| e.when);
    events
}

/// Snapshot completo do Claude Code: limites reais + tokens.
pub fn snapshot() -> AiUsage {
    let now = Local::now();
    let limits = read_limits(now.with_timezone(&Utc));
    let events = collect_events();

    let mut usage = AiUsage {
        provider: "claude".into(),
        windows: limits.windows,
        age_min: limits.age_min,
        stale: limits.stale,
        block_tokens: active_block(&events, now),
        fetched_at: now_ms(),
        ..Default::default()
    };
    (usage.today_tokens, usage.today_messages, usage.top_model) = today_totals(&events, now.date_naive());
    usage
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 16, 14, 0, 0).unwrap()
    }

    fn window(id: &str, minutes: i64, percent: Option<i64>, resets_in: Option<i64>) -> AiWindow {
        AiWindow { id: id.into(), minutes: Some(minutes), percent, resets_in }
    }

    #[test]
    fn parses_limits_cache() {
        let cache = json!({
            "fetchedAtMs": (now().timestamp_millis() - 5 * 60_000) as f64,
            "utilization": { "limits": [
                { "kind": "session", "percent": 42, "resets_at": "2026-10-16T17:49:00+00:00" },
                { "kind": "weekly_all", "percent": 78, "resets_at": "2026-10-18T22:00:00+00:00" },
                { "kind": "outro_qualquer", "percent": 99, "resets_at": null }
            ]}
        });
        let limits = parse_limits(&cache, now());
        assert_eq!(limits.windows, vec![
            window("session", 300, Some(42), Some(229)),
            window("weekly", 10_080, Some(78), Some(2 * 24 * 60 + 8 * 60)),
        ]);
        assert_eq!(limits.age_min, Some(5));
        assert!(!limits.stale);
    }

    #[test]
    fn expired_window_is_unknown_not_zero() {
        let cache = json!({
            "utilization": { "limits": [
                { "kind": "session", "percent": 42, "resets_at": "2026-10-16T13:00:00+00:00" }
            ]}
        });
        let limits = parse_limits(&cache, now());
        assert_eq!(limits.windows, vec![window("session", 300, None, Some(-60))]);
        // Sem `fetchedAtMs` não há como saber a idade, então o dado conta como velho.
        assert_eq!(limits.age_min, None);
        assert!(limits.stale);
    }

    #[test]
    fn old_cache_is_stale() {
        let cache = json!({ "fetchedAtMs": (now().timestamp_millis() - 40 * 60_000) as f64 });
        let limits = parse_limits(&cache, now());
        assert_eq!(limits.age_min, Some(40));
        assert!(limits.stale);
    }

    #[test]
    fn missing_cache_is_unknown() {
        let limits = parse_limits(&json!({}), now());
        assert_eq!(limits, Limits { stale: true, ..Default::default() });
        assert!(limits.windows.is_empty());
    }

    #[test]
    fn parses_assistant_usage_line() {
        let line = json!({
            "type": "assistant",
            "timestamp": "2026-10-16T14:00:25.805Z",
            "requestId": "req_1",
            "message": { "id": "msg_1", "model": "claude-opus-5", "usage": {
                "input_tokens": 2, "output_tokens": 2537,
                "cache_creation_input_tokens": 53677, "cache_read_input_tokens": 100
            }}
        })
        .to_string();
        let (key, event) = parse_event(&line).unwrap();
        assert_eq!(key, ("msg_1".into(), "req_1".into()));
        assert_eq!(event.tokens, 2 + 2537 + 53677 + 100);
        assert_eq!(event.model.as_deref(), Some("claude-opus-5"));
    }

    #[test]
    fn ignores_lines_that_are_not_usage() {
        let user = json!({ "type": "user", "timestamp": "2026-10-16T14:00:25.805Z", "message": { "usage": {} } });
        assert!(parse_event(&user.to_string()).is_none());
        let no_usage = json!({ "type": "assistant", "timestamp": "2026-10-16T14:00:25.805Z", "message": {} });
        assert!(parse_event(&no_usage.to_string()).is_none());
        let no_time = json!({ "type": "assistant", "message": { "usage": { "input_tokens": 1 } } });
        assert!(parse_event(&no_time.to_string()).is_none());
        assert!(parse_event("{lixo").is_none());
    }
}

#[cfg(test)]
mod live_tests {
    /// Lê o uso real desta máquina. Os números têm de bater com o `/usage` do Claude Code e com o
    /// `python usage.py` do projeto claudeUsage, que é a implementação de referência desta leitura.
    #[test]
    #[ignore = "smoke test: lê os arquivos do Claude Code na máquina"]
    fn reads_local_usage() {
        let usage = super::snapshot();
        println!("janelas: {:?}", usage.windows);
        println!("idade do cache: {:?} min (velho: {})", usage.age_min, usage.stale);
        println!("tokens hoje: {} em {} mensagens", usage.today_tokens, usage.today_messages);
        println!("bloco de 5h: {}", usage.block_tokens);
        println!("modelo dominante: {:?}", usage.top_model);
    }
}
