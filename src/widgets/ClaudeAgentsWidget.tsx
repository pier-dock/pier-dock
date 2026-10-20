import { ipc, type ClaudeAgent, type ClaudeAgentState } from "../lib/ipc";
import { useClaudeAgents } from "../hooks/useClaudeAgents";
import { Tile, togglePanel } from "./Tile";
import { useT } from "../i18n";
import { ClaudeMascotIcon } from "../dock/ClaudeMascotIcon";

/** Qual status manda no rótulo e na cor do sprite quando há várias sessões: quem mais precisa de
 * atenção primeiro. */
const PRIORITY: ClaudeAgentState[] = ["permission", "error", "running", "done"];

export function leadStatus(agents: ClaudeAgent[]): ClaudeAgentState | null {
  for (const status of PRIORITY) {
    if (agents.some((a) => a.status === status)) return status;
  }
  return null;
}

/** Com uma sessão só (e a janela dela encontrada) foca direto o terminal; senão abre a lista
 * ancorada em `el`. É o clique do widget e do Clawd (`ClawdPet`). */
export function openClaudeAgents(agents: ClaudeAgent[], el: Element) {
  if (agents.length === 0) return;
  if (agents.length === 1 && agents[0].windowId !== null) {
    ipc.focusClaudeAgent(agents[0].sessionId).catch(console.error);
    return;
  }
  togglePanel("claude-agents", el);
}

export function ClaudeAgentsWidget() {
  const agents = useClaudeAgents();
  const t = useT().claudeAgents;
  const lead = leadStatus(agents);
  const label = agents.length === 0 ? t.idle : t.tile(agents.length, t.status[lead ?? "running"]);

  return (
    <Tile kind="claude-agents" label={label} className="claude-tile" onClick={(el) => openClaudeAgents(agents, el)}>
      <span className={`claude-sprite ${agents.length ? "walking" : ""} ${lead ? `claude-${lead}` : ""}`} aria-hidden>
        <ClaudeMascotIcon size={18} />
      </span>
      {agents.length > 0 && (
        <span className="claude-dots" aria-hidden>
          {agents.slice(0, 4).map((a) => (
            <span key={a.sessionId} className={`claude-dot claude-dot-${a.status}`} />
          ))}
        </span>
      )}
    </Tile>
  );
}
