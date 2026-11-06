//! Read-only adapter for T3 Code's V2 run metadata. No messages, prompts, tool arguments,
//! credentials, or provider conversation databases are queried. Unsupported schemas fail closed.
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use super::RawStatus;

// Only the most recent run of each thread is relevant. Resolve its native session identity to
// deduplicate with CLI hooks and Codex rollouts, even when a thread switches providers.
const QUERY: &str = r#"
SELECT r.provider, t.thread_id,
       json_extract(pt.payload_json, '$.nativeThreadRef.nativeId'),
       COALESCE(json_extract(t.payload_json, '$.worktreePath'), p.workspace_root, ''),
       COALESCE(p.title, ''), r.status, r.requested_at, r.completed_at,
       EXISTS(SELECT 1 FROM orchestration_v2_projection_runtime_requests q
              JOIN orchestration_v2_projection_nodes n ON n.node_id = q.node_id
              WHERE q.thread_id = t.thread_id AND n.run_id = r.run_id
                AND q.status IN ('pending', 'open', 'waiting'))
FROM orchestration_v2_projection_threads t
JOIN orchestration_v2_projection_runs r ON r.thread_id = t.thread_id
LEFT JOIN orchestration_v2_projection_provider_threads pt ON pt.provider_thread_id = r.provider_thread_id
LEFT JOIN projection_projects p ON p.project_id = t.project_id
WHERE t.deleted_at IS NULL AND t.archived_at IS NULL
  AND r.ordinal = (SELECT MAX(latest.ordinal) FROM orchestration_v2_projection_runs latest
                   WHERE latest.thread_id = t.thread_id)
"#;

fn state_path() -> PathBuf {
    std::env::var_os("PIER_T3_STATE_FILE")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".t3/userdata/statev2.sqlite"))
}

pub(super) fn window_pids() -> Vec<u32> {
    crate::windows_list::with_cached_windows(|windows| {
        windows
            .iter()
            .filter(|w| w.title.contains("T3 Code") || w.exe.to_lowercase().contains("t3-code"))
            .map(|w| w.pid)
            .collect()
    })
}

pub(super) fn read(now: f64) -> Vec<RawStatus> {
    let path = state_path();
    if !path.is_file() {
        return Vec::new();
    }
    let pids = window_pids();
    // A persisted running row alone is not liveness evidence after T3 exits or crashes.
    if pids.is_empty() {
        return Vec::new();
    }
    let Ok(connection) =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
    else {
        return Vec::new();
    };
    let _ = connection.busy_timeout(Duration::from_millis(25));
    query(&connection, now, &pids).unwrap_or_default()
}

fn query(connection: &Connection, now: f64, pids: &[u32]) -> rusqlite::Result<Vec<RawStatus>> {
    let mut statement = connection.prepare(QUERY)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, bool>(8)?,
        ))
    })?;
    let mut result = Vec::new();
    for row in rows {
        let (driver, thread, native_id, cwd, project, run_status, requested, completed, pending) = row?;
        let Some(status) = status(&run_status, pending) else {
            continue;
        };
        let active = status == "running" || status == "permission";
        let updated_at = if active {
            now
        } else {
            let Some(at) = completed
                .as_deref()
                .or(Some(requested.as_str()))
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            else {
                continue;
            };
            at.timestamp_millis() as f64
        };
        let provider = match driver.as_str() {
            "claudeAgent" | "claude" => "claude",
            "codex" => "codex",
            "opencode" => "opencode",
            "gemini" => "gemini",
            "antigravity" => "antigravity",
            _ => "t3",
        };
        let session_id = match native_id.filter(|s| !s.is_empty()) {
            Some(id) if provider != "t3" => id,
            Some(id) => format!("t3:{driver}:{id}"),
            None => format!("t3:{driver}:{thread}"),
        };
        result.push(RawStatus {
            provider: provider.into(),
            session_id,
            status: status.into(),
            cwd,
            project,
            pid_chain: pids.to_vec(),
            updated_at,
        });
    }
    Ok(result)
}

fn status(run: &str, pending: bool) -> Option<&'static str> {
    match run {
        "preparing" | "queued" | "starting" | "running" | "waiting" => {
            Some(if pending { "permission" } else { "running" })
        }
        "completed" | "interrupted" | "cancelled" => Some("done"),
        "failed" => Some("error"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(r#"
          CREATE TABLE projection_projects (project_id TEXT, title TEXT, workspace_root TEXT);
          CREATE TABLE orchestration_v2_projection_threads (thread_id TEXT, project_id TEXT, payload_json TEXT, deleted_at TEXT, archived_at TEXT);
          CREATE TABLE orchestration_v2_projection_runs (run_id TEXT, thread_id TEXT, provider TEXT, ordinal INTEGER, provider_thread_id TEXT, status TEXT, requested_at TEXT, completed_at TEXT);
          CREATE TABLE orchestration_v2_projection_provider_threads (provider_thread_id TEXT, payload_json TEXT);
          CREATE TABLE orchestration_v2_projection_runtime_requests (thread_id TEXT, node_id TEXT, status TEXT);
          CREATE TABLE orchestration_v2_projection_nodes (node_id TEXT, run_id TEXT);
          INSERT INTO projection_projects VALUES ('project', 'Pier', '/projects/pier');
          INSERT INTO orchestration_v2_projection_threads VALUES ('agy', 'project', '{}', NULL, NULL), ('codex', 'project', '{}', NULL, NULL);
          INSERT INTO orchestration_v2_projection_provider_threads VALUES ('agy-native', '{"nativeThreadRef":{"nativeId":"agy-session"}}'), ('codex-native', '{"nativeThreadRef":{"nativeId":"codex-session"}}');
          INSERT INTO orchestration_v2_projection_runs VALUES
            ('old', 'agy', 'codex', 1, 'codex-native', 'running', '2026-10-22T20:00:00Z', NULL),
            ('agy-run', 'agy', 'antigravity', 2, 'agy-native', 'running', '2026-10-22T20:00:01Z', NULL),
            ('codex-run', 'codex', 'codex', 1, 'codex-native', 'running', '2026-10-22T20:00:01Z', NULL);
          INSERT INTO orchestration_v2_projection_nodes VALUES ('node', 'agy-run'), ('old-node', 'old');
        "#).unwrap();
        c
    }

    #[test]
    fn counts_concurrent_t3_providers_once_using_native_session_ids() {
        let list = query(&fixture(), 99.0, &[42]).unwrap();
        assert_eq!(list.len(), 2);
        let agy = list.iter().find(|s| s.provider == "antigravity").unwrap();
        assert_eq!(agy.session_id, "agy-session");
        assert_eq!(agy.status, "running");
        assert_eq!(agy.updated_at, 99.0);
        assert_eq!(agy.pid_chain, vec![42]);
        assert!(list
            .iter()
            .any(|s| s.provider == "codex" && s.session_id == "codex-session"));
    }

    #[test]
    fn permission_requires_a_pending_request_in_the_current_run() {
        let c = fixture();
        c.execute_batch(
            "INSERT INTO orchestration_v2_projection_runtime_requests VALUES ('agy', 'old-node', 'pending');",
        )
        .unwrap();
        assert!(query(&c, 99.0, &[]).unwrap().iter().all(|s| s.status == "running"));
        c.execute_batch("INSERT INTO orchestration_v2_projection_runtime_requests VALUES ('agy', 'node', 'pending');")
            .unwrap();
        assert_eq!(
            query(&c, 99.0, &[])
                .unwrap()
                .iter()
                .find(|s| s.provider == "antigravity")
                .unwrap()
                .status,
            "permission"
        );
        c.execute_batch("UPDATE orchestration_v2_projection_runtime_requests SET status = 'resolved';")
            .unwrap();
        assert!(query(&c, 99.0, &[]).unwrap().iter().all(|s| s.status == "running"));
    }

    #[test]
    fn completion_and_failure_are_not_renewed_and_archived_threads_are_excluded() {
        let c = fixture();
        c.execute_batch("UPDATE orchestration_v2_projection_runs SET status = 'failed', completed_at = '2026-10-22T20:00:03Z' WHERE run_id = 'agy-run';").unwrap();
        let list = query(&c, 9999999999999.0, &[]).unwrap();
        let agy = list.iter().find(|s| s.provider == "antigravity").unwrap();
        assert_eq!(agy.status, "error");
        assert!(super::super::is_expired(
            &agy.status,
            (9999999999999.0 - agy.updated_at) / 1000.0
        ));
        c.execute_batch("UPDATE orchestration_v2_projection_threads SET archived_at = 'now' WHERE thread_id = 'agy';")
            .unwrap();
        assert_eq!(query(&c, 99.0, &[]).unwrap().len(), 1);
    }

    #[test]
    fn unknown_providers_and_unbound_native_threads_still_count() {
        let c = fixture();
        c.execute_batch("UPDATE orchestration_v2_projection_runs SET provider = 'custom-ai', provider_thread_id = NULL WHERE run_id = 'agy-run';").unwrap();
        let list = query(&c, 99.0, &[]).unwrap();
        assert!(list
            .iter()
            .any(|s| s.provider == "t3" && s.session_id == "t3:custom-ai:agy"));
        c.execute_batch(
            "UPDATE orchestration_v2_projection_runs SET provider_thread_id = 'codex-native' WHERE run_id = 'agy-run';",
        )
        .unwrap();
        assert!(query(&c, 99.0, &[])
            .unwrap()
            .iter()
            .any(|s| s.provider == "t3" && s.session_id == "t3:custom-ai:codex-session"));
        assert_eq!(status("waiting", false), Some("running"));
        assert_eq!(status("interrupted", false), Some("done"));
        assert_eq!(status("idle", false), None);
    }

    #[test]
    fn missing_schema_does_not_guess_running_sessions() {
        assert!(query(&Connection::open_in_memory().unwrap(), 99.0, &[]).is_err());
    }

    #[test]
    #[ignore = "smoke test: reads local T3 run metadata"]
    fn reads_live_antigravity_run() {
        let list = read(crate::ai_usage::now_ms());
        for s in &list {
            println!("{} | {} | {}", s.provider, s.project, s.status);
        }
        assert!(list
            .iter()
            .any(|s| s.provider == "antigravity" && s.status == "running"));
    }
}
