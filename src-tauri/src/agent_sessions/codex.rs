//! Codex CLI and app-server lifecycle fallback. Hooks are not required: T3 Code and other
//! app-server clients persist task_started/task_complete in the same local rollout files.
//! Only metadata and lifecycle fields are deserialized; messages and tool contents are ignored.
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::Deserialize;

use super::{codex_root, is_expired, RawStatus, STALE_RUNNING_SECS};

const MAX_RECORD: usize = 1024 * 1024;

#[derive(Default)]
pub(super) struct Sessions {
    files: HashMap<PathBuf, Tail>,
    last_scan: Option<Instant>,
}

impl Sessions {
    pub(super) fn read(&mut self, now: f64) -> Vec<RawStatus> {
        self.read_from(codex_root().join("sessions"), now)
    }

    fn read_from(&mut self, root: PathBuf, now: f64) -> Vec<RawStatus> {
        if self.last_scan.is_none_or(|at| at.elapsed() >= Duration::from_secs(5)) {
            let cutoff = SystemTime::now()
                .checked_sub(Duration::from_secs(STALE_RUNNING_SECS as u64))
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let paths = crate::ai_usage::recent_files_in(root, cutoff);
            self.files.retain(|path, _| paths.contains(path));
            for path in paths {
                self.files.entry(path).or_default();
            }
            self.last_scan = Some(Instant::now());
        }
        let mut result: HashMap<String, RawStatus> = HashMap::new();
        for (path, tail) in &mut self.files {
            tail.read(path);
            if tail.session_id.is_empty() {
                continue;
            }
            if tail.status.is_empty() {
                continue;
            }
            let project = Path::new(&tail.cwd)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            // An app-server can serve many projects from one process. The rollout identifies its
            // client, not its PID. For T3, use its actual open windows; never guess a terminal PID.
            let pid_chain = if tail.originator.eq_ignore_ascii_case("T3 Code") {
                super::t3::window_pids()
            } else {
                Vec::new()
            };
            let raw = RawStatus {
                provider: "codex".into(),
                session_id: tail.session_id.clone(),
                status: tail.status.clone(),
                cwd: tail.cwd.clone(),
                project,
                pid_chain,
                updated_at: tail.updated_at,
            };
            match result.get(&raw.session_id) {
                Some(existing) if existing.updated_at >= raw.updated_at => {}
                _ => {
                    result.insert(raw.session_id.clone(), raw);
                }
            }
        }
        // Deduplicate before expiring: an older running copy must not revive a session whose
        // newer completion has just expired.
        result
            .into_values()
            .filter(|raw| !is_expired(&raw.status, (now - raw.updated_at) / 1000.0))
            .collect()
    }
}

#[derive(Default)]
struct Tail {
    offset: u64,
    pending: Vec<u8>,
    oversized: bool,
    session_id: String,
    cwd: String,
    originator: String,
    turn_id: Option<String>,
    status: String,
    updated_at: f64,
}

#[derive(Deserialize)]
struct Record {
    #[serde(rename = "type")]
    kind: String,
    timestamp: Option<String>,
    payload: Payload,
}

#[derive(Deserialize, Default)]
struct Payload {
    #[serde(rename = "type", default)]
    kind: String,
    id: Option<String>,
    cwd: Option<String>,
    originator: Option<String>,
    turn_id: Option<String>,
    error: Option<serde::de::IgnoredAny>,
}

impl Tail {
    fn read(&mut self, path: &Path) {
        let Ok(mut file) = File::open(path) else { return };
        let Ok(meta) = file.metadata() else { return };
        if meta.len() < self.offset {
            *self = Self::default();
        }
        if meta.len() == self.offset || file.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }
        let mut buffer = [0; 8192];
        while let Ok(n) = file.read(&mut buffer) {
            if n == 0 {
                break;
            }
            self.offset += n as u64;
            self.feed(&buffer[..n]);
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for part in bytes.split_inclusive(|b| *b == b'\n') {
            if !self.oversized && self.pending.len() + part.len() <= MAX_RECORD {
                self.pending.extend_from_slice(part);
            } else {
                self.pending.clear();
                self.oversized = true;
            }
            if part.ends_with(b"\n") {
                if !self.oversized {
                    if let Ok(record) = serde_json::from_slice::<Record>(&self.pending) {
                        self.apply(record);
                    }
                }
                self.pending.clear();
                self.oversized = false;
            }
        }
    }

    fn apply(&mut self, record: Record) {
        let payload = record.payload;
        match record.kind.as_str() {
            "session_meta" => {
                if let Some(id) = payload.id {
                    self.session_id = id;
                }
                if let Some(cwd) = payload.cwd {
                    self.cwd = cwd;
                }
                if let Some(originator) = payload.originator {
                    self.originator = originator;
                }
                return;
            }
            "turn_context" => {
                if let Some(cwd) = payload.cwd {
                    self.cwd = cwd;
                }
                return;
            }
            "event_msg" => {}
            _ => return,
        }
        let Some(timestamp) = record
            .timestamp
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        else {
            return;
        };
        let at = timestamp.timestamp_millis() as f64;
        if at < self.updated_at {
            return;
        }
        match payload.kind.as_str() {
            "task_started" => {
                self.turn_id = payload.turn_id;
                self.status = "running".into();
            }
            "task_complete" | "turn_aborted" => {
                if payload.turn_id.is_some() && self.turn_id.is_some() && payload.turn_id != self.turn_id {
                    return;
                }
                self.status = if payload.error.is_some() { "error" } else { "done" }.into();
            }
            _ if self.status == "running" => {} // Activity refreshes liveness, never revives a finished turn.
            _ => return,
        }
        self.updated_at = at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(kind: &str, turn: &str, second: u32) -> String {
        format!("{{\"type\":\"event_msg\",\"timestamp\":\"2026-10-22T20:00:{second:02}Z\",\"payload\":{{\"type\":\"{kind}\",\"turn_id\":\"{turn}\"}}}}\n")
    }
    fn session() -> Tail {
        let mut tail = Tail::default();
        tail.feed(b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"session-a\",\"cwd\":\"/projects/pier\",\"originator\":\"T3 Code\"}}\n");
        tail
    }
    #[test]
    fn concurrent_sessions_are_deduplicated_and_finished_sessions_expire() {
        let root = std::env::temp_dir().join(format!("pier-codex-sessions-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let meta = "{\"type\":\"session_meta\",\"payload\":{\"id\":\"a\",\"cwd\":\"/project\"}}\n";
        std::fs::write(
            root.join("a.jsonl"),
            format!("{meta}{}", event("task_started", "turn-a", 0)),
        )
        .unwrap();
        std::fs::write(
            root.join("resumed-a.jsonl"),
            format!("{meta}{}", event("task_complete", "turn-a", 1)),
        )
        .unwrap();
        std::fs::write(
            root.join("b.jsonl"),
            format!(
                "{}{}",
                meta.replace("\"a\"", "\"b\""),
                event("task_started", "turn-b", 0)
            ),
        )
        .unwrap();
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-22T20:00:02Z")
            .unwrap()
            .timestamp_millis() as f64;
        let mut sessions = Sessions::default();
        let list = sessions.read_from(root.clone(), at);
        assert_eq!(list.len(), 2);
        assert_eq!(list.iter().find(|s| s.session_id == "a").unwrap().status, "done");
        let list = sessions.read_from(root.clone(), at + 21_000.0);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_id, "b");
        assert!(sessions.read_from(root.clone(), at + 601_000.0).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "smoke test: reads local Codex lifecycle metadata"]
    fn reads_live_codex_sessions() {
        let list = Sessions::default().read(crate::ai_usage::now_ms());
        for session in &list {
            println!("{} | {} | {}", session.session_id, session.project, session.status);
        }
        assert!(list.iter().any(|s| s.status == "running"));
    }
    #[test]
    fn t3_turns_start_finish_and_resume_without_hooks() {
        let mut tail = session();
        assert!(tail.status.is_empty());
        tail.feed(event("task_started", "a", 0).as_bytes());
        assert_eq!(tail.status, "running");
        tail.feed(event("task_complete", "a", 1).as_bytes());
        assert_eq!(tail.status, "done");
        let done_at = tail.updated_at;
        tail.feed(event("token_count", "a", 2).as_bytes());
        assert_eq!(tail.updated_at, done_at);
        tail.feed(event("task_started", "b", 3).as_bytes());
        tail.feed(event("task_complete", "a", 4).as_bytes());
        assert_eq!(tail.status, "running");
        tail.feed(event("turn_aborted", "b", 5).as_bytes());
        assert_eq!(tail.status, "done");
    }
    #[test]
    fn partial_records_and_large_messages_do_not_drop_lifecycle_events() {
        let mut tail = session();
        let start = event("task_started", "a", 0);
        tail.feed(&start.as_bytes()[..30]);
        assert!(tail.status.is_empty());
        tail.feed(&start.as_bytes()[30..]);
        tail.feed(&vec![b'x'; MAX_RECORD * 2]);
        assert!(tail.pending.len() <= MAX_RECORD);
        tail.feed(b"\nnot-json\n");
        tail.feed(event("task_complete", "a", 1).as_bytes());
        assert_eq!(tail.status, "done");
    }
    #[test]
    fn failure_is_error_and_activity_keeps_only_running_turns_alive() {
        let mut tail = session();
        tail.feed(event("task_started", "a", 0).as_bytes());
        let started = tail.updated_at;
        tail.feed(event("token_count", "a", 1).as_bytes());
        assert!(tail.updated_at > started);
        tail.feed(
            event("task_complete", "a", 2)
                .replace(
                    "\"turn_id\":\"a\"",
                    "\"turn_id\":\"a\",\"error\":{\"message\":\"ignored\"}",
                )
                .as_bytes(),
        );
        assert_eq!(tail.status, "error");
    }
    #[test]
    fn file_tail_reads_only_appended_bytes_and_resets_on_truncation() {
        use std::io::Write;
        let path = std::env::temp_dir().join(format!("pier-codex-tail-{}.jsonl", std::process::id()));
        std::fs::write(&path, event("task_started", "a", 0)).unwrap();
        let mut tail = session();
        tail.read(&path);
        let offset = tail.offset;
        tail.read(&path);
        assert_eq!(tail.offset, offset);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(event("task_complete", "a", 1).as_bytes())
            .unwrap();
        tail.read(&path);
        assert_eq!(tail.status, "done");
        std::fs::write(&path, event("task_started", "b", 2)).unwrap();
        tail.read(&path);
        assert_eq!(tail.status, "running");
        assert!(tail.session_id.is_empty());
        std::fs::remove_file(path).unwrap();
    }
}
