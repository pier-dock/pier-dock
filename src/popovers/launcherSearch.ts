import type { AppEntry } from "../lib/ipc";

/** Compara sem depender de maiúsculas nem de acentos, como a busca das configurações. */
function normalize(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "");
}

/** O nome do app contém a busca? Busca vazia casa com todos. */
export function matchesQuery(app: AppEntry, query: string): boolean {
  const q = normalize(query);
  return !q || normalize(app.name).includes(q);
}

/**
 * Apps do lançador: ordenados por nome e filtrados pela busca. A lista vem do backend e pode
 * chegar em qualquer ordem, então a ordem alfabética é feita aqui.
 */
export function filterApps(apps: AppEntry[], query: string): AppEntry[] {
  const sorted = [...apps].sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  return sorted.filter((a) => matchesQuery(a, query));
}
