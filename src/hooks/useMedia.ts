import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type MediaSession, type MediaState } from "../lib/ipc";
import { setActiveInterval } from "../lib/dockActivity";

export function useMedia(enabled = true): MediaState {
  const [state, setState] = useState<MediaState>({ sessions: [] });
  useEffect(() => {
    if (!enabled) return;
    ipc.mediaState().then(setState);
    const unlisten = listen<MediaState>("media-changed", (e) => setState(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, [enabled]);
  return state;
}

/** Sessão a exibir: a que está tocando, senão a "atual" do Windows, senão a primeira. */
export function pickSession(state: MediaState, preferredId?: string | null): MediaSession | undefined {
  const { sessions, current } = state;
  return (
    sessions.find((s) => s.id === preferredId) ??
    sessions.find((s) => s.playing) ??
    sessions.find((s) => s.id === current) ??
    sessions[0]
  );
}

const thumbs = new Map<string, Promise<string | null>>();

export function useThumbnail(key: string | null | undefined): string | null {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    if (!key) {
      setSrc(null);
      return;
    }
    let pending = thumbs.get(key);
    if (!pending) {
      pending = ipc.mediaThumbnail(key);
      thumbs.set(key, pending);
    }
    let alive = true;
    pending.then((s) => alive && setSrc(s));
    return () => {
      alive = false;
    };
  }, [key]);
  return src;
}

/** Posição atual interpolada a partir da última medição (o Spotify publica a posição raramente). */
export function usePlaybackPosition(session: MediaSession | undefined): number {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!session?.playing) return;
    return setActiveInterval(() => setNow(Date.now()), 500);
  }, [session?.playing]);
  if (!session) return 0;
  const elapsed = session.playing ? Math.max(0, now - session.updatedAtMs) : 0;
  return Math.min(session.positionMs + elapsed, session.durationMs || Infinity);
}
