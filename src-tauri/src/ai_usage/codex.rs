//! Uso do Codex CLI (OpenAI): percentuais reais de limite + contagem de tokens.
//!
//! Fonte única: `~/.codex/sessions/**/rollout-*.jsonl` (ou `$CODEX_HOME/sessions`), um arquivo por
//! sessão e por dia. Dele interessam duas linhas:
//!
//! 1. `turn_context` — o modelo do turno (`gpt-5.6-luna`).
//! 2. `event_msg` com `payload.type == "token_count"` — o `payload.info.last_token_usage.total_tokens`
//!    do turno, o `rate_limits` da conta e o timestamp. O `total_token_usage` é o acumulado da
//!    sessão: somá-lo infla os totais em ordens de grandeza, então ele é ignorado.
//!
//! O `rate_limits` é a mesma fonte do `/status` do CLI: `primary` e `secondary` com `used_percent`,
//! `window_minutes` e `resets_at` (epoch em segundos). Ele só é renovado enquanto o Codex roda —
//! cada turno regrava —, então vale o mais recente e a idade dele importa (ver `STALE_MINUTES`).
//! Conta por chave de API não tem limites de plano e vem com `rate_limits` nulo; aí o widget mostra
//! só os tokens.
//!
//! Nada sai para a rede: dos rollouts são lidos apenas os campos numéricos de `usage`, o `model` e
//! o timestamp — nunca o conteúdo das conversas, os comandos executados ou os diffs.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use serde_json::Value;

use super::{active_block, now_ms, recent_files, today_totals, AiUsage, AiWindow, Event, STALE_MINUTES};

// --- caminhos ---------------------------------------------------------------

/// Pasta de dados do Codex CLI: `$CODEX_HOME` ou `~/.codex`.
fn config_root() -> PathBuf {
    match std::env::var_os("CODEX_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => dirs::home_dir().unwrap_or_else(|| ".".into()).join(".codex"),
    }
}

fn sessions_dir() -> PathBuf {
    config_root().join("sessions")
}

/// Se o Codex CLI já rodou nesta máquina.
pub fn detected() -> bool {
    sessions_dir().is_dir() || config_root().is_dir()
}

// --- limites (rate_limits) --------------------------------------------------

#[derive(Debug, Default, PartialEq)]
pub struct Limits {
    pub windows: Vec<AiWindow>,
    pub seen_at: Option<DateTime<Utc>>,
}

/// Converte o `rate_limits` de um evento nas janelas da conta.
///
/// As duas janelas vêm nomeadas por papel, não por duração: no plano Plus a `primary` costuma ser a
/// semana (`window_minutes` 10080) e no Business a de 5h (300). O rótulo do widget sai da duração.
pub fn parse_limits(rate_limits: &Value, now: DateTime<Utc>) -> Vec<AiWindow> {
    ["primary", "secondary"]
        .iter()
        .filter_map(|id| {
            let entry = rate_limits.get(id)?;
            if entry.is_null() {
                return None;
            }
            let minutes = entry.get("window_minutes").and_then(Value::as_i64);
            let percent = entry.get("used_percent").and_then(Value::as_f64).map(|p| p.round() as i64);
            let resets_in = entry.get("resets_at").and_then(Value::as_i64).map(|at| (at - now.timestamp()).div_euclid(60));
            // Janela que já virou: o percentual descreve um período que acabou, então vale
            // desconhecido em vez de um número parado.
            let expired = resets_in.is_some_and(|m| m <= 0);
            Some(AiWindow { id: (*id).into(), minutes, percent: if expired { None } else { percent }, resets_in })
        })
        .collect()
}

// --- eventos (token_count) --------------------------------------------------

/// Converte uma linha de rollout no evento de tokens do turno, com os limites quando eles vêm nela.
pub fn parse_turn(line: &str) -> Option<(Event, Option<Value>)> {
    let record: Value = serde_json::from_str(line).ok()?;
    let payload = record.get("payload")?;
    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return None;
    }
    let timestamp = record.get("timestamp").and_then(Value::as_str)?;
    // Os timestamps vêm em UTC; converte para o fuso local, senão o contador de "hoje" viraria o
    // dia às 21h.
    let when = DateTime::parse_from_rfc3339(timestamp).ok()?.with_timezone(&Local);
    let tokens = payload.pointer("/info/last_token_usage/total_tokens").and_then(Value::as_i64)?;
    let limits = payload.get("rate_limits").filter(|v| !v.is_null()).cloned();
    Some((Event { when, tokens, model: None }, limits))
}

/// O modelo do turno, das linhas `turn_context`.
pub fn parse_model(line: &str) -> Option<String> {
    let record: Value = serde_json::from_str(line).ok()?;
    record.pointer("/payload/model").and_then(Value::as_str).map(String::from)
}

/// Eventos e limites dos arquivos dados. Cada arquivo tem o modelo do seu último `turn_context`,
/// que é o que os turnos seguintes usaram; os limites ficam com o `rate_limits` mais recente.
fn scan(files: Vec<PathBuf>, now: DateTime<Utc>) -> (Vec<Event>, Limits) {
    let mut events = Vec::new();
    let mut limits = Limits::default();
    for path in files {
        let Ok(file) = std::fs::File::open(&path) else { continue };
        let mut model: Option<String> = None;
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            // Filtro barato antes de pagar o custo do parse de JSON.
            if line.contains("\"model\"") {
                if let Some(found) = parse_model(&line) {
                    model = Some(found);
                }
            }
            if !line.contains("\"token_count\"") {
                continue;
            }
            let Some((mut event, rate_limits)) = parse_turn(&line) else { continue };
            event.model = model.clone();
            let seen_at = event.when.with_timezone(&Utc);
            if let Some(rate_limits) = rate_limits {
                if limits.seen_at.map_or(true, |at| seen_at > at) {
                    limits = Limits { windows: parse_limits(&rate_limits, now), seen_at: Some(seen_at) };
                }
            }
            events.push(event);
        }
    }
    events.sort_by_key(|e| e.when);
    (events, limits)
}

/// Snapshot completo do Codex CLI: limites do plano + tokens dos rollouts recentes.
pub fn snapshot() -> AiUsage {
    let now_local = Local::now();
    let now = now_local.with_timezone(&Utc);
    let (events, limits) = scan(recent_files(sessions_dir()), now);

    let age_min = limits.seen_at.map(|at| (now - at).num_minutes().max(0));
    let mut usage = AiUsage {
        provider: "codex".into(),
        windows: limits.windows,
        age_min,
        // Sem nenhuma leitura de limites o dado conta como velho, como no Claude Code sem cache.
        stale: age_min.map_or(true, |min| min > STALE_MINUTES),
        block_tokens: active_block(&events, now_local),
        fetched_at: now_ms(),
        ..Default::default()
    };
    (usage.today_tokens, usage.today_messages, usage.top_model) = today_totals(&events, now_local.date_naive());
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

    fn window(id: &str, minutes: Option<i64>, percent: Option<i64>, resets_in: Option<i64>) -> AiWindow {
        AiWindow { id: id.into(), minutes, percent, resets_in }
    }

    #[test]
    fn parses_rate_limits() {
        let reset = now().timestamp() + 7 * 24 * 3600;
        let limits = json!({
            "limit_id": "codex",
            "primary": { "used_percent": 42.6, "window_minutes": 300, "resets_at": reset },
            "secondary": { "used_percent": 0.0, "window_minutes": 10080, "resets_at": reset + 60 },
            "credits": { "has_credits": false, "balance": "0" }
        });
        assert_eq!(parse_limits(&limits, now()), vec![
            window("primary", Some(300), Some(43), Some(7 * 24 * 60)),
            window("secondary", Some(10_080), Some(0), Some(7 * 24 * 60 + 1)),
        ]);
    }

    #[test]
    fn handles_missing_and_expired_windows() {
        // Conta por chave de API: nenhum limite de plano.
        assert!(parse_limits(&json!({ "primary": null, "secondary": null }), now()).is_empty());
        assert!(parse_limits(&json!({}), now()).is_empty());

        // A janela virou: percentual desconhecido, tempo restante negativo.
        let limits = json!({ "primary": { "used_percent": 90.0, "window_minutes": 300, "resets_at": now().timestamp() - 3600 } });
        assert_eq!(parse_limits(&limits, now()), vec![window("primary", Some(300), None, Some(-60))]);

        // Sem a duração a janela continua valendo, só sem rótulo.
        let no_minutes = json!({ "primary": { "used_percent": 15.0, "resets_at": now().timestamp() + 3600 } });
        assert_eq!(parse_limits(&no_minutes, now()), vec![window("primary", None, Some(15), Some(60))]);
    }

    #[test]
    fn parses_token_count_line() {
        let line = json!({
            "timestamp": "2026-10-16T14:00:25.805Z",
            "ordinal": 11,
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "total_token_usage": { "input_tokens": 12579, "output_tokens": 5, "total_tokens": 999_999 },
                    "last_token_usage": { "input_tokens": 12579, "output_tokens": 5, "total_tokens": 12584 },
                    "model_context_window": 258400
                },
                "rate_limits": { "primary": { "used_percent": 1.0, "window_minutes": 10080, "resets_at": 1 } }
            }
        })
        .to_string();
        let (event, limits) = parse_turn(&line).unwrap();
        // Só o gasto do turno entra; o acumulado da sessão é ignorado.
        assert_eq!(event.tokens, 12584);
        // O timestamp vem em UTC e é guardado no fuso local: o mesmo instante, em UTC de novo.
        assert_eq!(event.when.with_timezone(&Utc).to_rfc3339_opts(chrono::SecondsFormat::Millis, true), "2026-10-16T14:00:25.805Z");
        assert!(limits.is_some());

        // Sem `rate_limits` (chave de API) o evento continua valendo.
        let no_limits = json!({
            "timestamp": "2026-10-16T14:00:25.805Z",
            "type": "event_msg",
            "payload": { "type": "token_count", "info": { "last_token_usage": { "total_tokens": 10 } } }
        });
        let (event, limits) = parse_turn(&no_limits.to_string()).unwrap();
        assert_eq!((event.tokens, limits.is_some()), (10, false));
    }

    #[test]
    fn ignores_lines_that_are_not_turns() {
        let other_event = json!({ "timestamp": "2026-10-16T14:00:25Z", "type": "event_msg", "payload": { "type": "agent_message" } });
        assert!(parse_turn(&other_event.to_string()).is_none());
        let no_tokens = json!({ "timestamp": "2026-10-16T14:00:25Z", "type": "event_msg", "payload": { "type": "token_count", "info": {} } });
        assert!(parse_turn(&no_tokens.to_string()).is_none());
        let no_time = json!({ "type": "event_msg", "payload": { "type": "token_count", "info": { "last_token_usage": { "total_tokens": 1 } } } });
        assert!(parse_turn(&no_time.to_string()).is_none());
        assert!(parse_turn("{lixo").is_none());
    }

    #[test]
    fn reads_the_model_from_turn_context() {
        let turn = json!({ "type": "turn_context", "payload": { "turn_id": "01a1", "model": "gpt-5.6-luna" } });
        assert_eq!(parse_model(&turn.to_string()).as_deref(), Some("gpt-5.6-luna"));
        let token_count = json!({ "type": "event_msg", "payload": { "type": "token_count", "info": { "model_context_window": 258400 } } });
        assert_eq!(parse_model(&token_count.to_string()), None);
    }

    #[test]
    fn scan_keeps_the_newest_limits_and_sums_the_turns() {
        let root = std::env::temp_dir().join(format!("pier-codex-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let turn = |iso: &str, tokens: i64, percent: f64| {
            json!({
                "timestamp": iso,
                "type": "event_msg",
                "payload": {
                    "type": "token_count",
                    "info": { "last_token_usage": { "total_tokens": tokens } },
                    "rate_limits": { "primary": { "used_percent": percent, "window_minutes": 10080, "resets_at": now().timestamp() + 3600 } }
                }
            })
            .to_string()
        };
        let older = root.join("rollout-2026-10-15T10-00-00.jsonl");
        let newer = root.join("rollout-2026-10-16T10-00-00.jsonl");
        std::fs::write(&older, format!("{}\n{}\n", parse_model_line(), turn("2026-10-15T10:00:00Z", 100, 10.0))).unwrap();
        std::fs::write(&newer, format!("{}\n{}\n", parse_model_line(), turn("2026-10-16T10:00:00Z", 25, 80.0))).unwrap();

        let (events, limits) = scan(vec![newer, older], now());
        assert_eq!(events.iter().map(|e| e.tokens).sum::<i64>(), 125);
        assert_eq!(events[0].model.as_deref(), Some("gpt-5.6-luna"));
        // A leitura mais recente dos limites é a que vale.
        assert_eq!(limits.windows, vec![window("primary", Some(10_080), Some(80), Some(60))]);
        assert_eq!(limits.seen_at, Some(Utc.with_ymd_and_hms(2026, 10, 16, 10, 0, 0).unwrap()));

        std::fs::remove_dir_all(&root).unwrap();
    }

    fn parse_model_line() -> String {
        json!({ "type": "turn_context", "payload": { "model": "gpt-5.6-luna" } }).to_string()
    }
}

#[cfg(test)]
mod live_tests {
    /// Lê o uso real desta máquina. Os percentuais têm de bater com o `/status` do Codex CLI e com o
    /// `rate_limits` do último rollout em `~/.codex/sessions`.
    #[test]
    #[ignore = "smoke test: lê os arquivos do Codex CLI na máquina"]
    fn reads_local_usage() {
        let usage = super::snapshot();
        println!("janelas: {:?}", usage.windows);
        println!("idade da leitura: {:?} min (velho: {})", usage.age_min, usage.stale);
        println!("tokens hoje: {} em {} turnos", usage.today_tokens, usage.today_messages);
        println!("bloco de 5h: {}", usage.block_tokens);
        println!("modelo dominante: {:?}", usage.top_model);
    }
}
