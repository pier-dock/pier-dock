import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type Note } from "../lib/ipc";

/** Os lembretes são arquivos editáveis fora do app, então a dock relê a pasta de tempos em tempos. */
const REFRESH_MS = 30_000;

/**
 * Lembretes salvos em Documentos. Só quem passa `poll` relê a pasta; os demais recebem as
 * mudanças feitas pelo painel pelo evento `notes-updated`.
 */
export function useNotes(poll = false): Note[] {
  const [notes, setNotes] = useState<Note[]>([]);
  useEffect(() => {
    const load = () => ipc.notesList().then(setNotes).catch(() => {});
    load();
    const unlisten = listen<Note[]>("notes-updated", (e) => setNotes(e.payload));
    const id = poll ? window.setInterval(load, REFRESH_MS) : undefined;
    return () => {
      unlisten.then((f) => f());
      if (id) window.clearInterval(id);
    };
  }, [poll]);
  return notes;
}
