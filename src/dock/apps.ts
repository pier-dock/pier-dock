import type { AppEntry, WindowInfo } from "../lib/ipc";
import { prettyExeName } from "../lib/format.ts";
import { os } from "../lib/platform.ts";
import { parseTitleCount } from "./badge.ts";
import { appKey } from "./order.ts";

export interface DockApp {
  key: string;
  name: string;
  pinned?: AppEntry;
  windows: WindowInfo[];
  iconKey: string;
  /** O que abrir quando o app não tem janelas (ou para "nova janela"). */
  launchTarget: string;
  /** App instalado correspondente, para fixar com os dados certos. */
  installed?: AppEntry;
  /** Notificações pendentes somadas das janelas do app; 0 quando não há. */
  badge: number;
}

const lower = (s?: string | null) => (s ?? "").toLowerCase();

function iconKeyForWindow(w: WindowInfo): string {
  // Linux: o desktop id dá o ícone do tema; macOS: o exe já é o bundle .app.
  if (os === "linux") return w.aumid ?? w.exe;
  if (os === "macos") return w.exe;
  // Janelas UWP pertencem ao ApplicationFrameHost; o ícone certo vem do AUMID.
  if (w.aumid && (w.aumid.includes("!") || lower(w.exe).endsWith("applicationframehost.exe"))) {
    return `shell:AppsFolder\\${w.aumid}`;
  }
  return w.exe;
}

/**
 * De onde extrair o ícone de um app fixado/instalado. Para apps Win32 o item do AppsFolder só tem
 * um ícone pequeno, então o .exe dá um resultado bem melhor; UWP e PWAs (cujo exe é o do
 * navegador) precisam do AppsFolder.
 */
export function iconKeyForEntry(entry: AppEntry): string {
  if (os === "linux") return entry.aumid ?? entry.target;
  if (os === "macos") return entry.target;
  const aumid = lower(entry.aumid);
  const isPackagedOrPwa = aumid.includes("!") || aumid.includes("_crx_");
  if (entry.exe && !isPackagedOrPwa && lower(entry.exe).endsWith(".exe")) return entry.exe;
  return entry.target;
}

function findPinned(pinned: AppEntry[], w: WindowInfo): number {
  if (w.aumid) {
    const byAumid = pinned.findIndex((p) => p.aumid && lower(p.aumid) === lower(w.aumid));
    if (byAumid >= 0) return byAumid;
    // PWAs (YouTube Music instalado pelo Chrome/Edge) rodam no chrome.exe/msedge.exe, mas são
    // apps próprios: não podem cair no Chrome fixado só por causa do exe.
    if (lower(w.aumid).includes("_crx_")) return -1;
  }
  if (!w.exe) return -1;
  return pinned.findIndex((p) => p.exe && lower(p.exe) === lower(w.exe));
}

/** App instalado correspondente a uma janela: dá nome e ícone certos (ex.: PWAs que rodam no chrome.exe). */
function findInstalled(installed: AppEntry[], w: WindowInfo): AppEntry | undefined {
  if (w.aumid) {
    const byAumid = installed.find((a) => lower(a.aumid) === lower(w.aumid));
    if (byAumid || lower(w.aumid).includes("_crx_")) return byAumid;
  }
  return installed.find((a) => a.exe && lower(a.exe) === lower(w.exe));
}

/** O que abrir para um app aberto que não está na lista de instalados. */
function launchTargetForWindow(w: WindowInfo, isUwp: boolean): string {
  // No Linux o desktop id (`firefox.desktop`) é resolvido pelo backend.
  if (os === "linux" && w.aumid) return w.aumid;
  return w.aumid && isUwp ? `shell:AppsFolder\\${w.aumid}` : w.exe;
}

/** Junta apps fixados com janelas abertas: cada janela vai para o app fixado correspondente ou forma um grupo próprio. */
export function buildDockApps(
  pinned: AppEntry[],
  windows: WindowInfo[],
  showUnpinned: boolean,
  installed: AppEntry[] = [],
): DockApp[] {
  const pinnedApps: DockApp[] = pinned.map((p) => ({
    key: appKey(p.id),
    name: p.name,
    pinned: p,
    windows: [],
    iconKey: iconKeyForEntry(p),
    launchTarget: p.target,
    badge: 0,
  }));
  const running = new Map<string, DockApp>();

  for (const w of windows) {
    const idx = findPinned(pinned, w);
    if (idx >= 0) {
      pinnedApps[idx].windows.push(w);
      continue;
    }
    if (!showUnpinned || (!w.exe && !w.aumid)) continue;
    const groupKey = `run:${lower(w.aumid) || lower(w.exe)}`;
    let app = running.get(groupKey);
    if (!app) {
      const isUwp = lower(w.exe).endsWith("applicationframehost.exe");
      const match = findInstalled(installed, w);
      app = {
        key: groupKey,
        name: match?.name ?? (isUwp ? w.title : prettyExeName(w.exe)),
        windows: [],
        iconKey: match ? iconKeyForEntry(match) : iconKeyForWindow(w),
        launchTarget: match?.target ?? launchTargetForWindow(w, isUwp),
        installed: match,
        badge: 0,
      };
      running.set(groupKey, app);
    }
    app.windows.push(w);
  }
  const apps = [...pinnedApps, ...running.values()];
  for (const app of apps) {
    const fromTitle = app.windows.reduce((sum, w) => sum + parseTitleCount(w.title), 0);
    const fromSystem = Math.max(0, ...app.windows.map((w) => w.badge ?? 0));
    app.badge = Math.max(fromTitle, fromSystem);
  }
  return apps;
}

/** Converte um app aberto (não fixado) em um app fixado. */
export function toPinned(app: DockApp): AppEntry {
  if (app.installed) return app.installed;
  const w = app.windows[0];
  return {
    id: app.key,
    name: app.name,
    target: app.launchTarget,
    exe: w?.exe ?? null,
    aumid: w?.aumid ?? null,
  };
}
