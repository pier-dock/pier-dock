import { settledSizes } from "./magnifyGeometry.ts";

export interface FitItem {
  key: string;
  /** Comprimento natural no eixo da barra (px). */
  length: number;
  /** Comprimento no formato compacto, para os itens que têm um (os widgets largos). */
  compact?: number;
  /** Itens fixos nunca vão para o menu de excedentes. */
  fixed?: boolean;
}

export interface Fit {
  /** Itens que passam a ser desenhados no formato compacto. */
  compact: Set<string>;
  /** Itens que não couberam e vão para o menu, na ordem da barra. */
  overflow: string[];
}

/**
 * Decide o que fazer para os itens caberem em `available` px. Parte sempre dos comprimentos
 * naturais, nunca dos atuais: usar o que já está compactado faria a barra alternar entre os
 * estados a cada medição. Primeiro compacta os widgets largos, do fim para o começo; se ainda
 * não couber, move os últimos itens não fixos para o menu, reservando `chip` px para o botão dele.
 */
export function fitItems(items: FitItem[], available: number, gap: number, chip: number): Fit {
  const compact = new Set<string>();
  const lengths = items.map((i) => i.length);
  const total = (ls: number[]) => ls.reduce((sum, l) => sum + l, 0) + gap * Math.max(0, ls.length - 1);

  if (total(lengths) <= available) return { compact, overflow: [] };

  for (let i = items.length - 1; i >= 0 && total(lengths) > available; i--) {
    const c = items[i].compact;
    if (c == null || c >= lengths[i]) continue;
    lengths[i] = c;
    compact.add(items[i].key);
  }
  if (total(lengths) <= available) return { compact, overflow: [] };

  const kept = items.map((_, i) => i);
  const overflow: string[] = [];
  while (kept.length && total([...kept.map((i) => lengths[i]), chip]) > available) {
    let at = kept.length - 1;
    while (at >= 0 && items[kept[at]].fixed) at--;
    if (at < 0) break;
    const [i] = kept.splice(at, 1);
    overflow.unshift(items[i].key);
    compact.delete(items[i].key);
  }
  return { compact, overflow };
}

/** Menor ícone que a dock usa para caber; abaixo disso os apps deixam de ser reconhecíveis. */
export const MIN_FIT_ICON = 24;
/**
 * Folga exigida para o ícone voltar a crescer. O relógio lateral muda de corpo com o ícone, então
 * o resto da barra varia junto: sem essa folga o tamanho ficaria alternando entre dois valores.
 */
const GROW_SLACK = 4;

export interface DockFitInput {
  iconSize: number;
  magnification: number;
  /** Comprimento disponível para a barra (px). */
  available: number;
  /** Comprimento da barra sem as áreas dos ícones: widgets, vãos, moldura (px). */
  rest: number;
  /** Quantos ícones de app a barra tem. */
  count: number;
  /** Tamanho do ícone na medição anterior. */
  previous?: number;
}

export interface DockFit {
  icon: number;
  scale: number;
}

/** Quanto a fileira cresce com o cursor no meio dela, onde a magnificação alcança mais ícones. */
function peakGrowth(count: number, icon: number, scale: number): number {
  if (count === 0 || scale <= 1) return 0;
  // Ícones colados: nunca menos crescimento que o real, com qualquer vão entre eles.
  const row = Array.from({ length: count }, (_, i) => ({ center: i * icon, initial: icon }));
  const middle = Math.floor((count - 1) / 2) * icon;
  const growth = (cursor: number) => settledSizes(cursor, row, icon, scale).reduce((sum, size) => sum + size - icon, 0);
  return Math.max(growth(middle), growth(middle + icon / 2));
}

/**
 * Tamanho do ícone e magnificação para a barra caber em `available` px. Encolhe o ícone só o
 * necessário para a barra caber em repouso; a magnificação fica limitada ao que sobra.
 */
export function fitDock({ iconSize, magnification, available, rest, count, previous }: DockFitInput): DockFit {
  const largest = (room: number) => {
    if (count === 0) return iconSize;
    return Math.min(iconSize, Math.max(Math.min(iconSize, MIN_FIT_ICON), Math.floor((room - rest) / count)));
  };
  const fits = largest(available);
  const icon = previous === undefined ? fits : Math.max(largest(available - GROW_SLACK), Math.min(previous, fits));

  const slack = available - rest - count * icon;
  if (peakGrowth(count, icon, magnification) <= slack) return { icon, scale: magnification };
  let [low, high] = [1, magnification];
  for (let pass = 0; pass < 12; pass++) {
    const middle = (low + high) / 2;
    if (peakGrowth(count, icon, middle) <= slack) low = middle;
    else high = middle;
  }
  return { icon, scale: Math.floor(low * 100) / 100 };
}
