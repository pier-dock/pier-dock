import { useEffect, useRef, useState } from "react";
import { Plus, StickyNote, Trash2 } from "lucide-react";
import { ipc, MAX_NOTE_CHARS, NOTE_COLORS, type Note, type NoteColor } from "../lib/ipc";
import { useNotes } from "../hooks/useNotes";
import { ActionFooter } from "../ui/ActionFooter";
import { useT } from "../i18n";

const SAVE_DELAY_MS = 500;

/** Um lembrete editável: salva sozinho meio segundo depois da última tecla. */
function NoteCard({ note }: { note: Note }) {
  const { notes: t } = useT();
  const [text, setText] = useState(note.text);
  const [confirming, setConfirming] = useState(false);
  const timer = useRef<number>();
  const latest = useRef(text);

  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    if (!confirming) return;
    const id = window.setTimeout(() => setConfirming(false), 3000);
    return () => window.clearTimeout(id);
  }, [confirming]);

  const save = (value: string, color: NoteColor) => {
    if (!value.trim()) return;
    ipc.notesSave(note.id, value, color).catch(console.error);
  };
  const edit = (value: string) => {
    const next = value.slice(0, MAX_NOTE_CHARS);
    setText(next);
    latest.current = next;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => save(next, note.color), SAVE_DELAY_MS);
  };
  const remove = () => {
    if (!confirming) return setConfirming(true);
    window.clearTimeout(timer.current);
    ipc.notesDelete(note.id).catch(console.error);
  };

  return (
    <li className={`note-card ${note.color}`}>
      <textarea
        className="note-input"
        value={text}
        maxLength={MAX_NOTE_CHARS}
        aria-label={t.noteLabel}
        onChange={(e) => edit(e.target.value)}
        onBlur={() => {
          window.clearTimeout(timer.current);
          if (latest.current !== note.text) save(latest.current, note.color);
        }}
      />
      <div className="note-bar">
        <span className="note-colors">
          {NOTE_COLORS.map((color) => (
            <button
              key={color}
              type="button"
              className={`note-swatch ${color}${color === note.color ? " active" : ""}`}
              aria-label={t.colors[color]}
              aria-pressed={color === note.color}
              onClick={() => save(latest.current, color)}
            />
          ))}
        </span>
        <span className="note-count">{t.counter(text.length, MAX_NOTE_CHARS)}</span>
        <button type="button" className={`note-delete${confirming ? " confirm" : ""}`} onClick={remove} aria-label={confirming ? t.deleteConfirm : t.delete}>
          <Trash2 size={13} aria-hidden />
          {confirming && <span>{t.deleteConfirm}</span>}
        </button>
      </div>
    </li>
  );
}

export function Notes() {
  const notes = useNotes();
  const { notes: t } = useT();
  const create = () => ipc.notesSave(null, t.newText, "yellow").catch(console.error);

  return (
    <>
      <div className="market-header">
        <span className="eyebrow">{t.title}</span>
      </div>
      {notes.length ? (
        <ul className="notes-list" aria-label={t.title}>
          {notes.map((note) => (
            <NoteCard key={note.id} note={note} />
          ))}
        </ul>
      ) : (
        <div className="popover-empty">
          <span className="empty-icon">
            <StickyNote size={22} strokeWidth={1.8} />
          </span>
          <span className="empty-title">{t.empty}</span>
          <span className="popover-meta">{t.emptyHint}</span>
          <button className="action primary" onClick={create}>
            <Plus size={14} aria-hidden /> {t.add}
          </button>
        </div>
      )}
      <ActionFooter context={t.title} actions={[{ label: t.add, keys: "↵", onClick: create }]} />
    </>
  );
}
