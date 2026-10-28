import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { Check, ChevronDown } from "lucide-react";
import { ipc, type OutputDevice } from "../lib/ipc";

/** Keep choices in this webview: native select popups can make the widget lose native focus. */
export function OutputPicker({ label, placeholder, outputs, value, disabled, onChoose }: {
  label: string;
  placeholder: string;
  outputs: OutputDevice[];
  value: string | null;
  disabled: boolean;
  onChoose: (output: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const pointerDown = useRef(false);
  const options = useRef(new Map<string, HTMLButtonElement>());
  const current = outputs.find((output) => output.id === value);
  const initialId = current?.id ?? outputs[0]?.id;

  const close = (restoreFocus: boolean) => {
    setOpen(false);
    if (restoreFocus) trigger.current?.focus();
  };
  const expand = () => {
    if (disabled || !outputs.length) return;
    setOpen(true);
    // Enable keyboard input on platforms where the dock's panels initially do not receive it.
    void ipc.focusPanel().catch(console.error);
  };
  useEffect(() => {
    if (open && initialId) options.current.get(initialId)?.focus();
  }, [open, initialId]);
  useEffect(() => {
    if (!open) return;
    const startPointer = () => { pointerDown.current = true; };
    const endPointer = () => {
      pointerDown.current = false;
      // A cancelled or released drag may not produce a click. Wait until the
      // current pointer event finishes before checking whether focus left.
      window.setTimeout(() => {
        if (root.current && !root.current.contains(document.activeElement)) setOpen(false);
      }, 0);
    };
    const outside = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    // Closing on mousedown can move another app's picker before its click lands.
    window.addEventListener("pointerdown", startPointer, true);
    window.addEventListener("pointerup", endPointer, true);
    window.addEventListener("pointercancel", endPointer, true);
    window.addEventListener("click", outside);
    return () => {
      window.removeEventListener("pointerdown", startPointer, true);
      window.removeEventListener("pointerup", endPointer, true);
      window.removeEventListener("pointercancel", endPointer, true);
      window.removeEventListener("click", outside);
      pointerDown.current = false;
    };
  }, [open]);

  useEffect(() => {
    if (disabled || !outputs.length) setOpen(false);
  }, [disabled, outputs.length]);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation(); // First Escape dismisses the picker, not the entire panel.
      close(true);
      return;
    }
    if (!outputs.length || !["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    if (!open && event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    event.stopPropagation();
    if (!open) { expand(); return; }
    const index = outputs.findIndex((output) => options.current.get(output.id) === document.activeElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? outputs.length - 1
      : (index + (event.key === "ArrowDown" ? 1 : -1) + outputs.length) % outputs.length;
    options.current.get(outputs[next]?.id)?.focus();
  };

  return (
    <div ref={root} className="sound-output-picker" onKeyDown={onKeyDown}
      onBlur={(event) => { if (!pointerDown.current && !event.currentTarget.contains(event.relatedTarget as Node | null)) setOpen(false); }}>
      <button ref={trigger} type="button" className="sound-app-output" aria-label={label} aria-describedby={`${id}-value`}
        aria-haspopup="listbox" aria-expanded={open} aria-controls={open ? id : undefined} disabled={disabled || !outputs.length}
        onClick={() => open ? close(false) : expand()}>
        <span id={`${id}-value`}>{current?.name ?? placeholder}</span>
        <ChevronDown size={12} aria-hidden />
      </button>
      {open && <div id={id} role="listbox" aria-label={label} className="sound-output-options">
        {outputs.map((output) => <button key={output.id} type="button" role="option" aria-selected={output.id === value}
          className="sound-output-option" disabled={disabled} tabIndex={-1}
          ref={(element) => { if (element) options.current.set(output.id, element); else options.current.delete(output.id); }}
          onClick={() => { close(true); if (output.id !== value) onChoose(output.id); }}>
          <span>{output.name}</span>
          {output.id === value && <Check size={13} aria-hidden />}
        </button>)}
      </div>}
    </div>
  );
}
