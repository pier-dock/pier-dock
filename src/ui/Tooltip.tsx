import { useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { Kbd } from "./Kbd";

interface TooltipProps {
  label: string;
  keys?: string;
  /** Deixa o texto quebrar em várias linhas em vez de cortar com reticências. */
  wrap?: boolean;
}

/** Tooltip da dock, acima do item. Decorativo: o nome acessível fica no `aria-label` do botão. */
export function Tooltip({ label, keys, wrap = false }: TooltipProps) {
  const ref = useRef<HTMLSpanElement>(null);
  const [shift, setShift] = useState(0);

  // Com nomes longos a dica pode passar da janela da dock: puxa de volta para dentro da borda.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const edge = 8;
    const root = el.closest<HTMLElement>(".dock-root");
    const vertical = root?.dataset.pos === "left" || root?.dataset.pos === "right";
    let next = 0;
    if (vertical) {
      if (rect.top < edge) next = edge - rect.top;
      else if (rect.bottom > window.innerHeight - edge) next = window.innerHeight - edge - rect.bottom;
    } else {
      if (rect.left < edge) next = edge - rect.left;
      else if (rect.right > window.innerWidth - edge) next = window.innerWidth - edge - rect.right;
    }
    setShift(Math.round(next));
  }, [label, wrap]);

  return (
    <span
      ref={ref}
      className={`tooltip ${keys ? "with-keys" : ""} ${wrap ? "tooltip-wrap" : ""}`}
      style={{ "--tooltip-shift": `${shift}px` } as CSSProperties}
      aria-hidden
    >
      <span className="tooltip-label">{label}</span>
      {keys && <Kbd keys={keys} />}
    </span>
  );
}
