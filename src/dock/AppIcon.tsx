import { useContext, useEffect, useRef, useState } from "react";
import { motion, useAnimationControls, useMotionValueEvent } from "framer-motion";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Menu } from "@tauri-apps/api/menu";
import { ipc } from "../lib/ipc";
import { keys } from "../lib/platform";
import { useIcon } from "../hooks/useIcon";
import { useConfig } from "../store/config";
import { Tooltip } from "../ui/Tooltip";
import { Magnify, MagnifyCtx } from "./Magnify";
import { toPinned, type DockApp } from "./apps";
import { useT } from "../i18n";

/** Espera antes de abrir a faixa de prévias: passar o mouse para o lado não deve abrir nada. */
const PREVIEW_DELAY = 320;

interface Props {
  app: DockApp;
  active: boolean;
  sortable: boolean;
  /** Posição para o atalho Ctrl+Alt+N (⌃⌥N no macOS, 1 a 9), se o app tiver um. */
  shortcut?: number;
  /** Abre a faixa de prévias das janelas depois de um instante de hover. */
  onPreview?: (app: DockApp, centerX: number) => void;
}

export function AppIcon({ app, active, sortable, shortcut, onPreview }: Props) {
  const src = useIcon(app.iconKey);
  const bounce = useAnimationControls();
  const [hover, setHover] = useState(false);
  const previewTimer = useRef<number | null>(null);
  const cancelPreview = () => {
    if (previewTimer.current !== null) window.clearTimeout(previewTimer.current);
    previewTimer.current = null;
  };
  // Quando o Rust liga o click-through ou a dock se esconde, o WebView não recebe mouseleave; a
  // dock sinaliza a saída do cursor zerando o mouseX.
  useMotionValueEvent(useContext(MagnifyCtx)!.mouseX, "change", (x) => {
    if (x !== Infinity) return;
    setHover(false);
    // Arrastar ou esconder a dock também cancela a espera da prévia.
    cancelPreview();
  });
  useEffect(() => cancelPreview, []);
  const update = useConfig((s) => s.update);
  const t = useT().dock;
  const { listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: app.key,
    disabled: !sortable,
  });

  const launch = () => {
    bounce.start({ y: [0, -14, 0, -6, 0], transition: { duration: 0.7 } });
    ipc.launchApp(app.launchTarget).catch(console.error);
  };

  const onClick = (e: React.MouseEvent) => {
    if (e.shiftKey || app.windows.length === 0) launch();
    else ipc.toggleAppWindows(app.windows.map((w) => w.id));
  };

  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const windowItems = app.windows.slice(0, 10).map((w) => ({
      id: `win-${w.id}`,
      text: w.title.length > 60 ? `${w.title.slice(0, 57)}…` : w.title,
      action: () => ipc.activateWindow(w.id),
    }));
    const pinItem = app.pinned
      ? {
          id: "unpin",
          text: t.unpin,
          action: () => update((c) => ({ pinnedApps: c.pinnedApps.filter((p) => p.id !== app.pinned!.id) })),
        }
      : { id: "pin", text: t.pin, action: () => update((c) => ({ pinnedApps: [...c.pinnedApps, toPinned(app)] })) };
    const menu = await Menu.new({
      items: [
        ...windowItems,
        ...(windowItems.length ? [{ item: "Separator" as const }] : []),
        { id: "new", text: app.windows.length ? t.newWindow : t.open(app.name), action: launch },
        pinItem,
        ...(app.windows.length
          ? [
              {
                id: "close",
                text: app.windows.length > 1 ? t.closeWindows(app.windows.length) : t.closeWindow,
                action: () => ipc.closeWindows(app.windows.map((w) => w.id)),
              },
            ]
          : []),
      ],
    });
    await menu.popup();
  };

  const windows = app.windows.length;
  const state = active ? t.focused : windows > 1 ? t.windows(windows) : windows ? t.running : "";

  const onMouseEnter = (e: React.MouseEvent<HTMLDivElement>) => {
    setHover(true);
    if (!onPreview || app.windows.length < 2 || isDragging) return;
    const el = e.currentTarget;
    previewTimer.current = window.setTimeout(() => {
      previewTimer.current = null;
      const rect = el.getBoundingClientRect();
      onPreview(app, rect.left + rect.width / 2);
    }, PREVIEW_DELAY);
  };

  const onMouseLeave = () => {
    setHover(false);
    cancelPreview();
  };

  return (
    <div
      ref={setNodeRef}
      className={`dock-item ${isDragging ? "dragging" : ""}`}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {hover && !isDragging && <Tooltip label={app.name} keys={shortcut ? `${keys.appShortcut} ${shortcut}` : undefined} />}
      <button
        className={`app-button ${hover && !isDragging ? "hovered" : ""}`}
        aria-label={state ? `${app.name} (${state})` : app.name}
        onClick={onClick}
        onContextMenu={onContextMenu}
        {...listeners}
      >
        <Magnify>
          <motion.span className="app-icon" animate={bounce}>
            {src ? <img src={src} alt="" draggable={false} /> : <span className="app-icon-fallback">{app.name[0]}</span>}
          </motion.span>
        </Magnify>
        <span className="indicators" aria-hidden>
          <span className={`indicator ${active ? "active" : windows ? "on" : ""}`} />
          {windows > 1 && <span className="indicator on" />}
        </span>
      </button>
    </div>
  );
}
