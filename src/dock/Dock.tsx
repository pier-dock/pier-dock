import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMotionValue } from "framer-motion";
import { LayoutGrid, Search } from "lucide-react";
import { DndContext, PointerSensor, useSensor, useSensors, type DragEndEvent, type DragOverEvent } from "@dnd-kit/core";
import { arrayMove, horizontalListSortingStrategy, SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { latestSender, sameHitRect } from "./hitRect";
import { Menu } from "@tauri-apps/api/menu";
import { setActiveInterval, setDockActive } from "../lib/dockActivity";
import { ipc, type AppEntry, type Config, type UpdateInfo, type WidgetConfig } from "../lib/ipc";
import { labels, os, widgetSupported } from "../lib/platform";
import { dict } from "../i18n";
import { useConfig } from "../store/config";
import { useWindows } from "../hooks/useWindows";
import { usePendingUpdate } from "../hooks/usePendingUpdate";
import { useSetupNeeded } from "../hooks/useSetupNeeded";
import { reuseDockApps } from "./appIdentity";
import { buildDockApps, type DockApp } from "./apps";
import { stableOpenApps } from "./runningOrder";
import { PREVIEW_GAP, barRect, dockMetrics, isVertical, previewSpace } from "./layout";
import { useFitIcon } from "./useFitIcon";
import { WindowPreview } from "./WindowPreview";
import { AgentPet } from "./AgentPet";
import { PokemonPet } from "./PokemonPet";
import { MagnifyCtx, MagnifyGeometry } from "./Magnify";
import { resetDockHover } from "./hoverReset";
import { AppIcon } from "./AppIcon";
import { SortableItem } from "./SortableItem";
import { DockUpdateProvider } from "./dockUpdate";
import { ProfilePeek } from "./ProfilePeek";
import { scopedUpdate, setWindowProfile } from "./profiles";
import { useProfileScroll } from "./useProfileScroll";
import { WidgetGroupSlot } from "./WidgetGroupSlot";
import { createGroup } from "./groups";
import { activeGroups, addDivider, applyOrder, groupKey, groupIdOf, isDivider, isGroupKey, LAUNCHER_KEY, removeItem, resolveOrder, START_KEY, widgetKey } from "./order";
import { DockWidget, DockWidgetsProvider } from "../widgets/DockWidgets";
import { togglePanel } from "../widgets/Tile";
import { Tooltip } from "../ui/Tooltip";

/** Espera antes de fechar a faixa quando o cursor sai da dock (ver o efeito do `mouseleave`). */
const PREVIEW_CLOSE_DELAY = 200;
/** Tempo segurando um widget sobre outro para criar o grupo, em vez de só reordenar. */
const GROUP_MERGE_DELAY = 350;

/** Algo que pede o usuário: configuração pendente do sistema ou versão nova. */
interface Attention {
  id: string;
  tone: "setup" | "update";
  label: string;
  open: () => void;
}

function attentionItems(setupNeeded: boolean, update: UpdateInfo | null): Attention[] {
  const t = dict().dock;
  return [
    // O aviso de configuração aparece em todas as abas.
    ...(setupNeeded ? [{ id: "setup", tone: "setup" as const, label: t.setupNeeded, open: () => void ipc.openSettings().catch(console.error) }] : []),
    // A instalação é confirmada na aba Geral.
    ...(update
      ? [{ id: "update", tone: "update" as const, label: t.updateAvailable(update.version), open: () => void ipc.openSettings("general").catch(console.error) }]
      : []),
  ];
}

const attentionMenuItems = (attention: Attention[]) => attention.map((a) => ({ id: a.id, text: a.label, action: a.open }));

/** Ponto no canto do botão de apps com o aviso mais importante; o menu do botão lista todos. */
function AttentionDot({ attention }: { attention: Attention }) {
  const [hover, setHover] = useState(false);
  const [focused, setFocused] = useState(false);
  return (
    <button
      className="attention-dot"
      data-tone={attention.tone}
      aria-label={attention.label}
      title={attention.label}
      onPointerDown={(e) => e.stopPropagation()}
      onClick={attention.open}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
    >
      {(hover || focused) && <Tooltip label={attention.label} />}
    </button>
  );
}

/** Abre o lançador de apps do sistema (Menu Iniciar, Launchpad, grade de apps do GNOME). */
function StartButton({ onRemove, attention }: { onRemove: () => void; attention: Attention[] }) {
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({ items: [...attentionMenuItems(attention), { id: "remove", text: dict().dock.removeFromDock, action: onRemove }] });
    await menu.popup();
  };
  return (
    <SortableItem id={START_KEY} className={attention.length ? "widget-slot has-attention" : "widget-slot"} onContextMenu={onContextMenu}>
      <button className="launcher-button" aria-label={labels().startButton} title={labels().startButton} onClick={() => ipc.openShell("start")}>
        <Search size={18} strokeWidth={2} aria-hidden />
      </button>
      {attention[0] && <AttentionDot attention={attention[0]} />}
    </SortableItem>
  );
}

/** Lançador de apps do Pier: abre a grade de instalados acima do botão (ver `popovers/Launcher`). */
function LauncherButton({ onRemove, attention }: { onRemove: () => void; attention: Attention[] }) {
  const ref = useRef<HTMLButtonElement>(null);
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({ items: [...attentionMenuItems(attention), { id: "remove", text: dict().dock.removeFromDock, action: onRemove }] });
    await menu.popup();
  };
  return (
    <SortableItem id={LAUNCHER_KEY} className={attention.length ? "widget-slot has-attention" : "widget-slot"} onContextMenu={onContextMenu}>
      <button
        ref={ref}
        className="launcher-button"
        aria-label={dict().dock.launcher}
        title={dict().dock.launcher}
        onClick={() => ref.current && togglePanel(LAUNCHER_KEY, ref.current)}
      >
        <LayoutGrid size={18} strokeWidth={2} aria-hidden />
      </button>
      {attention[0] && <AttentionDot attention={attention[0]} />}
    </SortableItem>
  );
}

function Divider({ id, onRemove }: { id: string; onRemove: () => void }) {
  const showMenu = async () => {
    const menu = await Menu.new({ items: [{ id: "remove", text: dict().dock.removeDivider, action: onRemove }] });
    await menu.popup();
  };
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    await showMenu();
  };
  return (
    <SortableItem id={id} className="divider-item" onContextMenu={onContextMenu}>
      <button className="divider-action" aria-label={dict().apps.divider} onClick={showMenu}>
        <span className="divider" />
      </button>
    </SortableItem>
  );
}

/** Clique num app da dock (também usado pelos atalhos Ctrl+Alt+N, ⌃⌥N no macOS). */
function clickApp(app: DockApp) {
  if (app.windows.length) ipc.toggleAppWindows(app.windows.map((w) => w.id));
  else ipc.launchApp(app.launchTarget).catch(console.error);
}

export function Dock({ config, profileId, monitor }: { config: Config; profileId: string; monitor: string | null }) {
  return <DockWidgetsProvider config={config}><DockLayout config={config} profileId={profileId} monitor={monitor} /></DockWidgetsProvider>;
}

const DockLayout = memo(function DockLayout({ config, profileId, monitor }: { config: Config; profileId: string; monitor: string | null }) {
  const storeUpdate = useConfig((s) => s.update);
  const profileRef = useRef(profileId);
  profileRef.current = profileId;
  // Estável entre trocas de perfil: o que assina eventos nativos com ele não precisa assinar de novo.
  const update = useCallback<typeof storeUpdate>((patch) => scopedUpdate(storeUpdate, profileRef.current)(patch), [storeUpdate]);
  const snapshot = useWindows();
  const pendingUpdate = usePendingUpdate();
  const setupNeeded = useSetupNeeded();
  const attention = attentionItems(setupNeeded, pendingUpdate);
  const position = config.dockPosition;
  const vertical = isVertical(position);
  const full = config.dockWidth === "full";
  const groupsList = useMemo(() => activeGroups(config), [config]);
  // O cursor na direção da barra: x nas bordas de cima e de baixo, y nas laterais.
  const mouse = useMotionValue(Infinity);
  const barRef = useRef<HTMLDivElement>(null);
  // Janela em prévia: guarda o app pelo `key`, porque a lista de apps é remontada a cada snapshot.
  const [preview, setPreview] = useState<{ key: string; x: number; y: number } | null>(null);
  const switchTo = useCallback((id: string) => {
    void storeUpdate((c) => setWindowProfile(c, monitor, id));
    setPreview(null);
    void ipc.closePanel().catch(console.error);
  }, [storeUpdate, monitor]);
  const profileScroll = useProfileScroll({
    barRef,
    enabled: config.profileScroll,
    invert: config.profileScrollInvert,
    vertical,
    animateLength: !full,
    profiles: config.dockProfiles,
    currentId: profileId,
    onSwitch: switchTo,
  });

  const [installed, setInstalled] = useState<AppEntry[]>([]);
  useEffect(() => {
    const refresh = () => ipc.listInstalledApps().then(setInstalled).catch(() => {});
    refresh();
    return setActiveInterval(refresh, 10 * 60_000);
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
    groupsList.length > 0,
  );
  const fit = useFitIcon(barRef, mouse, {
    iconSize: config.iconSize,
    magnification: Math.max(1, config.magnification),
    vertical,
    margin: full ? 0 : metrics.margin,
  });
  const magnifyContext = useMemo(() => ({ mouse, base: fit.icon, scale: fit.scale, position }), [mouse, fit.icon, fit.scale, position]);
  const previousApps = useRef<DockApp[]>([]);
  const apps = useMemo(
    () => {
      const next = buildDockApps(config.pinnedApps, snapshot.windows, config.showRunningUnpinned, installed);
      const ordered = stableOpenApps(previousApps.current, reuseDockApps(previousApps.current, next));
      previousApps.current = ordered;
      return ordered;
    },
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
    const radius = parseFloat(getComputedStyle(bar).borderTopLeftRadius) || 0;
    const sender = latestSender(ipc.setHitRect, sameHitRect, console.error);
    let frame: number | null = null;
    const report = () => {
      frame = null;
      const rect = barRect(position, full, metrics.margin, { width: bar.offsetWidth, height: bar.offsetHeight }, {
        width: window.innerWidth,
        height: window.innerHeight,
      });
      sender.submit({ x: rect.x, y: rect.y, width: rect.width, height: rect.height, radius });
    };
    const schedule = () => {
      if (frame === null) frame = requestAnimationFrame(report);
    };
    const observer = new ResizeObserver(schedule);
    observer.observe(bar);
    window.addEventListener("resize", schedule);
    schedule();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", schedule);
      if (frame !== null) cancelAnimationFrame(frame);
      sender.dispose();
    };
  }, [position, full, metrics.margin]);

  // O Rust decide quando a dock aparece (ao abrir, ao trocar de monitor, ocultar automaticamente);
  // aqui só anima. Ela começa fora da tela e entra quando o Rust manda.
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    // Na janela, não o `listen` global: esse recebe também o `emit_to` feito para as outras docks,
    // e a visibilidade de uma mexeria na barra da outra.
    const unlisten = getCurrentWebviewWindow().listen<boolean>("dock-visibility", (e) => {
      setVisible(e.payload);
      if (!e.payload) {
        mouse.set(Infinity);
        resetDockHover();
      }
    });
    // Só depois de registrar: um evento emitido antes disso se perderia e a barra ficaria escondida.
    unlisten.then(() => ipc.dockVisible()).then(setVisible);
    return () => {
      unlisten.then((f) => f());
    };
  }, [mouse]);
  useEffect(() => setDockActive(visible), [visible]);

  // Linux: o click-through recorta a região de input da janela, e o Rust só sabe do hover por aqui.
  useEffect(() => {
    if (os !== "linux") return;
    const root = document.documentElement;
    const enter = () => ipc.setDockHover(true);
    const leave = () => {
      ipc.setDockHover(false);
      mouse.set(Infinity);
      resetDockHover();
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
      getCurrentWebviewWindow().listen("dock-pointer-left", () => {
        mouse.set(Infinity);
        resetDockHover();
        setPreview(null);
      }),
      getCurrentWebviewWindow().listen<number>("shortcut-app", (e) => {
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

  const sensorOptions = useMemo(() => ({ activationConstraint: { distance: 8 } }), []);
  const sensors = useSensors(useSensor(PointerSensor, sensorOptions));
  const order = useMemo(() => resolveOrder(config), [config]);
  const groups = useMemo(() => new Map(groupsList.map((group) => [groupKey(group.id), group])), [groupsList]);

  const [merge, setMerge] = useState<{ key: string; ready: boolean } | null>(null);
  const mergeRef = useRef<{ key: string; ready: boolean } | null>(null);
  const mergeTimer = useRef<number | null>(null);
  const clearMerge = useCallback(() => {
    if (mergeTimer.current !== null) {
      window.clearTimeout(mergeTimer.current);
      mergeTimer.current = null;
    }
    mergeRef.current = null;
    setMerge(null);
  }, []);
  useEffect(() => () => {
    if (mergeTimer.current !== null) window.clearTimeout(mergeTimer.current);
  }, []);

  const onDragStart = () => {
    clearMerge();
    mouse.set(Infinity);
  };

  const onDragOver = ({ active, over }: DragOverEvent) => {
    const from = String(active.id);
    const kind = from.startsWith("widget:") ? from.slice("widget:".length) : null;
    if (!kind || !over) {
      clearMerge();
      return;
    }
    const target = String(over.id);
    if (target === from) {
      clearMerge();
      return;
    }
    if (isGroupKey(target)) {
      const group = groupsList.find((g) => g.id === groupIdOf(target));
      if (!group || group.members.some((member) => member === kind)) {
        clearMerge();
        return;
      }
    } else if (!target.startsWith("widget:")) {
      clearMerge();
      return;
    }
    if (mergeRef.current?.key === target) return;
    if (mergeTimer.current !== null) window.clearTimeout(mergeTimer.current);
    const pending = { key: target, ready: false };
    mergeRef.current = pending;
    setMerge(pending);
    mergeTimer.current = window.setTimeout(() => {
      if (mergeRef.current?.key !== target) return;
      const ready = { key: target, ready: true };
      mergeRef.current = ready;
      setMerge(ready);
      mergeTimer.current = null;
    }, GROUP_MERGE_DELAY);
  };

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const pending = mergeRef.current;
    clearMerge();
    if (!over || active.id === over.id) return;
    const activeKey = String(active.id);
    const overKey = String(over.id);
    if (pending?.ready && pending.key === overKey) {
      update((c) => createGroup(c, activeKey, overKey) ?? {});
      return;
    }
    update((c) => {
      const keys = resolveOrder(c);
      const from = keys.indexOf(activeKey);
      const to = keys.indexOf(overKey);
      if (from < 0 || to < 0) return {};
      return applyOrder(c, arrayMove(keys, from, to));
    });
  };

  const remove = (key: string) => update((c) => removeItem(c, key));

  // A faixa fecha quando o cursor sai dela e do ícone do app dela (para outro ícone, por exemplo).
  // A espera dá tempo de cruzar o vão entre o ícone e a faixa sem ela sumir no caminho.
  const previewLeaveTimer = useRef<number | null>(null);
  const keepPreview = useCallback(() => {
    if (previewLeaveTimer.current !== null) window.clearTimeout(previewLeaveTimer.current);
    previewLeaveTimer.current = null;
  }, []);
  const releasePreview = useCallback(() => {
    keepPreview();
    previewLeaveTimer.current = window.setTimeout(() => {
      previewLeaveTimer.current = null;
      setPreview(null);
    }, PREVIEW_CLOSE_DELAY);
  }, [keepPreview]);
  useEffect(() => keepPreview, [keepPreview]);
  const onPreviewHover = useCallback((inside: boolean) => (inside ? keepPreview() : releasePreview()), [keepPreview, releasePreview]);
  const onIconHover = useCallback((app: DockApp, inside: boolean) => {
    if (app.key === preview?.key) onPreviewHover(inside);
  }, [preview?.key, onPreviewHover]);

  // Só abre a faixa com mais de uma janela: com uma só o clique no ícone já resolve.
  const showPreview = useCallback((app: DockApp, anchor: { x: number; y: number }) => {
    keepPreview();
    setPreview(app.windows.length > 1 ? { key: app.key, ...anchor } : null);
  }, [keepPreview]);

  const onBackgroundMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    const t = dict().dock;
    const menu = await Menu.new({
      items: [
        ...attentionMenuItems(attention),
        ...(config.dockProfiles.length > 1
          ? [{
              text: t.profile(config.dockProfiles.find((p) => p.id === profileId)?.name ?? ""),
              items: [
                ...config.dockProfiles.map((p) => ({ id: `profile-${p.id}`, text: p.name, checked: p.id === profileId, action: () => switchTo(p.id) })),
                { item: "Separator" as const },
                { id: "edit-profiles", text: t.editProfiles, action: () => ipc.openSettings("profiles") },
              ],
            }]
          : [{ id: "edit-profiles", text: t.editProfiles, action: () => ipc.openSettings("profiles") }]),
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

  // Numa barra em pé os widgets usam o formato compacto: os largos são horizontais. O mapa de
  // contribuições não tem formato que caiba na espessura, então fica só com o total do ano.
  const renderWidget = (w: WidgetConfig) => (
    <DockWidget kind={w.kind} size={vertical ? (w.kind === "github-contrib" ? "count" : "small") : w.size} />
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

  // Um ponto só: no lançador do Pier, ou no botão Iniciar se o lançador estiver oculto. Sem
  // nenhum dos dois, os avisos ficam no menu do fundo da dock.
  const attentionHost = config.showLauncher ? LAUNCHER_KEY : START_KEY;
  const renderItem = (key: string) => {
    if (key === START_KEY) return <StartButton key={key} onRemove={() => remove(key)} attention={attentionHost === key ? attention : []} />;
    if (key === LAUNCHER_KEY) return <LauncherButton key={key} onRemove={() => remove(key)} attention={attentionHost === key ? attention : []} />;
    if (isDivider(key)) return <Divider key={key} id={key} onRemove={() => remove(key)} />;
    const group = groups.get(key);
    if (group) {
      return (
        <WidgetGroupSlot
          key={key}
          group={group}
          widgets={config.widgets}
          vertical={vertical}
          merge={merge?.key === key ? (merge.ready ? "ready" : "pending") : undefined}
        />
      );
    }
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
        <SortableItem key={key} id={key} className={`widget-slot${merge?.key === key ? (merge.ready ? " merge-ready" : " merge-pending") : ""}`}>
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
    ["--dock-thickness" as string]: `${metrics.barThickness}px`,
    // A espessura segue o ícone configurado (o layout no Rust é um só para todas as docks): o ícone
    // encolhido para caber no monitor continua centrado nela.
    ["--dock-item-offset" as string]: `${metrics.itemOffset + (config.iconSize - fit.icon) / 2}px`,
    ["--group-cell" as string]: `${metrics.groupCell}px`,
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
    <DockUpdateProvider value={update}>
    <MagnifyGeometry {...magnifyContext}>
    <div
      className="dock-root"
      data-pos={position}
      style={{
        ["--icon" as string]: `${fit.icon}px`,
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
        <MagnifyCtx.Provider value={magnifyContext}>
          <DndContext sensors={sensors} onDragStart={onDragStart} onDragOver={onDragOver} onDragCancel={clearMerge} onDragEnd={onDragEnd}>
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
        {config.dockProfiles.length > 1 && (
          <span className={`tooltip dock-profile-hud ${profileScroll.hudVisible ? "visible" : ""}`} role="status">
            <span className="tooltip-label">{config.dockProfiles.find((p) => p.id === profileId)?.name}</span>
            <span className="indicators" aria-hidden>
              {config.dockProfiles.map((p) => (
                <span key={p.id} className={`indicator ${p.id === profileId ? "active" : "on"}`} />
              ))}
            </span>
          </span>
        )}
      </div>
      {profileScroll.peekId && (
        <ProfilePeek config={config} profileId={profileScroll.peekId} barRef={barRef} barStyle={barStyle} vertical={vertical} renderWidget={renderWidget} />
      )}
      {config.claudePetEnabled && (
        <AgentPet
          mascot={config.agentPetMascot}
          pokemonId={config.agentPokemonId}
          position={position}
          full={full}
          margin={metrics.margin}
          visible={visible}
          barRef={barRef}
        />
      )}
      {config.pokemonPetEnabled &&
        config.pokemonPetIds.map((id, index) => (
          <PokemonPet key={id} id={id} index={index} count={config.pokemonPetIds.length} position={position} full={full} margin={metrics.margin} visible={visible} barRef={barRef} />
        ))}
      {previewApp && preview && (
        <WindowPreview app={previewApp} anchor={preview} position={position} onClose={() => setPreview(null)} onHoverChange={onPreviewHover} />
      )}
    </div>
    </MagnifyGeometry>
    </DockUpdateProvider>
  );
});
