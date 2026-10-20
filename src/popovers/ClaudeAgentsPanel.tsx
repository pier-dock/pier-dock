import { ipc } from "../lib/ipc";
import { useClaudeAgents } from "../hooks/useClaudeAgents";
import { timeAgo } from "../lib/format";
import { useT } from "../i18n";
import { ClaudeMascotIcon } from "../dock/ClaudeMascotIcon";

/** Lista das sessões ativas; clicar numa foca a janela do terminal/IDE que a lançou. */
export function ClaudeAgentsPanel() {
  const agents = useClaudeAgents();
  const t = useT().claudeAgents;

  return (
    <>
      <header className="popover-header">
        <span className="popover-heading">{t.title}</span>
      </header>
      <div className="popover-body">
        {agents.length === 0 ? (
          <div className="popover-empty">
            <span className="empty-icon">
              <ClaudeMascotIcon size={22} />
            </span>
            <span className="empty-title">{t.empty}</span>
          </div>
        ) : (
          <ul className="list">
            {agents.map((a) => (
              <li key={a.sessionId}>
                <button
                  className="list-row claude-row"
                  disabled={a.windowId === null}
                  title={a.windowId === null ? t.focusFailed : undefined}
                  onClick={() => ipc.focusClaudeAgent(a.sessionId).catch(console.error)}
                >
                  <span className={`claude-dot claude-dot-${a.status}`} aria-hidden />
                  <span className="list-main">
                    <span className="list-title">{a.project || a.cwd}</span>
                    <span className="list-sub">
                      {t.status[a.status]} · {timeAgo(a.updatedAt)}
                    </span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </>
  );
}
