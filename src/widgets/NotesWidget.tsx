import { useEffect, useState } from "react";
import { StickyNote } from "lucide-react";
import type { Note } from "../lib/ipc";
import { useT } from "../i18n";
import { Tile } from "./Tile";

/** Tempo de cada lembrete na dock, como o painel de cotações do widget de mercado. */
const ROTATE_MS = 5000;

export function NotesWidget({ notes, wide }: { notes: Note[]; wide: boolean }) {
  const t = useT().notes;
  const [index, setIndex] = useState(0);
  useEffect(() => {
    if (notes.length < 2) return;
    const id = window.setInterval(() => setIndex((current) => (current + 1) % notes.length), ROTATE_MS);
    return () => window.clearInterval(id);
  }, [notes.length]);

  if (!notes.length) {
    return (
      <Tile kind="notes" label={t.tileEmpty} className="notes-tile idle">
        <StickyNote size={wide ? 22 : 18} strokeWidth={1.8} aria-hidden />
        {wide && <span className="notes-idle-text">{t.tileEmpty}</span>}
      </Tile>
    );
  }

  const current = index % notes.length;
  const note = notes[current];
  const many = notes.length > 1;
  const dot = <span className={`note-dot ${note.color}`} />;
  const text = <span className="notes-text">{note.text}</span>;
  return (
    <Tile kind="notes" label={t.tileLabel(note.text)} className={`notes-tile ${wide ? "wide" : "small"}`}>
      <span key={note.id} className="notes-content">
        {wide ? (
          <>
            {dot}
            <span className={`notes-body${many ? " many" : ""}`}>
              {text}
              {many && <span className="notes-position">{t.position(current + 1, notes.length)}</span>}
            </span>
          </>
        ) : (
          <>
            <span className="notes-head">
              {dot}
              {many && <span className="notes-position">{`${current + 1}/${notes.length}`}</span>}
            </span>
            {text}
          </>
        )}
      </span>
    </Tile>
  );
}
