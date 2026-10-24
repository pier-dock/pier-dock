import { ipc, type AgentSession, type AgentSessionState } from "../lib/ipc";
import { useAgentSessions } from "../hooks/useAgentSessions";
import { usePokemonSprite } from "../hooks/usePokemonSprite";
import { useConfig } from "../store/config";
import { Tile, togglePanel } from "./Tile";
import { useT } from "../i18n";
import { ClaudeMascotIcon } from "../dock/ClaudeMascotIcon";

/** Qual status manda no rótulo e na cor do sprite quando há várias sessões: quem mais precisa de
 * atenção primeiro. */
const PRIORITY: AgentSessionState[] = ["permission", "error", "running", "done"];

export function leadStatus(agents: AgentSession[]): AgentSessionState | null {
  for (const status of PRIORITY) {
    if (agents.some((a) => a.status === status)) return status;
  }
  return null;
}

/** Com uma sessão só (e a janela dela encontrada) foca direto o terminal; senão abre a lista
 * ancorada em `el`. É o clique do widget e do mascote (`AgentPet`). */
export function openAgentSessions(agents: AgentSession[], el: Element) {
  if (agents.length === 0) return;
  if (agents.length === 1 && agents[0].windowId !== null) {
    ipc.focusAgentSession(agents[0].provider, agents[0].sessionId).catch(console.error);
    return;
  }
  togglePanel("ai-agents", el);
}

export function AgentSessionsWidget() {
  const agents = useAgentSessions();
  const config = useConfig((s) => s.config);
  const t = useT().agentSessions;
  const mascot = config?.agentPetMascot ?? "clawd";
  const sprite = usePokemonSprite(mascot === "pokemon" ? (config?.agentPokemonId ?? 25) : null);
  const showPokemon = mascot === "pokemon" && !!sprite;
  const lead = leadStatus(agents);
  const label = agents.length === 0 ? t.idle : t.tile(agents.length, t.status[lead ?? "running"]);

  return (
    <Tile kind="ai-agents" label={label} className="claude-tile" onClick={(el) => openAgentSessions(agents, el)}>
      <span className={`claude-sprite ${agents.length ? "walking" : ""} ${lead ? `claude-${lead}` : ""}`} aria-hidden>
        {showPokemon && sprite ? (
          <img className="agent-tile-sprite" src={sprite} alt="" draggable={false} />
        ) : (
          <ClaudeMascotIcon size={18} />
        )}
      </span>
      {agents.length > 0 && (
        <span className="claude-dots" aria-hidden>
          {agents.slice(0, 4).map((a) => (
            <span key={`${a.provider}:${a.sessionId}`} className={`claude-dot claude-dot-${a.status}`} />
          ))}
        </span>
      )}
    </Tile>
  );
}
