import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type ClaudeAgent } from "../lib/ipc";

/** Sessões do Claude Code ao vivo, lidas dos hooks instalados em `~/.claude/settings.json`. */
export function useClaudeAgents(): ClaudeAgent[] {
  const [agents, setAgents] = useState<ClaudeAgent[]>([]);
  useEffect(() => {
    ipc.getClaudeAgents().then(setAgents).catch(() => {});
    const unlisten = listen<ClaudeAgent[]>("claude-agents-changed", (e) => setAgents(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  return agents;
}
