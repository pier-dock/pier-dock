import { openUrl } from "@tauri-apps/plugin-opener";
import { useAiUsage } from "../hooks/useAiUsage";
import { limitLevel, providerName, PROVIDER_USAGE_URL, windows, type LimitWindow } from "../lib/aiUsage";
import { formatPercent, formatResetIn, formatTokens, shortModel } from "../lib/format";
import { ActionFooter } from "../ui/ActionFooter";
import { dict, useT } from "../i18n";

/** Barra de uma janela de limite. Percentual desconhecido fica vazia, não em zero. */
function Limit({ limit }: { limit: LimitWindow }) {
  const level = limitLevel(limit.percent);
  const t = useT().aiUsage;
  return (
    <div className="usage-bar">
      <div className="usage-bar-head">
        <span>{limit.name}</span>
        <span className="usage-bar-value">
          {formatPercent(limit.percent)}
          {limit.resetsIn !== null && limit.resetsIn > 0 && <> · {t.resetsIn(formatResetIn(limit.resetsIn))}</>}
        </span>
      </div>
      <div
        className="usage-track"
        role="meter"
        aria-label={limit.name}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={limit.percent ?? 0}
      >
        <span className={`usage-fill ai-${level}`} style={{ width: `${limit.percent ?? 0}%` }} />
      </div>
    </div>
  );
}

/** "atualizado há 4 min"; com a IA fechada há mais de 15 min, "coletado há 45min". */
function freshness(ageMin: number, stale: boolean): string {
  const t = dict().aiUsage;
  if (stale) return t.collectedAgo(formatResetIn(ageMin));
  return ageMin < 1 ? t.updatedNow : t.updatedAgo(ageMin);
}

/**
 * Limites da conta e tokens da IA escolhida no widget. A IA que respondeu vem no cabeçalho: no
 * "automático" o widget pode ter caído em qualquer uma delas, e o número sozinho não diria qual.
 */
export function AiUsagePanel() {
  const usage = useAiUsage();
  const limits = windows(usage);
  const unknown = limits.every((w) => w.percent === null);
  const { aiUsage: t, common } = useT();
  const provider = usage ? providerName(usage.provider) : t.title;

  return (
    <>
      <div className="usage-header">
        <span className="eyebrow">{provider}</span>
        {usage && usage.ageMin !== null && (
          <span
            className={`popover-meta ${usage.stale ? "ai-stale" : ""}`}
            title={usage.stale ? t.staleHint(provider) : undefined}
          >
            {freshness(usage.ageMin, usage.stale)}
          </span>
        )}
      </div>
      <div className="ai-limits">
        {limits.map((limit, i) => (
          <Limit key={usage?.windows[i]?.id ?? i} limit={limit} />
        ))}
        {unknown && <span className="popover-meta">{t.openOnce(provider)}</span>}
      </div>
      <div className="usage-section">
        <span className="eyebrow">Tokens</span>
        <div className="usage-bar-head">
          <span>{common.today}</span>
          <span className="usage-bar-value">{usage ? `${formatTokens(usage.todayTokens)} · ${t.messages(usage.todayMessages)}` : "…"}</span>
        </div>
        <div className="usage-bar-head">
          <span>{t.block}</span>
          <span className="usage-bar-value">{usage ? formatTokens(usage.blockTokens) : "…"}</span>
        </div>
        {usage?.topModel && (
          <div className="usage-bar-head">
            <span>{t.model}</span>
            <span className="usage-bar-value">{shortModel(usage.topModel)}</span>
          </div>
        )}
      </div>
      <ActionFooter
        context={provider}
        actions={[
          {
            label: t.viewOnline,
            keys: "↵",
            onClick: () => openUrl(PROVIDER_USAGE_URL[usage?.provider ?? "claude"]),
          },
        ]}
      />
    </>
  );
}
