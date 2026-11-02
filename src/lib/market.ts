import { locale } from "../i18n/index.ts";

/** Ações e FIIs que a brapi responde sem chave; o mesmo conjunto está em `market.rs`. */
export const FREE_MARKET_TICKERS = ["PETR4", "VALE3", "MGLU3", "ITUB4"] as const;

export function normalizeSymbol(value: string): string {
  return value.trim().toUpperCase();
}

/** Separa por vírgula, espaço, ponto e vírgula ou quebra de linha; descarta repetidos e vazios. */
export function parseSymbols(value: string): string[] {
  const seen = new Set<string>();
  for (const part of value.split(/[\s,;]+/)) {
    const symbol = normalizeSymbol(part);
    if (symbol && !seen.has(symbol)) seen.add(symbol);
  }
  return [...seen];
}

export function isFreeTicker(symbol: string): boolean {
  return (FREE_MARKET_TICKERS as readonly string[]).includes(normalizeSymbol(symbol));
}

const priceFormats = new Map<string, Intl.NumberFormat>();

/**
 * "R$ 52,71" ou "R$ 0,000123": abaixo de um real a cotação precisa de mais casas, senão cripto
 * barata aparece sempre como zero.
 */
export function formatPrice(value: number, currency = "BRL"): string {
  const abs = Math.abs(value);
  const digits = abs > 0 && abs < 0.01 ? 6 : abs < 1 ? 4 : 2;
  return numberFormat(`price:${currency || "BRL"}:${digits}`, {
    style: "currency",
    currency: currency || "BRL",
    minimumFractionDigits: 2,
    maximumFractionDigits: digits,
  }).format(value);
}

/** "+0,98%" ou "-0,20%", com o sinal sempre visível. */
export function formatChange(percent: number): string {
  return `${numberFormat("change", { signDisplay: "always", minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(percent)}%`;
}

/** Preço sem o símbolo da moeda, para caber no tile: "52,71", "0,0001", "380 mil". */
export function formatQuote(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 100_000) return numberFormat("quote:compact", { notation: "compact", maximumFractionDigits: 1 }).format(value);
  const digits = abs > 0 && abs < 0.01 ? 4 : abs < 1 ? 3 : 2;
  return numberFormat(`quote:${digits}`, { minimumFractionDigits: 2, maximumFractionDigits: digits }).format(value);
}

/** "726,6 bi", "61,7 mi": volume e valor de mercado não cabem por extenso. */
export function formatCompact(value: number): string {
  return numberFormat("compact", { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

function numberFormat(key: string, options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const cacheKey = `${locale()}:${key}`;
  let format = priceFormats.get(cacheKey);
  if (!format) priceFormats.set(cacheKey, (format = new Intl.NumberFormat(locale(), options)));
  return format;
}
