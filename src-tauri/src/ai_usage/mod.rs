//! Uso das IAs de linha de comando: percentuais reais de limite + contagem de tokens.
//!
//! O widget nasceu lendo só o Claude Code; hoje ele lê qualquer uma das IAs de `PROVIDERS`, a
//! escolhida na configuração ("auto", "claude" ou "codex"). Cada provedor entrega o mesmo formato —
//! as janelas de limite (percentual e tempo até o reset), os tokens do dia e do bloco em andamento
//! e o modelo dominante —, então o frontend não sabe de qual CLI veio o número.
//!
//! Tudo sai de arquivos que os próprios CLIs já mantêm em disco e nada vai para a rede. De cada
//! transcript são lidos apenas os campos numéricos de `usage`, o nome do modelo e o timestamp;
//! nunca o conteúdo das conversas. Veja `claude.rs` e `codex.rs` para as fontes de cada um.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local, NaiveDate, Timelike};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::util::ErrStr;

pub(crate) mod claude;
pub(crate) mod codex;

/// Arquivo mais velho que isso não influencia nem "hoje" nem o bloco de 5h, então nem é aberto.
/// É o que derruba a varredura de centenas de arquivos para um punhado.
pub(crate) const LOOKBACK_HOURS: i64 = 48;

/// Tamanho da janela de uso que o Claude Code chama de "sessão".
pub(crate) const BLOCK_HOURS: i64 = 5;

/// Acima disso o percentual é considerado velho. O cache de limites só é renovado enquanto o CLI
/// roda: com ele fechado o número congela, e mostrar um valor parado como se fosse atual seria pior
/// do que avisar.
pub(crate) const STALE_MINUTES: i64 = 15;

/// As IAs que o widget sabe ler, na ordem de preferência do "auto".
pub const PROVIDERS: [&str; 2] = ["claude", "codex"];

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiWindow {
    /// Papel da janela na ferramenta (`session`, `weekly`, `primary`, `secondary`). O rótulo vem da
    /// duração, não daqui; o id só serve de chave para a lista no frontend.
    pub id: String,
    /// Duração da janela em minutos, quando a ferramenta informa (300 = sessão de 5h, 10080 = semana).
    pub minutes: Option<i64>,
    /// Percentual já usado. `None` quer dizer desconhecido — cache ausente ou janela que já virou —,
    /// nunca zero.
    pub percent: Option<i64>,
    /// Minutos até o reset da janela. Negativo se já passou.
    pub resets_in: Option<i64>,
}

/// O snapshot de uma IA, já normalizado entre os provedores.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiUsage {
    /// Qual IA respondeu: "claude" ou "codex".
    pub provider: String,
    /// As janelas de limite, na ordem da ferramenta. Vazia quando ela não guarda percentuais.
    pub windows: Vec<AiWindow>,
    /// Idade da leitura dos limites, em minutos.
    pub age_min: Option<i64>,
    pub stale: bool,
    /// Tokens somados dos transcripts (entrada + saída + escrita e leitura de cache).
    pub today_tokens: i64,
    pub today_messages: i64,
    pub block_tokens: i64,
    pub top_model: Option<String>,
    pub fetched_at: f64,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiProvider {
    pub id: String,
    /// Se os arquivos de dados da IA existem nesta máquina.
    pub detected: bool,
}

static CACHE: Mutex<Option<AiUsage>> = Mutex::new(None);

// --- peças comuns aos provedores --------------------------------------------

/// Um turno com tokens gastos, seja de um transcript do Claude Code ou de um rollout do Codex.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub when: DateTime<Local>,
    pub tokens: i64,
    pub model: Option<String>,
}

pub fn now_ms() -> f64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

/// Arquivos de `root` modificados dentro da janela de lookback, em qualquer nível abaixo dele.
///
/// A varredura é recursiva porque o Claude Code guarda a sessão principal em `projects/<slug>/<sessão>.jsonl`
/// e os subagentes em `projects/<slug>/<sessão>/subagents/agent-*.jsonl` — e o que eles gastam conta
/// no mesmo limite; o Codex usa `sessions/<ano>/<mês>/<dia>/rollout-*.jsonl`. Só diretórios de
/// verdade entram na pilha, para um link não virar laço.
pub fn recent_files_in(root: PathBuf, cutoff: SystemTime) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(entry.path());
            } else if entry.path().extension().is_some_and(|ext| ext == "jsonl")
                && entry.metadata().and_then(|m| m.modified()).is_ok_and(|m| m >= cutoff)
            {
                files.push(entry.path());
            }
        }
    }
    files
}

/// Os arquivos do provedor dentro da janela de lookback.
pub fn recent_files(root: PathBuf) -> Vec<PathBuf> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs((LOOKBACK_HOURS * 3600) as u64))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    recent_files_in(root, cutoff)
}

/// Tokens do bloco de 5h em andamento.
///
/// Um bloco novo começa quando passam 5h do início do bloco atual ou quando há um intervalo de mais
/// de 5h sem atividade. O início é arredondado para baixo na hora cheia. Mesma regra do ccusage.
pub fn active_block(events: &[Event], now: DateTime<Local>) -> i64 {
    let window = chrono::Duration::hours(BLOCK_HOURS);
    let mut start: Option<DateTime<Local>> = None;
    let mut last: Option<DateTime<Local>> = None;
    let mut total = 0;

    for event in events {
        let gap = last.map_or(0, |l| (event.when - l).num_seconds());
        let elapsed = start.map_or(0, |s| (event.when - s).num_seconds());
        if start.is_none() || elapsed > window.num_seconds() || gap > window.num_seconds() {
            let hour = event.when.with_minute(0).and_then(|d| d.with_second(0)).and_then(|d| d.with_nanosecond(0));
            start = Some(hour.unwrap_or(event.when));
            total = 0;
        }
        total += event.tokens;
        last = Some(event.when);
    }

    match start {
        // Passadas as 5h do início, o bloco expirou e não há uso corrente.
        Some(start) if now < start + window => total,
        _ => 0,
    }
}

/// Tokens de hoje, turnos de hoje e o modelo dominante do dia.
pub fn today_totals(events: &[Event], today: NaiveDate) -> (i64, i64, Option<String>) {
    let mut models: HashMap<&str, i64> = HashMap::new();
    let (mut tokens, mut messages) = (0, 0);
    for event in events.iter().filter(|e| e.when.date_naive() == today) {
        tokens += event.tokens;
        messages += 1;
        if let Some(model) = event.model.as_deref() {
            *models.entry(model).or_default() += 1;
        }
    }
    (tokens, messages, models.into_iter().max_by_key(|&(_, count)| count).map(|(model, _)| model.to_string()))
}

// --- provedores -------------------------------------------------------------

pub fn provider_detected(id: &str) -> bool {
    match id {
        "codex" => codex::detected(),
        _ => claude::detected(),
    }
}

fn snapshot_for(id: &str) -> AiUsage {
    match id {
        "codex" => codex::snapshot(),
        _ => claude::snapshot(),
    }
}

/// Snapshot da IA escolhida. "auto" (ou qualquer valor desconhecido, de uma config antiga) decide
/// entre todas: entre as que têm percentual conhecido, a de leitura mais recente; empate fica com a
/// primeira da lista, para o resultado não depender da ordem em que os arquivos foram lidos.
///
/// Sem percentual em nenhuma delas — CLI que nunca rodou ou conta por chave de API, que não guarda
/// limites — vale a que gastou tokens hoje; se nenhuma gastou, a primeira, que é o Claude Code, e o
/// widget mostra o estado "sem dado" de sempre.
pub fn pick_auto(candidates: Vec<AiUsage>) -> AiUsage {
    let with_limits: Vec<&AiUsage> = candidates.iter().filter(|u| u.windows.iter().any(|w| w.percent.is_some())).collect();
    let chosen = if !with_limits.is_empty() {
        let mut best = with_limits[0];
        for usage in &with_limits {
            if usage.age_min.unwrap_or(i64::MAX) < best.age_min.unwrap_or(i64::MAX) {
                best = usage;
            }
        }
        Some(best)
    } else {
        let mut best: Option<&AiUsage> = None;
        for usage in candidates.iter().filter(|u| u.today_tokens > 0) {
            if best.is_none_or(|b| usage.today_tokens > b.today_tokens) {
                best = Some(usage);
            }
        }
        best.or_else(|| candidates.first())
    };
    chosen.cloned().unwrap_or_default()
}

fn snapshot(choice: &str) -> AiUsage {
    match choice {
        "claude" | "codex" => snapshot_for(choice),
        _ => pick_auto(PROVIDERS.iter().map(|id| snapshot_for(id)).collect()),
    }
}

// --- comandos ---------------------------------------------------------------

#[tauri::command]
pub async fn ai_usage(app: AppHandle, provider: String) -> Result<AiUsage, String> {
    // Varre arquivos em disco, então sai da thread assíncrona.
    let usage = tauri::async_runtime::spawn_blocking(move || snapshot(&provider)).await.err_str()?;
    *CACHE.lock().unwrap() = Some(usage.clone());
    let _ = app.emit("ai-usage-updated", &usage);
    Ok(usage)
}

#[tauri::command]
pub fn ai_usage_cached() -> Option<AiUsage> {
    CACHE.lock().unwrap().clone()
}

/// As IAs conhecidas e quais delas têm dados nesta máquina, para a escolha nas configurações.
#[tauri::command]
pub fn ai_providers() -> Vec<AiProvider> {
    PROVIDERS.iter().map(|id| AiProvider { id: (*id).into(), detected: provider_detected(id) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashSet;

    fn at(iso: &str) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(iso).unwrap().with_timezone(&Local)
    }

    fn event(iso: &str, tokens: i64) -> Event {
        Event { when: at(iso), tokens, model: Some("claude-opus-5".into()) }
    }

    fn usage(provider: &str, percent: Option<i64>, age_min: Option<i64>, tokens: i64) -> AiUsage {
        AiUsage {
            provider: provider.into(),
            windows: vec![AiWindow {
                id: "session".into(),
                minutes: Some(300),
                percent,
                resets_in: Some(100),
            }],
            age_min,
            today_tokens: tokens,
            ..Default::default()
        }
    }

    #[test]
    fn finds_transcripts_at_any_depth() {
        // A sessão principal fica logo abaixo do projeto, os subagentes dois níveis mais fundo.
        let root = std::env::temp_dir().join(format!("pier-ai-test-{}", std::process::id()));
        let subagents = root.join("projeto").join("sessao").join("subagents");
        std::fs::create_dir_all(&subagents).unwrap();
        std::fs::write(root.join("projeto").join("sessao.jsonl"), "{}").unwrap();
        std::fs::write(subagents.join("agent-1.jsonl"), "{}").unwrap();
        std::fs::write(root.join("projeto").join("notas.txt"), "x").unwrap();

        let found = recent_files_in(root.clone(), SystemTime::UNIX_EPOCH);
        let names: HashSet<_> = found.iter().filter_map(|p| p.file_name()).collect();
        assert_eq!(names.len(), 2, "achou {found:?}");
        assert!(names.contains(std::ffi::OsStr::new("sessao.jsonl")));
        assert!(names.contains(std::ffi::OsStr::new("agent-1.jsonl")));

        // Fora da janela de lookback nada é devolvido.
        let future = SystemTime::now() + Duration::from_secs(3600);
        assert!(recent_files_in(root.clone(), future).is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn block_sums_current_window() {
        let events = [event("2026-10-16T10:30:00Z", 100), event("2026-10-16T12:00:00Z", 50)];
        // O bloco começa às 10h (hora cheia) e vale até as 15h.
        assert_eq!(active_block(&events, at("2026-10-16T13:00:00Z")), 150);
    }

    #[test]
    fn gap_longer_than_block_starts_over() {
        let events = [event("2026-10-16T02:00:00Z", 100), event("2026-10-16T12:00:00Z", 50)];
        assert_eq!(active_block(&events, at("2026-10-16T13:00:00Z")), 50);
    }

    #[test]
    fn block_expires_after_five_hours() {
        let events = [event("2026-10-16T02:00:00Z", 100)];
        assert_eq!(active_block(&events, at("2026-10-16T13:00:00Z")), 0);
        assert_eq!(active_block(&[], at("2026-10-16T13:00:00Z")), 0);
    }

    #[test]
    fn long_session_rolls_into_new_block() {
        // Atividade contínua: passadas as 5h do início, um bloco novo começa e o total reinicia.
        let events = [
            event("2026-10-16T08:10:00Z", 10),
            event("2026-10-16T11:00:00Z", 20),
            event("2026-10-16T13:30:00Z", 30),
        ];
        assert_eq!(active_block(&events, at("2026-10-16T14:00:00Z")), 30);
    }

    #[test]
    fn today_totals_pick_the_dominant_model() {
        let today = Local.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap().date_naive();
        let events = [
            Event { model: Some("gpt-5.6-luna".into()), ..event("2026-10-16T10:00:00Z", 10) },
            event("2026-10-16T11:00:00Z", 20),
            Event { model: Some("gpt-5.6-luna".into()), ..event("2026-10-16T12:00:00Z", 30) },
            // O evento de ontem fica fora dos totais de hoje.
            Event { model: Some("gpt-5.6-luna".into()), ..event("2026-10-15T11:00:00Z", 40) },
        ];
        assert_eq!(today_totals(&events, today), (60, 3, Some("gpt-5.6-luna".into())));
        assert_eq!(today_totals(&[], today), (0, 0, None));
    }

    #[test]
    fn auto_picks_the_freshest_limits() {
        let claude = usage("claude", Some(10), Some(5), 0);
        let codex = usage("codex", Some(80), Some(1), 0);
        assert_eq!(pick_auto(vec![claude.clone(), codex.clone()]).provider, "codex");
        assert_eq!(pick_auto(vec![codex, claude]).provider, "codex");
    }

    #[test]
    fn auto_ignores_who_has_no_limits() {
        let claude = usage("claude", None, Some(1), 0);
        let codex = usage("codex", Some(20), Some(600), 0);
        // O Claude leu mais recentemente, mas não tem percentual; vale quem tem.
        assert_eq!(pick_auto(vec![claude, codex]).provider, "codex");
    }

    #[test]
    fn auto_falls_back_to_tokens_then_to_the_first() {
        let claude = usage("claude", None, None, 100);
        let codex = usage("codex", None, None, 900);
        assert_eq!(pick_auto(vec![claude.clone(), codex.clone()]).provider, "codex");
        assert_eq!(pick_auto(vec![codex, claude]).provider, "codex");

        let quiet = usage("claude", None, None, 0);
        let also_quiet = usage("codex", None, None, 0);
        assert_eq!(pick_auto(vec![quiet, also_quiet]).provider, "claude");
        // Config antiga sem nenhum provedor disponível: o padrão vazio, sem quebrar.
        assert_eq!(pick_auto(Vec::new()).provider, "");
    }
}
