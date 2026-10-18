import { useEffect, useMemo, useRef, useState } from "react";
import { Menu } from "@tauri-apps/api/menu";
import { Pin, Search } from "lucide-react";
import { ipc, type AppEntry } from "../lib/ipc";
import { iconKeyForEntry } from "../dock/apps";
import { useIcon } from "../hooks/useIcon";
import { useConfig } from "../store/config";
import { useT } from "../i18n";
import { filterApps } from "./launcherSearch";

function AppIcon({ app }: { app: AppEntry }) {
  const src = useIcon(iconKeyForEntry(app));
  return src ? <img className="launcher-icon" src={src} alt="" /> : <span className="launcher-icon fallback">{app.name[0]}</span>;
}

/**
 * Lançador de apps na mesma janela de painel dos widgets: grade com todos os apps instalados,
 * busca com foco ao abrir, setas/Enter para abrir e "Fixar na dock" no botão direito.
 */
export function Launcher() {
  const config = useConfig((s) => s.config);
  const update = useConfig((s) => s.update);
  const t = useT();
  const [apps, setApps] = useState<AppEntry[] | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const grid = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ipc
      .listInstalledApps()
      .then(setApps)
      .catch(() => setApps([]));
    input.current?.focus();
  }, []);

  const results = useMemo(() => filterApps(apps ?? [], query), [apps, query]);
  const pinned = useMemo(() => new Set((config?.pinnedApps ?? []).map((p) => p.id)), [config?.pinnedApps]);

  // A seleção sempre volta para o primeiro resultado quando a busca muda; a lista acompanha.
  useEffect(() => setSelected(0), [query]);
  useEffect(() => {
    grid.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const open = (app: AppEntry) => {
    ipc.launchApp(app.target).catch(console.error);
    ipc.closePanel();
  };

  const togglePin = (app: AppEntry) =>
    update((c) => ({
      pinnedApps: pinned.has(app.id) ? c.pinnedApps.filter((p) => p.id !== app.id) : [...c.pinnedApps, app],
    }));

  const onContextMenu = async (e: React.MouseEvent, app: AppEntry) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({
      items: [
        { id: "open", text: t.dock.open(app.name), action: () => open(app) },
        { id: "pin", text: pinned.has(app.id) ? t.dock.unpin : t.dock.pin, action: () => togglePin(app) },
      ],
    });
    await menu.popup();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      const app = results[selected];
      if (app) open(app);
    } else if (e.key === "Escape") {
      ipc.closePanel();
    }
  };

  return (
    <>
      <label className="search-field launcher-search">
        <Search size={16} aria-hidden />
        <span className="visually-hidden">{t.launcher.search}</span>
        <input
          ref={input}
          autoFocus
          value={query}
          placeholder={t.launcher.search}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
        />
      </label>
      {apps === null ? (
        <p className="popover-loading">{t.launcher.loading}</p>
      ) : results.length === 0 ? (
        <p className="popover-empty">{t.launcher.empty}</p>
      ) : (
        <div className="launcher-grid" role="listbox" aria-label={t.launcher.label} ref={grid}>
          {results.map((app, i) => (
            <button
              key={app.id}
              className="launcher-app"
              role="option"
              aria-selected={i === selected}
              title={app.name}
              onMouseEnter={() => setSelected(i)}
              onClick={() => open(app)}
              onContextMenu={(e) => onContextMenu(e, app)}
            >
              <AppIcon app={app} />
              <span className="launcher-name">{app.name}</span>
              {pinned.has(app.id) && <Pin className="launcher-pin" size={12} aria-hidden />}
            </button>
          ))}
        </div>
      )}
    </>
  );
}
