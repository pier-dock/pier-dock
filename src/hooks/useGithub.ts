import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type GithubData } from "../lib/ipc";
import { setActiveInterval } from "../lib/dockActivity";

/**
 * Dados do GitHub. Só a dock faz polling (`poll` = true); painéis leem o cache do Rust e
 * recebem as atualizações pelo evento.
 */
export function useGithub(pollMinutes?: number): GithubData | null {
  const [data, setData] = useState<GithubData | null>(null);
  useEffect(() => {
    ipc.githubCached().then((d) => d && setData(d));
    const unlisten = listen<GithubData>("github-updated", (e) => setData(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  useEffect(() => {
    if (!pollMinutes) return;
    const fetchNow = () => ipc.githubFetch().catch(() => {});
    fetchNow();
    return setActiveInterval(fetchNow, Math.max(1, pollMinutes) * 60_000);
  }, [pollMinutes]);
  return data;
}
