import { useEffect, useMemo, useRef, useState } from "react";
import { useMotionValue } from "framer-motion";
import { Search } from "lucide-react";
import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { arrayMove, horizontalListSortingStrategy, SortableContext } from "@dnd-kit/sortable";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Menu } from "@tauri-apps/api/menu";
import { ipc, type AppEntry, type Config, type WidgetConfig } from "../lib/ipc";
import { labels, os, widgetSupported } from "../lib/platform";
import { dict } from "../i18n";
import { useConfig } from "../store/config";
import { useWindows } from "../hooks/useWindows";
import { useMedia } from "../hooks/useMedia";
import { useGithub } from "../hooks/useGithub";
import { useBluetooth, useSystemStatus } from "../hooks/useSystem";
import { buildDockApps, type DockApp } from "./apps";
import { dockMetrics } from "./layout";
import { MagnifyCtx } from "./Magnify";
import { AppIcon } from "./AppIcon";
import { SortableItem } from "./SortableItem";
import { addDivider, applyOrder, isDivider, removeItem, resolveOrder, START_KEY, widgetKey } from "./order";
import { WidgetView, type WidgetData } from "../widgets/WidgetView";
import { useWeather } from "../hooks/useWeather";
import { useCalendar } from "../hooks/useCalendar";
import { useClaudeUsage } from "../hooks/useClaudeUsage";

/** Abre o lançador de apps do sistema (Menu Iniciar, Launchpad, grade de apps do GNOME). */
function StartButton({ onRemove }: { onRemove: () => void }) {
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({ items: [{ id: "remove", text: dict().dock.removeFromDock, action: onRemove }] });
    await menu.popup();
  };
  return (
    <SortableItem id={START_KEY} className="widget-slot" onContextMenu={onContextMenu}>
      <button className="launcher-button" aria-label={labels().startButton} title={labels().startButton} onClick={() => ipc.openShell("start")}>
        <Search size={18} strokeWidth={2} aria-hidden />
      </button>
    </SortableItem>
  );
}

function Divider({ id, onRemove }: { id: string; onRemove: () => void }) {
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({ items: [{ id: "remove", text: dict().dock.removeDivider, action: onRemove }] });
    await menu.popup();
  };
  return (
    <SortableItem id={id} className="divider-item" onContextMenu={onContextMenu}>
      <div className="divider" />
    </SortableItem>
  );
}

/** Clique num app da dock (também usado pelos atalhos Ctrl+Alt+N, ⌃⌥N no macOS). */
function clickApp(app: DockApp) {
  if (app.windows.length) ipc.toggleAppWindows(app.windows.map((w) => w.id));
  else ipc.launchApp(app.launchTarget).catch(console.error);
}

const GITHUB_WIDGETS = ["github-review", "github-mine", "github-commits", "github-contrib"];

export function Dock({ config }: { config: Config }) {
  const update = useConfig((s) => s.update);
  const snapshot = useWindows();
  const media = useMedia();
  const usesGithub = config.widgets.some((w) => w.enabled && GITHUB_WIDGETS.includes(w.kind));
  const github = useGithub(usesGithub ? config.githubPollMinutes : undefined);
  const system = useSystemStatus();
  const enabled = (kind: string) => config.widgets.some((w) => w.enabled && w.kind === kind);
  const place = config.weatherLocation;
  const weather = useWeather(enabled("weather"), place ? `${place.latitude},${place.longitude}` : undefined);
  const calendar = useCalendar(enabled("calendar"));
  const [bluetooth] = useBluetooth(enabled("bluetooth"));
  const claude = useClaudeUsage(enabled("claude-usage") ? config.claudePollMinutes : undefined);
  const mouseX = useMotionValue(Infinity);
  const barRef = useRef<HTMLDivElement>(null);

  const [installed, setInstalled] = useState<AppEntry[]>([]);
  useEffect(() => {
    const refresh = () => ipc.listInstalledApps().then(setInstalled).catch(() => {});
    refresh();
    const id = setInterval(refresh, 10 * 60_000);
    return () => clearInterval(id);
  }, []);

  const metrics = dockMetrics(config.iconSize, config.magnification, config.dockWidth === "full");
  const apps = useMemo(
    () => buildDockApps(config.pinnedApps, snapshot.windows, config.showRunningUnpinned, installed),
    [config.pinnedApps, snapshot.windows, config.showRunningUnpinned, installed],
  );
  const appsRef = useRef(apps);
  appsRef.current = apps;

  // Reserva de tela + tamanho da janela no Rust.
  useEffect(() => {
    ipc.setDockLayout({
      reservedHeight: metrics.reservedHeight,
      windowHeight: metrics.windowHeight,
      hoverHeadroom: metrics.hoverHeadroom,
    });
  }, [metrics.reservedHeight, metrics.windowHeight, metrics.hoverHeadroom]);

  // Área clicável: fora dela a janela vira click-through.
  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    // Pelo layout, não pelo getBoundingClientRect: o transform da animação de ocultar não pode
    // deslocar a área clicável.
    const report = () => {
      const width = bar.offsetWidth;
      const height = bar.offsetHeight;
      const style = getComputedStyle(bar);
      const bottom = parseFloat(style.bottom) || 0;
      const radius = parseFloat(style.borderTopLeftRadius) || 0;
      ipc.setHitRect({ x: (window.innerWidth - width) / 2, y: window.innerHeight - bottom - height, width, height, radius });
    };
    const observer = new ResizeObserver(report);
    observer.observe(bar);
    window.addEventListener("resize", report);
    report();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", report);
    };
  }, []);

  // Ocultar automaticamente: o Rust decide quando a dock aparece; aqui só anima.
  const [visible, setVisible] = useState(true);
  useEffect(() => {
    ipc.dockVisible().then(setVisible);
    const unlisten = listen<boolean>("dock-visibility", (e) => {
      setVisible(e.payload);
      if (!e.payload) mouseX.set(Infinity);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [mouseX]);

  // Linux: o click-through recorta a região de input da janela, e o Rust só sabe do hover por aqui.
  useEffect(() => {
    if (os !== "linux") return;
    const root = document.documentElement;
    const enter = () => ipc.setDockHover(true);
    const leave = () => {
      ipc.setDockHover(false);
      mouseX.set(Infinity);
    };
    root.addEventListener("mouseenter", enter);
    root.addEventListener("mouseleave", leave);
    return () => {
      root.removeEventListener("mouseenter", enter);
      root.removeEventListener("mouseleave", leave);
    };
  }, [mouseX]);

  useEffect(() => {
    const subs = [
      // Quando o Rust liga o click-through, o WebView não recebe mouseleave.
      listen("dock-pointer-left", () => mouseX.set(Infinity)),
      listen<number>("shortcut-app", (e) => {
        const app = appsRef.current[e.payload];
        if (app) clickApp(app);
      }),
      // Arrastar um .exe/.lnk/.desktop/.app/pasta do gerenciador de arquivos para a dock fixa o item
      // (no fim da ordem).
      getCurrentWebview().onDragDropEvent(async (e) => {
        if (e.payload.type !== "drop") return;
        const entries = await Promise.all(e.payload.paths.map((p) => ipc.resolveAppPath(p)));
        update((c) => ({
          pinnedApps: [...c.pinnedApps, ...entries.filter((n) => !c.pinnedApps.some((p) => p.id === n.id))],
        }));
      }),
    ];
    return () => subs.forEach((s) => s.then((unlisten) => unlisten()));
  }, [mouseX, update]);

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 8 } }));
  const order = resolveOrder(config);

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    update((c) => {
      const keys = resolveOrder(c);
      const from = keys.indexOf(String(active.id));
      const to = keys.indexOf(String(over.id));
      if (from < 0 || to < 0) return {};
      return applyOrder(c, arrayMove(keys, from, to));
    });
  };

  const remove = (key: string) => update((c) => removeItem(c, key));

  const onBackgroundMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    const t = dict().dock;
    const menu = await Menu.new({
      items: [
        { id: "settings", text: t.dockSettings, action: () => ipc.openSettings() },
        { id: "divider", text: t.addDivider, action: () => update((c) => addDivider(c)) },
        ...(config.showStartButton
          ? []
          : [{ id: "start", text: t.showStartButton(labels().startButton), action: () => update({ showStartButton: true }) }]),
        { id: "reload", text: t.reload, action: () => location.reload() },
        { item: "Separator" },
        { id: "quit", text: labels().quit, action: () => ipc.quit() },
      ],
    });
    await menu.popup();
  };

  const widgetData: WidgetData = {
    media,
    github,
    system,
    bluetooth,
    weather,
    weatherConfigured: !!config.weatherLocation,
    calendar,
    claude,
  };
  const renderWidget = (w: WidgetConfig) => <WidgetView kind={w.kind} size={w.size} data={widgetData} />;

  const pinnedApps = new Map(apps.filter((a) => a.pinned).map((a) => [a.key, a]));
  const unpinnedApps = apps.filter((a) => !a.pinned);
  const widgets = new Map(config.widgets.filter((w) => w.enabled && widgetSupported(w.kind)).map((w) => [widgetKey(w.kind), w]));
  const activeKey = apps.find((a) => a.windows.some((w) => w.id === snapshot.foreground))?.key;
  const shortcutFor = (app: DockApp) => {
    const n = apps.indexOf(app) + 1;
    return n >= 1 && n <= 9 ? n : undefined;
  };

  const renderItem = (key: string) => {
    if (key === START_KEY) return <StartButton key={key} onRemove={() => remove(key)} />;
    if (isDivider(key)) return <Divider key={key} id={key} onRemove={() => remove(key)} />;
    const app = pinnedApps.get(key);
    if (app) return <AppIcon key={key} app={app} active={key === activeKey} sortable shortcut={shortcutFor(app)} />;
    const widget = widgets.get(key);
    if (widget) {
      return (
        <SortableItem key={key} id={key} className="widget-slot">
          {renderWidget(widget)}
        </SortableItem>
      );
    }
    return null;
  };

  // Clicar em qualquer outro lugar da dock fecha o painel aberto; o próprio tile alterna o dele.
  const onPointerDown = (e: React.PointerEvent) => {
    if (!(e.target as Element).closest("[data-widget]")) ipc.closePanel();
  };

  return (
    <div className="dock-root" style={{ ["--icon" as string]: `${config.iconSize}px` }}>
      <div
        ref={barRef}
        className={`dock-bar ${config.dockWidth === "full" ? "dock-full" : ""} ${visible ? "" : "dock-hidden"}`}
        data-glass
        style={{ bottom: config.dockWidth === "full" ? 0 : metrics.margin, height: metrics.barHeight, padding: `0 ${metrics.padding}px` }}
        onMouseMove={(e) => mouseX.set(e.clientX)}
        onMouseLeave={() => mouseX.set(Infinity)}
        onPointerDown={onPointerDown}
        onContextMenu={onBackgroundMenu}
      >
        <MagnifyCtx.Provider value={{ mouseX, base: config.iconSize, scale: Math.max(1, config.magnification) }}>
          <DndContext sensors={sensors} onDragStart={() => mouseX.set(Infinity)} onDragEnd={onDragEnd}>
            <SortableContext items={order} strategy={horizontalListSortingStrategy}>
              {order.map(renderItem)}
            </SortableContext>
          </DndContext>
          {unpinnedApps.map((app) => (
            <AppIcon key={app.key} app={app} active={app.key === activeKey} sortable={false} shortcut={shortcutFor(app)} />
          ))}
        </MagnifyCtx.Provider>
      </div>
    </div>
  );
}
