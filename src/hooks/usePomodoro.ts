import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type PomodoroState } from "../lib/ipc";
import { setActiveInterval } from "../lib/dockActivity";

/** Estado do timer (que vive no Rust) e o tempo restante, atualizado a cada segundo. */
export function usePomodoro(): { state: PomodoroState | null; remainingMs: number } {
  const [state, setState] = useState<PomodoroState | null>(null);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    ipc.pomodoroState().then(setState).catch(() => {});
    const unlisten = listen<PomodoroState>("pomodoro-changed", (e) => setState(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  useEffect(() => {
    if (!state?.running) return;
    return setActiveInterval(() => setNow(Date.now()), 250);
  }, [state?.running]);
  const remainingMs = state?.running && state.endsAtMs ? Math.max(0, state.endsAtMs - now) : (state?.remainingMs ?? 0);
  return { state, remainingMs };
}

export const pomodoro = (action: "toggle" | "skip" | "reset") => () => void ipc.pomodoroControl(action).catch(() => {});
