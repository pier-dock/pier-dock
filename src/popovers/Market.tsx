import { LineChart, TrendingDown, TrendingUp } from "lucide-react";
import { ipc, type MarketAsset } from "../lib/ipc";
import { useMarket } from "../hooks/useMarket";
import { timeAgo } from "../lib/format";
import { formatChange, formatCompact, formatPrice, formatQuote } from "../lib/market";
import { ActionFooter } from "../ui/ActionFooter";
import { useT } from "../i18n";

function MarketRow({ asset }: { asset: MarketAsset }) {
  const t = useT().market;
  const day = asset.dayLow != null && asset.dayHigh != null ? t.dayRange(formatQuote(asset.dayLow), formatQuote(asset.dayHigh)) : null;
  const week = asset.weekLow != null && asset.weekHigh != null ? t.weekRange(formatQuote(asset.weekLow), formatQuote(asset.weekHigh)) : null;
  const volume = asset.volume != null ? t.volume(formatCompact(asset.volume)) : null;
  const cap = asset.marketCap != null ? t.marketCap(formatCompact(asset.marketCap)) : null;
  const details = [day, week, volume, cap].filter(Boolean).join(" · ");
  const tone = asset.changePercent > 0 ? "up" : asset.changePercent < 0 ? "down" : "flat";

  return (
    <li className="market-row">
      <span className="market-logo" aria-hidden>
        {asset.logoUrl ? <img src={asset.logoUrl} alt="" loading="lazy" /> : asset.symbol.slice(0, 1)}
      </span>
      <span className="market-row-main">
        <span className="market-row-head">
          <span className="market-row-symbol">{asset.symbol}</span>
          <span className="market-row-name">{asset.name}</span>
        </span>
        {details && <span className="market-row-meta">{details}</span>}
      </span>
      <span className="market-row-values">
        <span className="market-row-price">{formatPrice(asset.price, asset.currency)}</span>
        <span className={`market-row-change ${tone}`}>
          {tone === "up" && <TrendingUp size={12} aria-hidden />}
          {tone === "down" && <TrendingDown size={12} aria-hidden />}
          {formatChange(asset.changePercent)}
        </span>
      </span>
    </li>
  );
}

export function Market() {
  const data = useMarket();
  const { market: t } = useT();
  const assets = data?.assets ?? [];
  const skipped = [...(data?.unavailable ?? []), ...(data?.missing ?? [])];

  if (data && !assets.length) {
    return (
      <div className="popover-empty">
        <span className="empty-icon">
          <LineChart size={22} strokeWidth={1.8} />
        </span>
        <span className="empty-title">{skipped.length ? t.unavailable(skipped.join(", ")) : t.empty}</span>
        <span className="popover-meta">{skipped.length ? t.unavailableHint : t.emptyHint}</span>
        <button className="action primary" onClick={() => ipc.openSettings("widgets")}>
          {t.settings}
        </button>
      </div>
    );
  }

  return (
    <>
      <div className="market-header">
        <span className="eyebrow">{t.title}</span>
        {data && <span className="popover-meta">{t.updated(timeAgo(data.fetchedAt))}</span>}
      </div>
      {data?.error && <div className="error-box">{data.error}</div>}
      {data && assets.length > 0 && (
        <ul className="market-list" aria-label={t.title}>
          {assets.map((asset) => (
            <MarketRow key={asset.symbol} asset={asset} />
          ))}
        </ul>
      )}
      {!!data?.unavailable.length && assets.length > 0 && (
        <div className="market-note">
          <span>{t.unavailable(data.unavailable.join(", "))}</span>
          <span>{t.unavailableHint}</span>
        </div>
      )}
      {!!data?.missing.length && assets.length > 0 && <div className="market-note">{t.missing(data.missing.join(", "))}</div>}
      <p className="market-source">{t.source}</p>
      <ActionFooter
        context={t.title}
        actions={[
          { label: t.refresh, keys: "↵", onClick: () => ipc.marketFetch() },
          { label: t.settings, onClick: () => ipc.openSettings("widgets") },
        ]}
      />
    </>
  );
}
