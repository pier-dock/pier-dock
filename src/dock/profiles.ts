import type { Config, DockProfile } from "../lib/ipc";
import { newGroupId } from "./groups.ts";
import { groupKey, groupIdOf, isGroupKey } from "./order.ts";

export const DEFAULT_PROFILE_ICON = "layers";

const CONTENT_KEYS = ["accent", "pinnedApps", "widgets", "dockOrder", "widgetGroups", "showStartButton", "showLauncher"] as const;
type ContentKey = (typeof CONTENT_KEYS)[number];
type ProfileContent = Pick<Config, ContentKey>;
type Patch = Partial<Config> | ((c: Config) => Partial<Config>);

const isContentKey = (key: string): key is ContentKey => (CONTENT_KEYS as readonly string[]).includes(key);

function contentOf(source: ProfileContent): ProfileContent {
  const { accent, pinnedApps, widgets, dockOrder, widgetGroups, showStartButton, showLauncher } = source;
  return { accent, pinnedApps, widgets, dockOrder, widgetGroups, showStartButton, showLauncher };
}

const newProfileId = () => `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

/** A lista de perfis com o ativo igual ao topo da config, que é onde ele é editado. */
function syncedProfiles(config: Config): DockProfile[] {
  return config.dockProfiles.map((p) => (p.id === config.activeProfile ? { ...p, ...contentOf(config) } : p));
}

/** A config como `id` a enxerga: o conteúdo dele no lugar do conteúdo do topo. */
export function profileView(config: Config, id: string): Config {
  if (id === config.activeProfile) return config;
  const profile = config.dockProfiles.find((p) => p.id === id);
  return profile ? { ...config, ...contentOf(profile) } : config;
}

/** Leva para `id` o que é conteúdo de perfil na mudança; o resto da config continua global. */
export function patchForProfile(config: Config, id: string, patch: Partial<Config>): Partial<Config> {
  if (id === config.activeProfile || !config.dockProfiles.some((p) => p.id === id)) return patch;
  const shared: Partial<Config> = {};
  const content: Partial<ProfileContent> = {};
  for (const [key, value] of Object.entries(patch)) {
    Object.assign(isContentKey(key) ? content : shared, { [key]: value });
  }
  if (!Object.keys(content).length) return shared;
  return { ...shared, dockProfiles: config.dockProfiles.map((p) => (p.id === id ? { ...p, ...content } : p)) };
}

/** Um `update` da config que escreve em `id`, seja ele o perfil ativo ou não. */
export function scopedUpdate<R>(update: (patch: (c: Config) => Partial<Config>) => R, id: string) {
  return (patch: Patch) =>
    update((c) => patchForProfile(c, id, typeof patch === "function" ? patch(profileView(c, id)) : patch));
}

export function switchProfile(config: Config, id: string): Partial<Config> {
  const target = config.dockProfiles.find((p) => p.id === id);
  if (!target || id === config.activeProfile) return {};
  return { activeProfile: id, dockProfiles: syncedProfiles(config), ...contentOf(target) };
}

/**
 * O perfil de uma janela da dock. A principal (`monitor` nulo) usa o ativo; uma extra só tem perfil
 * próprio com o modo por monitor ligado e a dock em todos os monitores.
 */
export function windowProfileId(config: Config, monitor: string | null): string {
  if (!monitor || !config.profilePerMonitor || config.dockMonitor !== "all") return config.activeProfile;
  const linked = config.monitorProfiles[monitor];
  return linked && config.dockProfiles.some((p) => p.id === linked) ? linked : config.activeProfile;
}

export function setWindowProfile(config: Config, monitor: string | null, id: string): Partial<Config> {
  if (monitor && config.profilePerMonitor && config.dockMonitor === "all") {
    return { monitorProfiles: { ...config.monitorProfiles, [monitor]: id } };
  }
  return switchProfile(config, id);
}

/** O perfil seguinte (+1) ou anterior (-1), em círculo: depois do último vem o primeiro. */
export function neighborProfile(profiles: readonly DockProfile[], id: string, direction: 1 | -1): DockProfile | null {
  if (profiles.length < 2) return null;
  const index = profiles.findIndex((p) => p.id === id);
  if (index < 0) return null;
  return profiles[(index + direction + profiles.length) % profiles.length];
}

/** Grupos com id novo, para o painel de um grupo saber a qual perfil ele pertence. */
function withFreshGroups(profile: DockProfile): DockProfile {
  const ids = new Map(profile.widgetGroups.map((group) => [group.id, newGroupId()]));
  return {
    ...profile,
    widgetGroups: profile.widgetGroups.map((group) => ({ ...group, id: ids.get(group.id)! })),
    dockOrder: profile.dockOrder.map((key) => (isGroupKey(key) && ids.has(groupIdOf(key)) ? groupKey(ids.get(groupIdOf(key))!) : key)),
  };
}

/** Acrescenta um perfil vazio (só o botão de apps, para a barra ter onde receber o scroll). */
export function addEmptyProfile(config: Config, name: string): { id: string; patch: Partial<Config> } {
  const profile: DockProfile = {
    id: newProfileId(),
    name,
    icon: DEFAULT_PROFILE_ICON,
    accent: config.accent,
    pinnedApps: [],
    widgets: config.widgets.map((w) => ({ ...w, enabled: false })),
    dockOrder: [],
    widgetGroups: [],
    showStartButton: true,
    showLauncher: false,
  };
  return { id: profile.id, patch: { dockProfiles: [...syncedProfiles(config), profile] } };
}

/** Copia `sourceId` logo depois dele. */
export function duplicateProfile(config: Config, sourceId: string, name: string): { id: string; patch: Partial<Config> } | null {
  const profiles = syncedProfiles(config);
  const index = profiles.findIndex((p) => p.id === sourceId);
  if (index < 0) return null;
  const copy = { ...withFreshGroups(profiles[index]), id: newProfileId(), name };
  return { id: copy.id, patch: { dockProfiles: [...profiles.slice(0, index + 1), copy, ...profiles.slice(index + 1)] } };
}

/** Exclui um perfil; o último não pode sair. Se ele era o ativo, o vizinho assume. */
export function removeProfile(config: Config, id: string): Partial<Config> | null {
  const profiles = syncedProfiles(config);
  const index = profiles.findIndex((p) => p.id === id);
  if (index < 0 || profiles.length < 2) return null;
  const rest = profiles.filter((p) => p.id !== id);
  const monitorProfiles = Object.fromEntries(Object.entries(config.monitorProfiles).filter(([, profile]) => profile !== id));
  if (id !== config.activeProfile) return { dockProfiles: rest, monitorProfiles };
  const next = rest[Math.min(index, rest.length - 1)];
  return { dockProfiles: rest, monitorProfiles, activeProfile: next.id, ...contentOf(next) };
}

export function moveProfile(config: Config, id: string, delta: -1 | 1): Partial<Config> | null {
  const profiles = syncedProfiles(config);
  const from = profiles.findIndex((p) => p.id === id);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= profiles.length) return null;
  [profiles[from], profiles[to]] = [profiles[to], profiles[from]];
  return { dockProfiles: profiles };
}

/** Nome e ícone não são conteúdo da dock e nunca passam pelo topo da config. */
export function renameProfile(config: Config, id: string, patch: Partial<Pick<DockProfile, "name" | "icon">>): Partial<Config> {
  return { dockProfiles: config.dockProfiles.map((p) => (p.id === id ? { ...p, ...patch } : p)) };
}

/** Algum perfil usa o widget? O polling cobre todos, para os dados não piscarem na troca. */
export function anyProfileWidget(config: Config, matches: (kind: string) => boolean): boolean {
  const on = (widgets: Config["widgets"]) => widgets.some((w) => w.enabled && matches(w.kind));
  return on(config.widgets) || config.dockProfiles.some((p) => on(p.widgets));
}

/** O perfil dono de um grupo. Um painel de grupo não sabe de qual dock ele foi aberto. */
export function profileOfGroup(config: Config, groupId: string): string {
  if (config.widgetGroups.some((g) => g.id === groupId)) return config.activeProfile;
  return config.dockProfiles.find((p) => p.widgetGroups.some((g) => g.id === groupId))?.id ?? config.activeProfile;
}
