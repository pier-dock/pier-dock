import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type WeatherData } from "../lib/ipc";
import { setActiveInterval } from "../lib/dockActivity";

/** Previsão do tempo. Só a dock faz polling (`poll`); os outros leem o cache do Rust. */
export function useWeather(poll = false, location?: string): WeatherData | null {
  const [data, setData] = useState<WeatherData | null>(null);
  useEffect(() => {
    ipc.weatherCached().then((d) => d && setData(d));
    const unlisten = listen<WeatherData>("weather-updated", (e) => setData(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  // Trocar de cidade busca na hora.
  useEffect(() => {
    if (!poll) return;
    const fetchNow = () => ipc.weatherFetch().catch(() => {});
    fetchNow();
    return setActiveInterval(fetchNow, 15 * 60_000);
  }, [poll, location]);
  return data;
}
