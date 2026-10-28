import { useRef, useState, type ReactNode } from "react";
import { ipc } from "../lib/ipc";
import { currentView } from "../lib/view";
import { Tooltip } from "../ui/Tooltip";
import { PANEL_INSET, panelSize } from "../popovers/sizes";
import { useDockHoverReset } from "../dock/hoverReset";

/** Centro em que o painel deve abrir: o cartão do grupo, se foi ele que abriu o painel atual. */
export function panelAnchor(el: Element) {
  const rect = el.getBoundingClientRect();
  if (currentView.view === "panel" && currentView.kind.startsWith("group:") && currentView.anchor) return currentView.anchor;
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

/** Abre (ou fecha, se já estiver aberto) o painel do widget, ancorado em `el` e encostado na dock. */
export function togglePanel(kind: string, el: Element, size?: [number, number]) {
  const anchor = panelAnchor(el);
  const [w, h] = size ?? panelSize(kind);
  ipc
    .openPanel(kind, anchor.x, anchor.y, w + 2 * PANEL_INSET, h + 2 * PANEL_INSET)
    .catch(console.error);
}

/** Mostra o tooltip enquanto o mouse está sobre o elemento. */
export function useHover() {
  const [hover, setHover] = useState(false);
  // Com a dock escondida ou com click-through, o mouseleave não chega; a dock avisa a saída.
  const claimHover = useDockHoverReset(() => setHover(false));
  const onMouseEnter = () => {
    setHover(true);
    claimHover();
  };
  return { hover, bind: { onMouseEnter, onMouseLeave: () => setHover(false) } };
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
