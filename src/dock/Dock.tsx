import { useEffect, useMemo, useRef, useState } from "react";
import { useMotionValue } from "framer-motion";
import { LayoutGrid, Search } from "lucide-react";
import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { arrayMove, horizontalListSortingStrategy, SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
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
import { PREVIEW_GAP, barRect, dockMetrics, isVertical, previewSpace } from "./layout";
import { WindowPreview } from "./WindowPreview";
import { ClawdPet } from "./ClawdPet";
import { MagnifyCtx } from "./Magnify";
import { AppIcon } from "./AppIcon";
import { SortableItem } from "./SortableItem";
import { addDivider, applyOrder, isDivider, LAUNCHER_KEY, removeItem, resolveOrder, START_KEY, widgetKey } from "./order";
import { WidgetView, type WidgetData } from "../widgets/WidgetView";
import { togglePanel } from "../widgets/Tile";
import { useWeather } from "../hooks/useWeather";
import { useCalendar } from "../hooks/useCalendar";
import { useAiUsage } from "../hooks/useAiUsage";

/** Espera antes de fechar a faixa quando o cursor sai da dock (ver o efeito do `mouseleave`). */
const PREVIEW_CLOSE_DELAY = 200;

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

/** Lançador de apps do Pier: abre a grade de instalados acima do botão (ver `popovers/Launcher`). */
function LauncherButton({ onRemove }: { onRemove: () => void }) {
  const ref = useRef<HTMLButtonElement>(null);
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({ items: [{ id: "remove", text: dict().dock.removeFromDock, action: onRemove }] });
    await menu.popup();
  };
  return (
    <SortableItem id={LAUNCHER_KEY} className="widget-slot" onContextMenu={onContextMenu}>
      <button
        ref={ref}
        className="launcher-button"
        aria-label={dict().dock.launcher}
        title={dict().dock.launcher}
        onClick={() => ref.current && togglePanel(LAUNCHER_KEY, ref.current)}
      >
        <LayoutGrid size={18} strokeWidth={2} aria-hidden />
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
  const ai = useAiUsage(enabled("ai-usage") ? config.aiPollMinutes : undefined, config.aiProvider);
  const position = config.dockPosition;
  const vertical = isVertical(position);
  const full = config.dockWidth === "full";
  // O cursor na direção da barra: x nas bordas de cima e de baixo, y nas laterais.
  const mouse = useMotionValue(Infinity);
  const barRef = useRef<HTMLDivElement>(null);
  // Janela em prévia: guarda o app pelo `key`, porque a lista de apps é remontada a cada snapshot.
  const [preview, setPreview] = useState<{ key: string; x: number; y: number } | null>(null);

  const [installed, setInstalled] = useState<AppEntry[]>([]);
  useEffect(() => {
    const refresh = () => ipc.listInstalledApps().then(setInstalled).catch(() => {});
    refresh();
    const id = setInterval(refresh, 10 * 60_000);
    return () => clearInterval(id);
  }, []);

  // Com a faixa aberta a área clicável avança até ela, senão o cursor não a alcança (app nenhum
  // fica por cima da área reservada da dock). A janela já tem espaço para a faixa (`dockMetrics`).
  const metrics = dockMetrics(
    config.iconSize,
    config.magnification,
    full,
    config.dockHeight,
    previewSpace(position, !!preview),
    position,
  );
  const apps = useMemo(
    () => buildDockApps(config.pinnedApps, snapshot.windows, config.showRunningUnpinned, installed),
    [config.pinnedApps, snapshot.windows, config.showRunningUnpinned, installed],
  );
  const appsRef = useRef(apps);
  appsRef.current = apps;

  // O snapshot de janelas muda por conta própria: a faixa fecha quando o app some ou fica com
  // menos de duas janelas (as demais passam a caber no clique simples do ícone).
  useEffect(() => {
    setPreview((p) => (p && apps.some((a) => a.key === p.key && a.windows.length > 1) ? p : null));
  }, [apps]);

  // Reserva de tela + tamanho da janela no Rust.
  useEffect(() => {
    ipc.setDockLayout({
      reserved: metrics.reserved,
      windowSize: metrics.windowSize,
      hoverHeadroom: metrics.hoverHeadroom,
    });
  }, [metrics.reserved, metrics.windowSize, metrics.hoverHeadroom]);

  // Área clicável: fora dela a janela vira click-through.
  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    // Pelo layout, não pelo getBoundingClientRect: o transform da animação de ocultar não pode
    // deslocar a área clicável.
    const report = () => {
      const style = getComputedStyle(bar);
      const rect = barRect(position, full, metrics.margin, { width: bar.offsetWidth, height: bar.offsetHeight }, {
        width: window.innerWidth,
        height: window.innerHeight,
      });
      const radius = parseFloat(style.borderTopLeftRadius) || 0;
      ipc.setHitRect({ x: rect.x, y: rect.y, width: rect.width, height: rect.height, radius });
    };
    const observer = new ResizeObserver(report);
    observer.observe(bar);
    window.addEventListener("resize", report);
    report();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", report);
    };
  }, [position, full, metrics.margin]);

  // O Rust decide quando a dock aparece (ao abrir, ao trocar de monitor, ocultar automaticamente);
  // aqui só anima. Ela começa fora da tela e entra quando o Rust manda.
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    ipc.dockVisible().then(setVisible);
    const unlisten = listen<boolean>("dock-visibility", (e) => {
      setVisible(e.payload);
      if (!e.payload) mouse.set(Infinity);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [mouse]);

  // Linux: o click-through recorta a região de input da janela, e o Rust só sabe do hover por aqui.
  useEffect(() => {
    if (os !== "linux") return;
    const root = document.documentElement;
    const enter = () => ipc.setDockHover(true);
    const leave = () => {
      ipc.setDockHover(false);
      mouse.set(Infinity);
    };
    root.addEventListener("mouseenter", enter);
    root.addEventListener("mouseleave", leave);
    return () => {
      root.removeEventListener("mouseenter", enter);
      root.removeEventListener("mouseleave", leave);
    };
  }, [mouse]);

  // Sair da dock fecha a faixa de prévias. O evento fica no documento porque o cursor vai do ícone
  // para a faixa sem passar pela barra, e o mouseleave do ícone não pode fechar a faixa. A espera
  // curta é para o X11, que manda um leave em volta do redimensionamento da janela (que cresce
  // para caber a faixa): sem ela a faixa fecharia no instante em que abre.
  useEffect(() => {
    const root = document.documentElement;
    let timer: number | null = null;
    const cancel = () => {
      if (timer !== null) window.clearTimeout(timer);
      timer = null;
    };
    const leave = () => {
      cancel();
      timer = window.setTimeout(() => setPreview(null), PREVIEW_CLOSE_DELAY);
    };
    root.addEventListener("mouseleave", leave);
    root.addEventListener("mouseenter", cancel);
    root.addEventListener("mousemove", cancel);
    return () => {
      cancel();
      root.removeEventListener("mouseleave", leave);
      root.removeEventListener("mouseenter", cancel);
      root.removeEventListener("mousemove", cancel);
    };
  }, []);

  useEffect(() => {
    const subs = [
      // Quando o Rust liga o click-through, o WebView não recebe mouseleave.
      listen("dock-pointer-left", () => {
        mouse.set(Infinity);
        setPreview(null);
      }),
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
  }, [mouse, update]);

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

  // A faixa fecha quando o cursor sai dela e do ícone do app dela (para outro ícone, por exemplo).
  // A espera dá tempo de cruzar o vão entre o ícone e a faixa sem ela sumir no caminho.
  const previewLeaveTimer = useRef<number | null>(null);
  const keepPreview = () => {
    if (previewLeaveTimer.current !== null) window.clearTimeout(previewLeaveTimer.current);
    previewLeaveTimer.current = null;
  };
  const releasePreview = () => {
    keepPreview();
    previewLeaveTimer.current = window.setTimeout(() => {
      previewLeaveTimer.current = null;
      setPreview(null);
    }, PREVIEW_CLOSE_DELAY);
  };
  useEffect(() => keepPreview, []);
  const onPreviewHover = (inside: boolean) => (inside ? keepPreview() : releasePreview());
  const onIconHover = (app: DockApp, inside: boolean) => {
    if (app.key === preview?.key) onPreviewHover(inside);
  };

  // Só abre a faixa com mais de uma janela: com uma só o clique no ícone já resolve.
  const showPreview = (app: DockApp, anchor: { x: number; y: number }) => {
    keepPreview();
    setPreview(app.windows.length > 1 ? { key: app.key, ...anchor } : null);
  };

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
        ...(config.showLauncher ? [] : [{ id: "launcher", text: t.showLauncher, action: () => update({ showLauncher: true }) }]),
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
    ai,
  };
  // Numa barra em pé os widgets usam o formato compacto: os largos são horizontais. O mapa de
  // contribuições não tem formato que caiba na espessura, então fica só com o total do ano.
  const renderWidget = (w: WidgetConfig) => (
    <WidgetView kind={w.kind} size={vertical ? (w.kind === "github-contrib" ? "count" : "small") : w.size} data={widgetData} />
  );

  const pinnedApps = new Map(apps.filter((a) => a.pinned).map((a) => [a.key, a]));
  const unpinnedApps = apps.filter((a) => !a.pinned);
  const widgets = new Map(config.widgets.filter((w) => w.enabled && widgetSupported(w.kind)).map((w) => [widgetKey(w.kind), w]));
  const activeKey = apps.find((a) => a.windows.some((w) => w.id === snapshot.foreground))?.key;
  const previewApp = preview ? apps.find((a) => a.key === preview.key) : undefined;
  // A faixa fica logo depois da barra, para dentro da tela; o tamanho da janela (`windowSize`) já
  // reserva o espaço.
  const previewOffset = (full ? 0 : metrics.margin) + metrics.barThickness + PREVIEW_GAP;
  const shortcutFor = (app: DockApp) => {
    const n = apps.indexOf(app) + 1;
    return n >= 1 && n <= 9 ? n : undefined;
  };

  const renderItem = (key: string) => {
    if (key === START_KEY) return <StartButton key={key} onRemove={() => remove(key)} />;
    if (key === LAUNCHER_KEY) return <LauncherButton key={key} onRemove={() => remove(key)} />;
    if (isDivider(key)) return <Divider key={key} id={key} onRemove={() => remove(key)} />;
    const app = pinnedApps.get(key);
    if (app)
      return (
        <AppIcon
          key={key}
          app={app}
          active={key === activeKey}
          sortable
          shortcut={shortcutFor(app)}
          onPreview={showPreview}
          onHoverChange={onIconHover}
        />
      );
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

  // A barra encosta na borda da tela no eixo da espessura e fica centrada no eixo do comprimento
  // (de ponta a ponta ela ocupa a borda inteira).
  const barStyle: React.CSSProperties = {
    ["--dock-item-offset" as string]: `${metrics.itemOffset}px`,
  };
  if (vertical) {
    barStyle[position === "left" ? "left" : "right"] = full ? 0 : metrics.margin;
    barStyle.width = metrics.barThickness;
    barStyle.padding = `${metrics.padding}px 0`;
  } else {
    barStyle[position === "top" ? "top" : "bottom"] = full ? 0 : metrics.margin;
    barStyle.height = metrics.barThickness;
    barStyle.padding = `0 ${metrics.padding}px`;
  }

  return (
    <div
      className="dock-root"
      data-pos={position}
      style={{
        ["--icon" as string]: `${config.iconSize}px`,
        ["--preview-offset" as string]: `${previewOffset}px`,
      }}
    >
      <div
        ref={barRef}
        className={`dock-bar ${full ? "dock-full" : ""} ${visible ? "" : "dock-hidden"}`}
        data-glass
        style={barStyle}
        onMouseMove={(e) => mouse.set(vertical ? e.clientY : e.clientX)}
        onMouseLeave={() => mouse.set(Infinity)}
        onPointerDown={onPointerDown}
        onContextMenu={onBackgroundMenu}
      >
        <MagnifyCtx.Provider value={{ mouse, base: config.iconSize, scale: Math.max(1, config.magnification), position }}>
          <DndContext sensors={sensors} onDragStart={() => mouse.set(Infinity)} onDragEnd={onDragEnd}>
            <SortableContext items={order} strategy={vertical ? verticalListSortingStrategy : horizontalListSortingStrategy}>
              {order.map(renderItem)}
            </SortableContext>
          </DndContext>
          {unpinnedApps.map((app) => (
            <AppIcon
              key={app.key}
              app={app}
              active={app.key === activeKey}
              sortable={false}
              shortcut={shortcutFor(app)}
              onPreview={showPreview}
              onHoverChange={onIconHover}
            />
          ))}
        </MagnifyCtx.Provider>
      </div>
      {config.claudePetEnabled && <ClawdPet position={position} full={full} margin={metrics.margin} visible={visible} barRef={barRef} />}
      {previewApp && preview && (
        <WindowPreview app={previewApp} anchor={preview} position={position} onClose={() => setPreview(null)} onHoverChange={onPreviewHover} />
      )}
    </div>
  );
}
