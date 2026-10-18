import { useRef, useState } from "react";

const clamp = (v: number) => Math.min(1, Math.max(0, v));
/** Intervalo mínimo entre envios enquanto arrasta: cada um vira um comando do sistema (pactl, osascript…). */
const SEND_EVERY_MS = 60;

/**
 * Barra de 0 a 1. É feita à mão porque, no macOS, o movimento do mouse nos painéis é sintetizado
 * (`pointerBridge`) e chega só como `mousemove` na janela, o que o `<input type="range">` nativo não
 * acompanha.
 */
export function Slider({ value, onChange, label, disabled }: { value: number; onChange: (value: number) => void; label: string; disabled?: boolean }) {
  const track = useRef<HTMLSpanElement>(null);
  const [drag, setDrag] = useState<number | null>(null);
  const lastSent = useRef(0);
  const shown = drag ?? value;

  const at = (clientX: number) => {
    const rect = track.current!.getBoundingClientRect();
    return clamp((clientX - rect.left) / rect.width);
  };

  const send = (v: number, force: boolean) => {
    const now = performance.now();
    if (force || now - lastSent.current >= SEND_EVERY_MS) {
      lastSent.current = now;
      onChange(v);
    }
  };

  const onMouseDown = (e: React.MouseEvent) => {
    if (disabled || e.button !== 0) return;
    e.preventDefault();
    const start = at(e.clientX);
    setDrag(start);
    send(start, true);
    const move = (ev: MouseEvent) => {
      const v = at(ev.clientX);
      setDrag(v);
      send(v, false);
    };
    const up = (ev: MouseEvent) => {
      send(at(ev.clientX), true);
      setDrag(null);
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };

  const step = (delta: number) => !disabled && onChange(clamp(Math.round((value + delta) * 100) / 100));

  return (
    <span
      ref={track}
      className={`slider ${disabled ? "disabled" : ""} ${drag !== null ? "dragging" : ""}`}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(shown * 100)}
      aria-disabled={disabled}
      onMouseDown={onMouseDown}
      onWheel={(e) => step(e.deltaY < 0 ? 0.04 : -0.04)}
      onKeyDown={(e) => {
        const delta = { ArrowRight: 0.05, ArrowUp: 0.05, ArrowLeft: -0.05, ArrowDown: -0.05 }[e.key];
        if (delta !== undefined) {
          e.preventDefault();
          step(delta);
        }
      }}
    >
      <span className="slider-track">
        <span className="slider-fill" style={{ width: `${shown * 100}%` }} />
      </span>
      <span className="slider-thumb" style={{ left: `${shown * 100}%` }} />
    </span>
  );
}
