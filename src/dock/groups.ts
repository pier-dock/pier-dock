import type { Config, WidgetGroup, WidgetKind } from "../lib/ipc";
import { groupIdOf, groupKey, isGroupKey, resolveOrder, widgetKey, type OrderConfig } from "./order.ts";

/** Identificador curto do grupo; não usa `crypto.randomUUID` para funcionar em qualquer WebView. */
export const newGroupId = () => `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

const kindOf = (key: string) => (key.startsWith("widget:") ? (key.slice("widget:".length) as WidgetKind) : null);

const enabledKinds = (config: OrderConfig) => new Set(config.widgets.filter((w) => w.enabled).map((w) => w.kind));

const groupedBy = (config: OrderConfig, kind: WidgetKind) => (config.widgetGroups ?? []).find((group) => group.members.includes(kind));

/**
 * Tira `kind` do grupo em que ele está, devolvendo a ordem já sem a chave do grupo quando ele
 * fica com menos de dois membros ligados. O chamador decide onde o widget entra depois.
 */
function detachMember(config: OrderConfig, kind: WidgetKind): { groups: WidgetGroup[]; order: string[] } {
  const group = groupedBy(config, kind);
  const current = resolveOrder(config);
  if (!group) return { groups: config.widgetGroups ?? [], order: current };

  const members = group.members.filter((member) => member !== kind);
  const enabled = enabledKinds(config);
  const stillActive = members.filter((member) => enabled.has(member)).length >= 2;
  let order = current;
  if (!stillActive) {
    const key = groupKey(group.id);
    const index = current.indexOf(key);
    const rest = current.filter((item) => item !== key);
    const replacement = members.filter((member) => enabled.has(member)).map(widgetKey);
    const at = index < 0 ? rest.length : Math.min(index, rest.length);
    order = [...rest.slice(0, at), ...replacement, ...rest.slice(at)];
  }
  const groups = members.length
    ? (config.widgetGroups ?? []).map((g) => (g.id === group.id ? { ...g, members } : g))
    : (config.widgetGroups ?? []).filter((g) => g.id !== group.id);
  return { groups, order };
}

/** Cria um grupo novo com o widget arrastado entrando depois do alvo. */
export function createGroup(config: OrderConfig, draggedKey: string, targetKey: string): Partial<Config> | null {
  const dragged = kindOf(draggedKey);
  if (!dragged) return null;
  if (isGroupKey(targetKey)) return addToGroup(config, groupIdOf(targetKey), dragged);

  const target = kindOf(targetKey);
  if (!target || target === dragged) return null;
  const targetGroup = groupedBy(config, target);
  if (targetGroup) return addToGroup(config, targetGroup.id, dragged);

  const detached = groupedBy(config, dragged) ? detachMember(config, dragged) : { groups: config.widgetGroups ?? [], order: resolveOrder(config) };
  const order = detached.order.filter((key) => key !== draggedKey);
  const index = order.indexOf(targetKey);
  if (index < 0) return null;

  const group: WidgetGroup = { id: newGroupId(), members: [target, dragged] };
  order[index] = groupKey(group.id);
  return { dockOrder: order, widgetGroups: [...detached.groups, group] };
}

/** Acrescenta um widget a um grupo existente, tirando-o do grupo anterior (se houver). */
export function addToGroup(config: OrderConfig, groupId: string, kind: WidgetKind): Partial<Config> | null {
  const target = (config.widgetGroups ?? []).find((group) => group.id === groupId);
  if (!target) return null;
  if (target.members.includes(kind)) return {};

  if (groupedBy(config, kind)) {
    const detached = detachMember(config, kind);
    const current = detached.groups.find((group) => group.id === groupId);
    if (!current) return null;
    let order = detached.order;
    const key = groupKey(groupId);
    if (!order.includes(key)) {
      const firstIndex = order.findIndex((item) => current.members.some((member) => widgetKey(member) === item));
      const rest = order.filter((item) => !current.members.some((member) => widgetKey(member) === item));
      const at = firstIndex < 0 ? rest.length : Math.min(firstIndex, rest.length);
      order = [...rest.slice(0, at), key, ...rest.slice(at)];
    }
    order = order.filter((item) => item !== widgetKey(kind));
    return {
      dockOrder: order,
      widgetGroups: detached.groups.map((group) => (group.id === groupId ? { ...group, members: [...group.members, kind] } : group)),
    };
  }

  let order = resolveOrder(config).filter((item) => item !== widgetKey(kind));
  const key = groupKey(groupId);
  if (!order.includes(key)) {
    const firstIndex = order.findIndex((item) => target.members.some((member) => widgetKey(member) === item));
    const rest = order.filter((item) => !target.members.some((member) => widgetKey(member) === item));
    const at = firstIndex < 0 ? rest.length : Math.min(firstIndex, rest.length);
    order = [...rest.slice(0, at), key, ...rest.slice(at)];
  }
  return {
    dockOrder: order,
    widgetGroups: (config.widgetGroups ?? []).map((group) => (group.id === groupId ? { ...group, members: [...group.members, kind] } : group)),
  };
}

/** Tira um widget do grupo; com menos de dois membros ligados o grupo se desfaz e os membros voltam. */
export function removeFromGroup(config: OrderConfig, groupId: string, kind: WidgetKind): Partial<Config> | null {
  const group = (config.widgetGroups ?? []).find((g) => g.id === groupId);
  if (!group || !group.members.includes(kind)) return null;

  const members = group.members.filter((member) => member !== kind);
  const enabled = enabledKinds(config);
  const stillActive = members.filter((member) => enabled.has(member)).length >= 2;
  const key = groupKey(groupId);
  const order = resolveOrder(config);
  const index = order.indexOf(key);
  const rest = order.filter(
    (item) => item !== key && item !== widgetKey(kind) && !members.some((member) => widgetKey(member) === item),
  );
  const replacement = stillActive ? [key] : members.filter((member) => enabled.has(member)).map(widgetKey);
  const at = index < 0 ? rest.length : Math.min(index, rest.length);
  const groups = members.length
    ? (config.widgetGroups ?? []).map((g) => (g.id === groupId ? { ...g, members } : g))
    : (config.widgetGroups ?? []).filter((g) => g.id !== groupId);
  return {
    dockOrder: [...rest.slice(0, at), ...replacement, widgetKey(kind), ...rest.slice(at)],
    widgetGroups: groups,
  };
}

/** Desfaz o grupo, devolvendo todos os membros à dock na posição em que o cartão estava. */
export function dissolveGroup(config: OrderConfig, groupId: string): Partial<Config> {
  const group = (config.widgetGroups ?? []).find((g) => g.id === groupId);
  if (!group) return {};
  const key = groupKey(groupId);
  const order = resolveOrder(config);
  const index = order.indexOf(key);
  const rest = order.filter((item) => item !== key && !group.members.some((member) => widgetKey(member) === item));
  const at = index < 0 ? rest.length : Math.min(index, rest.length);
  return {
    dockOrder: [...rest.slice(0, at), ...group.members.map(widgetKey), ...rest.slice(at)],
    widgetGroups: (config.widgetGroups ?? []).filter((g) => g.id !== groupId),
  };
}

/** Cria um grupo a partir de uma escolha nas configurações, no lugar do primeiro widget escolhido. */
export function createGroupFromKinds(config: OrderConfig, kinds: WidgetKind[]): Partial<Config> | null {
  const enabled = enabledKinds(config);
  const grouped = new Set<string>((config.widgetGroups ?? []).flatMap((group) => group.members));
  const members = [...new Set(kinds)].filter((kind) => enabled.has(kind) && !grouped.has(kind));
  if (members.length < 2) return null;

  const order = resolveOrder(config);
  const index = order.indexOf(widgetKey(members[0]));
  if (index < 0) return null;
  const rest = order.filter((item) => !members.some((member) => widgetKey(member) === item));
  const group: WidgetGroup = { id: newGroupId(), members };
  const at = Math.min(index, rest.length);
  return {
    dockOrder: [...rest.slice(0, at), groupKey(group.id), ...rest.slice(at)],
    widgetGroups: [...(config.widgetGroups ?? []), group],
  };
}
