import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type MarketData } from "../lib/ipc";
import { setActiveInterval } from "../lib/dockActivity";

/**
 * Cotações do mercado. Só a dock faz polling (`pollMinutes`); painéis e a pré-visualização leem o
 * cache do Rust e recebem as atualizações pelo evento. `reloadKey` muda quando os ativos mudam,
 * para buscar na hora em vez de esperar o próximo intervalo.
 */
export function useMarket(pollMinutes?: number, reloadKey?: string): MarketData | null {
  const [data, setData] = useState<MarketData | null>(null);
  useEffect(() => {
    ipc.marketCached().then((d) => d && setData(d));
    const unlisten = listen<MarketData>("market-updated", (e) => setData(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  useEffect(() => {
    if (!pollMinutes) return;
    const fetchNow = () => ipc.marketFetch().catch(() => {});
    fetchNow();
    return setActiveInterval(fetchNow, Math.max(1, pollMinutes) * 60_000);
  }, [pollMinutes, reloadKey]);
  return data;
}
