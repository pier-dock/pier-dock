import type { AppEntry } from "../lib/ipc";

/** Compara sem depender de maiúsculas nem de acentos, como a busca das configurações. */
function normalize(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
}

/**
 * Apps do lançador: ordenados por nome e filtrados pela busca. A lista vem do backend e pode
 * chegar em qualquer ordem, então a ordem alfabética é feita aqui.
 */
export function filterApps(apps: AppEntry[], query: string): AppEntry[] {
  const sorted = [...apps].sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  const q = normalize(query);
  return q ? sorted.filter((a) => normalize(a.name).includes(q)) : sorted;
}
