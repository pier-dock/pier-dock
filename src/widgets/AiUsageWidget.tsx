import { Sparkles } from "lucide-react";
import type { AiUsage } from "../lib/ipc";
import { limitLevel, providerName, tightest } from "../lib/aiUsage";
import { formatPercent, formatResetIn, formatTokens } from "../lib/format";
import { Ring } from "../ui/Ring";
import { Tile } from "./Tile";
import { useT } from "../i18n";

/**
 * Anel do limite mais apertado da IA escolhida — a sessão de 5h ou a semana, no Claude Code; a
 * janela do plano, no Codex. Sem dado o anel fica vazio e o número vira "--": o cache de limites só
 * existe depois que a ferramenta roda uma vez. Qual IA respondeu aparece no rótulo (o tooltip), no
 * cabeçalho do painel e na escolha das configurações.
 */
export function AiUsageWidget({ data, wide }: { data: AiUsage | null; wide: boolean }) {
  const limit = tightest(data);
  const level = limitLevel(limit.percent);
  const progress = (limit.percent ?? 0) / 100;
  const t = useT().aiUsage;
  const provider = data ? providerName(data.provider) : t.title;
  const parts = [
    limit.percent === null
      ? t.tileNoData(provider)
      : t.tileLimit(provider, limit.name, formatPercent(limit.percent), formatResetIn(limit.resetsIn)),
  ];
  if (data?.todayTokens) parts.push(t.tileTokensToday(formatTokens(data.todayTokens)));
  // A leitura congela com a ferramenta fechada; avisar é melhor do que passar um número velho por atual.
  if (data?.stale && data.ageMin) parts.push(t.collectedAgo(formatResetIn(data.ageMin)));
  const className = `ai-tile ${wide ? "wide" : "small"} ai-${level} ${data?.stale ? "stale" : ""}`;

  if (!wide) {
    return (
      <Tile kind="ai-usage" label={parts.join(" · ")} className={className}>
        <Ring size={40} radius={16.5} stroke={2.5} progress={progress} />
        <span className="ai-mark">
          <Sparkles size={9} strokeWidth={2.6} aria-hidden />
          <span className="ai-percent">
            {limit.percent ?? "--"}
            {limit.percent !== null && <small>%</small>}
          </span>
        </span>
      </Tile>
    );
  }

  return (
    <Tile kind="ai-usage" label={parts.join(" · ")} className={className}>
      <span className="ai-ring">
        <Ring size={34} radius={14} stroke={2.5} progress={progress} />
        <Sparkles size={12} strokeWidth={2.4} aria-hidden />
      </span>
      <span className="ai-text">
        <span className="ai-value">{formatPercent(limit.percent)}</span>
        <span className="ai-caption">{limit.percent === null ? t.title : limit.name}</span>
      </span>
    </Tile>
  );
}
