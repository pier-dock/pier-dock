import type { Config } from "../lib/ipc";

/** Ordem livre dos itens da dock. `pinnedApps` e `widgets` guardam os dados; `dockOrder`, só a posição. */
type OrderConfig = Pick<Config, "dockOrder" | "pinnedApps" | "widgets" | "showStartButton" | "showLauncher">;

export const START_KEY = "start";
/** Lançador de apps do Pier (o `start` abre o menu do sistema). */
export const LAUNCHER_KEY = "launcher";
export const appKey = (id: string) => `app:${id}`;
export const widgetKey = (kind: string) => `widget:${kind}`;
export const isDivider = (key: string) => key.startsWith("divider:");

/** Itens que devem estar na dock, sem separadores. */
function itemKeys(config: OrderConfig): string[] {
  return [
    ...(config.showStartButton ? [START_KEY] : []),
    ...(config.showLauncher ? [LAUNCHER_KEY] : []),
    ...config.pinnedApps.map((p) => appKey(p.id)),
    ...config.widgets.filter((w) => w.enabled).map((w) => widgetKey(w.kind)),
  ];
}

/** Grupo de um widget na ordem padrão: mídia, GitHub, IA ou sistema. */
function widgetGroup(kind: string): string {
  if (kind === "media") return "media";
  if (kind === "ai-usage") return "ai";
  return kind.startsWith("github-") ? "github" : "system";
}

/**
 * Ordem de quem ainda não arrumou a dock: botão de apps, apps, mídia, GitHub e sistema, com um
 * separador entre cada grupo.
 */
function legacyOrder(config: OrderConfig): string[] {
  const groups = new Map<string, string[]>([
    ["start", [...(config.showStartButton ? [START_KEY] : []), ...(config.showLauncher ? [LAUNCHER_KEY] : [])]],
    ["apps", config.pinnedApps.map((p) => appKey(p.id))],
  ]);
  for (const w of config.widgets.filter((w) => w.enabled)) {
    const group = widgetGroup(w.kind);
    groups.set(group, [...(groups.get(group) ?? []), widgetKey(w.kind)]);
  }
  return [...groups]
    .filter(([, keys]) => keys.length)
    .flatMap(([group, keys], i) => (i === 0 ? keys : [`divider:${group}`, ...keys]));
}

/**
 * Ordem a renderizar: a salva, sem itens que não existem mais (app desafixado, widget desligado) e
 * com os que ainda não têm lugar (app recém-fixado, widget recém-ligado) no fim.
 */
export function resolveOrder(config: OrderConfig): string[] {
  if (!config.dockOrder.length) return legacyOrder(config);
  const valid = new Set(itemKeys(config));
  const seen = new Set<string>();
  const kept = config.dockOrder.filter((key) => {
    if (seen.has(key) || (!isDivider(key) && !valid.has(key))) return false;
    seen.add(key);
    return true;
  });
  return [...kept, ...[...valid].filter((key) => !seen.has(key))];
}

/**
 * Salva uma nova ordem e reordena `pinnedApps` e `widgets` para acompanhá-la — assim o Ctrl+Alt+N
 * e as listas das configurações seguem a ordem da dock. Widgets desligados vão para o fim.
 */
export function applyOrder(config: OrderConfig, keys: string[]): Pick<Config, "dockOrder" | "pinnedApps" | "widgets"> {
  const rank = new Map(keys.map((key, i) => [key, i]));
  const pos = (key: string) => rank.get(key) ?? Number.MAX_SAFE_INTEGER;
  return {
    dockOrder: keys,
    pinnedApps: [...config.pinnedApps].sort((a, b) => pos(appKey(a.id)) - pos(appKey(b.id))),
    widgets: [...config.widgets].sort((a, b) => pos(widgetKey(a.kind)) - pos(widgetKey(b.kind))),
  };
}

/** Tira um item da dock: desafixa o app, desliga o widget, esconde o botão de apps ou apaga o separador. */
export function removeItem(config: OrderConfig, key: string): Partial<Config> {
  const dockOrder = resolveOrder(config).filter((k) => k !== key);
  if (key === START_KEY) return { dockOrder, showStartButton: false };
  if (key === LAUNCHER_KEY) return { dockOrder, showLauncher: false };
  if (key.startsWith("app:")) {
    const id = key.slice("app:".length);
    return { dockOrder, pinnedApps: config.pinnedApps.filter((p) => p.id !== id) };
  }
  if (key.startsWith("widget:")) {
    const kind = key.slice("widget:".length);
    return { dockOrder, widgets: config.widgets.map((w) => (w.kind === kind ? { ...w, enabled: false } : w)) };
  }
  return { dockOrder };
}

export function addDivider(config: OrderConfig): Partial<Config> {
  return { dockOrder: [...resolveOrder(config), `divider:${Date.now().toString(36)}`] };
}

/**
 * Move `key` uma posição para cima (-1) ou para baixo (+1) entre os itens que passam em `among`
 * (só os apps, só os widgets…); os outros itens ficam onde estão. `null` se não dá para mover.
 */
export function moveAmong(
  config: OrderConfig,
  key: string,
  delta: -1 | 1,
  among: (key: string) => boolean,
): Pick<Config, "dockOrder" | "pinnedApps" | "widgets"> | null {
  const keys = resolveOrder(config);
  const slots = keys.flatMap((k, i) => (among(k) ? [i] : []));
  const from = slots.findIndex((i) => keys[i] === key);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= slots.length) return null;
  const next = [...keys];
  [next[slots[from]], next[slots[to]]] = [next[slots[to]], next[slots[from]]];
  return applyOrder(config, next);
}
