import { dict } from "../i18n/index.ts";
import type { AiProviderId, AiUsage } from "./ipc";

/** A partir destes percentuais o anel troca de cor. */
export const WARN = 70;
export const CRITICAL = 90;

/** As IAs conhecidas, na ordem em que a escolha das configurações as oferece. */
export const AI_PROVIDERS: AiProviderId[] = ["claude", "codex"];

export type Level = "unknown" | "normal" | "warn" | "critical";

export function limitLevel(percent: number | null): Level {
  if (percent === null) return "unknown";
  if (percent >= CRITICAL) return "critical";
  if (percent >= WARN) return "warn";
  return "normal";
}

export interface LimitWindow {
  name: string;
  percent: number | null;
  resetsIn: number | null;
}

/**
 * Rótulo da janela. As duas do Claude Code têm nome consagrado (a sessão de 5h e a semana); nas
 * outras ferramentas só a duração é conhecida, e um `window_minutes` de 8h vira "janela de 8h".
 */
function windowName(minutes: number | null): string {
  const t = dict().aiUsage;
  if (minutes === null) return t.otherWindow;
  if (minutes === 300) return t.session;
  if (minutes === 10080) return t.week;
  const hours = minutes / 60;
  return t.window(hours < 24 ? `${Math.round(hours)}h` : `${Math.round(minutes / 1440)}d`);
}

export function windows(usage: AiUsage | null): LimitWindow[] {
  return (usage?.windows ?? []).map((w) => ({ name: windowName(w.minutes), percent: w.percent, resetsIn: w.resetsIn }));
}

/**
 * A janela mais perto do limite, que é a que vai bloquear primeiro — e por isso a que o tile
 * mostra. Janela com percentual desconhecido não concorre; se nenhuma for conhecida, devolve a
 * primeira (a sessão de 5h, no Claude Code), para o tile renderizar o estado "sem dado".
 */
export function tightest(usage: AiUsage | null): LimitWindow {
  const list = windows(usage);
  const known = list.filter((w) => w.percent !== null);
  if (!known.length) return list[0] ?? { name: dict().aiUsage.session, percent: null, resetsIn: null };
  return known.reduce((a, b) => ((b.percent ?? 0) > (a.percent ?? 0) ? b : a));
}

/** Nome da IA no idioma atual, para o rótulo do tile e o cabeçalho do painel. */
export function providerName(id: string): string {
  const t = dict().aiUsage.providers as Record<string, string>;
  return t[id] ?? id;
}

/** Onde ver o uso na web, por IA. */
export const PROVIDER_USAGE_URL: Record<AiProviderId, string> = {
  claude: "https://claude.ai/settings/usage",
  codex: "https://chatgpt.com/codex/settings/usage",
};
