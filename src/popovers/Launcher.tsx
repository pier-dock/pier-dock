import { useEffect, useMemo, useRef, useState } from "react";
import { Menu } from "@tauri-apps/api/menu";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { Search } from "lucide-react";
import { ipc, type AppEntry } from "../lib/ipc";
import { iconKeyForEntry } from "../dock/apps";
import { useIcon } from "../hooks/useIcon";
import { useConfig } from "../store/config";
import { useT } from "../i18n";
import { keys, os } from "../lib/platform";
import { ActionFooter } from "../ui/ActionFooter";
import { Kbd } from "../ui/Kbd";
import { Logo } from "../ui/Logo";
import { filterApps, matchesQuery } from "./launcherSearch";

/** Colunas da galeria de fixados; fixas para as setas ↑/↓ saberem o tamanho da linha. */
const COLUMNS = 6;

function AppIcon({ app, large = false }: { app: AppEntry; large?: boolean }) {
  const src = useIcon(iconKeyForEntry(app));
  const className = `launcher-icon${large ? " lg" : ""}`;
  return src ? <img className={className} src={src} alt="" /> : <span className={`${className} fallback`}>{app.name[0]}</span>;
}

/**
 * Lançador de apps na mesma janela de painel dos widgets: os fixados no lançador (lista própria,
 * separada da dock) em galeria, os demais instalados em lista, busca com foco ao abrir e atalhos
 * para abrir, fixar e abrir o menu do app.
 */
export function Launcher() {
  const config = useConfig((s) => s.config);
  const update = useConfig((s) => s.update);
  const t = useT();
  const [apps, setApps] = useState<AppEntry[] | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const body = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ipc
      .listInstalledApps()
      .then(setApps)
      .catch(() => setApps([]));
    input.current?.focus();
  }, []);

  const launcherPinned = config?.launcherPinned ?? [];
  const pinned = useMemo(() => new Set(launcherPinned.map((p) => p.id)), [launcherPinned]);
  const inDock = useMemo(() => new Set((config?.pinnedApps ?? []).map((p) => p.id)), [config?.pinnedApps]);
  // A galeria fica na ordem em que os apps foram fixados, não na alfabética.
  const pinnedResults = useMemo(() => launcherPinned.filter((a) => matchesQuery(a, query)), [launcherPinned, query]);
  const installedResults = useMemo(
    () => filterApps(apps ?? [], query).filter((a) => !pinned.has(a.id)),
    [apps, query, pinned],
  );
  const results = useMemo(() => [...pinnedResults, ...installedResults], [pinnedResults, installedResults]);
  const current = results[Math.min(selected, results.length - 1)];

  // A seleção sempre volta para o primeiro resultado quando a busca muda; a lista acompanha.
  useEffect(() => setSelected(0), [query]);
  useEffect(() => {
    body.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const open = (app: AppEntry) => {
    ipc.launchApp(app.target).catch(console.error);
    ipc.closePanel();
  };

  const togglePin = (app: AppEntry) =>
    update((c) => ({
      launcherPinned: pinned.has(app.id) ? c.launcherPinned.filter((p) => p.id !== app.id) : [...c.launcherPinned, app],
    }));

  const toggleDock = (app: AppEntry) =>
    update((c) => ({
      pinnedApps: inDock.has(app.id) ? c.pinnedApps.filter((p) => p.id !== app.id) : [...c.pinnedApps, app],
    }));

  const showMenu = async (app: AppEntry, at?: LogicalPosition) => {
    const menu = await Menu.new({
      items: [
        { id: "open", text: t.dock.open(app.name), action: () => open(app) },
        { id: "pin", text: pinned.has(app.id) ? t.launcher.unpin : t.launcher.pin, action: () => togglePin(app) },
        { id: "dock", text: inDock.has(app.id) ? t.dock.unpin : t.dock.pin, action: () => toggleDock(app) },
      ],
    });
    await menu.popup(at);
  };

  const onContextMenu = (e: React.MouseEvent, app: AppEntry) => {
    e.preventDefault();
    e.stopPropagation();
    showMenu(app);
  };

  // Pelo teclado o menu abre embaixo do item selecionado, não onde o ponteiro estiver.
  const showMenuForSelected = () => {
    if (!current) return;
    const rect = body.current?.querySelector('[aria-selected="true"]')?.getBoundingClientRect();
    showMenu(current, rect && new LogicalPosition(rect.left, rect.bottom));
  };

  const move = (key: string): number | null => {
    const last = results.length - 1;
    const inGallery = selected < pinnedResults.length;
    if (!inGallery) {
      if (key === "ArrowDown") return Math.min(selected + 1, last);
      if (key === "ArrowUp") return Math.max(selected - 1, 0);
      return null;
    }
    const lastTile = pinnedResults.length - 1;
    switch (key) {
      case "ArrowRight":
        return Math.min(selected + 1, last);
      case "ArrowLeft":
        return Math.max(selected - 1, 0);
      case "ArrowUp":
        return selected >= COLUMNS ? selected - COLUMNS : selected;
      case "ArrowDown":
        if (Math.floor(selected / COLUMNS) < Math.floor(lastTile / COLUMNS)) return Math.min(selected + COLUMNS, lastTile);
        return lastTile < last ? lastTile + 1 : selected;
      default:
        return null;
    }
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    const command = os === "macos" ? e.metaKey : e.ctrlKey;
    const next = move(e.key);
    if (next !== null) {
      e.preventDefault();
      setSelected(next);
    } else if (e.key === "Enter") {
      if (current) open(current);
    } else if (command && e.key.toLowerCase() === "p") {
      e.preventDefault();
      if (current) togglePin(current);
    } else if (command && e.key.toLowerCase() === "k") {
      e.preventDefault();
      showMenuForSelected();
    } else if (e.key === "Escape") {
      ipc.closePanel();
    }
  };

  const optionProps = (app: AppEntry, i: number) => ({
    role: "option",
    "aria-selected": i === selected,
    title: app.name,
    onMouseEnter: () => setSelected(i),
    onClick: () => open(app),
    onContextMenu: (e: React.MouseEvent) => onContextMenu(e, app),
  });

  return (
    <>
      <label className="search-field launcher-search">
        <Search size={18} aria-hidden />
        <span className="visually-hidden">{t.launcher.search}</span>
        <input
          ref={input}
          autoFocus
          value={query}
          placeholder={t.launcher.search}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
        />
        <Kbd keys="Esc" />
      </label>
      {results.length === 0 ? (
        apps === null ? (
          <p className="popover-loading">{t.launcher.loading}</p>
        ) : (
          <p className="popover-empty">{t.launcher.empty}</p>
        )
      ) : (
        <div className="launcher-body" role="listbox" aria-label={t.launcher.label} ref={body}>
          {pinnedResults.length > 0 && (
            <div role="group" aria-label={t.launcher.pinned}>
              <div className="eyebrow launcher-section" aria-hidden>
                {t.launcher.pinned}
              </div>
              <div className="launcher-gallery">
                {pinnedResults.map((app, i) => (
                  <button key={app.id} className="launcher-tile" {...optionProps(app, i)}>
                    <AppIcon app={app} large />
                    <span className="launcher-name">{app.name}</span>
                  </button>
                ))}
              </div>
            </div>
          )}
          {installedResults.length > 0 && (
            <div role="group" aria-label={t.launcher.installed}>
              <div className="eyebrow launcher-section" aria-hidden>
                {t.launcher.installed}
              </div>
              {installedResults.map((app, i) => (
                <button key={app.id} className="launcher-row" {...optionProps(app, pinnedResults.length + i)}>
                  <AppIcon app={app} />
                  <span className="launcher-name">{app.name}</span>
                  <span className="launcher-meta">{t.launcher.application}</span>
                </button>
              ))}
            </div>
          )}
          {apps === null && <p className="popover-loading launcher-loading">{t.launcher.loading}</p>}
        </div>
      )}
      <ActionFooter
        keyboard
        context={
          <span className="footer-source">
            <Logo size={16} />
            Pier
          </span>
        }
        actions={[
          { label: t.launcher.open, keys: "↵", onClick: () => current && open(current) },
          {
            label: current && pinned.has(current.id) ? t.launcher.unpin : t.launcher.pin,
            keys: `${keys.command} P`,
            onClick: () => current && togglePin(current),
          },
          { label: t.launcher.actions, keys: `${keys.command} K`, onClick: showMenuForSelected },
        ]}
      />
    </>
  );
}
