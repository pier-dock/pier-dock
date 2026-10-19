import { useRef, useState, type ReactNode } from "react";
import { ipc } from "../lib/ipc";
import { Tooltip } from "../ui/Tooltip";
import { PANEL_INSET, PANEL_SIZES } from "../popovers/sizes";

/** Abre (ou fecha, se já estiver aberto) o painel do widget, centrado em `el` e encostado na dock. */
export function togglePanel(kind: string, el: Element) {
  const rect = el.getBoundingClientRect();
  const [w, h] = PANEL_SIZES[kind] ?? [380, 420];
  ipc
    .openPanel(kind, rect.left + rect.width / 2, rect.top + rect.height / 2, w + 2 * PANEL_INSET, h + 2 * PANEL_INSET)
    .catch(console.error);
}

/** Mostra o tooltip enquanto o mouse está sobre o elemento. */
export function useHover() {
  const [hover, setHover] = useState(false);
  return { hover, bind: { onMouseEnter: () => setHover(true), onMouseLeave: () => setHover(false) } };
}

interface TileProps {
  kind: string;
  label: string;
  panel?: boolean;
  onClick?: (el: HTMLButtonElement) => void;
  onWheel?: (e: React.WheelEvent) => void;
  onContextMenu?: (e: React.MouseEvent) => void;
  className?: string;
  children: ReactNode;
}

/** Widget de um botão só: clicar abre o painel dele (ou faz `onClick`). */
export function Tile({ kind, label, panel = true, onClick, onWheel, onContextMenu, className = "", children }: TileProps) {
  const ref = useRef<HTMLButtonElement>(null);
  const { hover, bind } = useHover();

  const handleClick = () => {
    if (onClick && ref.current) return onClick(ref.current);
    if (panel && ref.current) togglePanel(kind, ref.current);
  };

  return (
    <button
      ref={ref}
      className={`tile ${className}`}
      data-widget
      aria-label={label}
      onClick={handleClick}
      onWheel={onWheel}
      onContextMenu={onContextMenu}
      {...bind}
    >
      {hover && <Tooltip label={label} />}
      {children}
    </button>
  );
}
