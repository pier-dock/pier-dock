import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type AiProviderChoice, type AiUsage } from "../lib/ipc";

/**
 * Uso das IAs de linha de comando. Só a dock faz polling (`pollMinutes`), porque a leitura varre
 * arquivos em disco; o painel lê o cache do Rust e recebe as atualizações pelo evento.
 */
export function useAiUsage(pollMinutes?: number, provider: AiProviderChoice = "auto"): AiUsage | null {
  const [data, setData] = useState<AiUsage | null>(null);
  useEffect(() => {
    ipc.aiUsageCached().then((d) => d && setData(d));
    const unlisten = listen<AiUsage>("ai-usage-updated", (e) => setData(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  useEffect(() => {
    if (!pollMinutes) return;
    const fetchNow = () => ipc.aiUsage(provider).catch(() => {});
    fetchNow();
    const id = setInterval(fetchNow, Math.max(1, pollMinutes) * 60_000);
    return () => clearInterval(id);
  }, [pollMinutes, provider]);
  return data;
}
