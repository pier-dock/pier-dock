import { useEffect, useState } from "react";
import { LineChart } from "lucide-react";
import type { MarketAsset, MarketData } from "../lib/ipc";
import { formatChange, formatQuote } from "../lib/market";
import { useT } from "../i18n";
import { Tile } from "./Tile";
import { setActiveInterval } from "../lib/dockActivity";

/** Tempo de cada página de cotações, como o painel de uma bolsa. */
const ROTATE_MS = 4000;

const tone = (value: number) => (value > 0 ? "up" : value < 0 ? "down" : "flat");

function AssetCell({ asset }: { asset: MarketAsset }) {
  return (
    <span className="market-cell">
      <span className="market-symbol">{asset.symbol}</span>
      <span className="market-quote">{formatQuote(asset.price)}</span>
      <span className={`market-change ${tone(asset.changePercent)}`}>{formatChange(asset.changePercent)}</span>
    </span>
  );
}

export function MarketWidget({ data, wide }: { data: MarketData | null; wide: boolean }) {
  const t = useT().market;
  const assets = data?.assets ?? [];
  const perPage = wide ? 3 : 1;
  const pages = Math.ceil(assets.length / perPage);
  const [page, setPage] = useState(0);
  useEffect(() => {
    if (pages < 2) return;
    return setActiveInterval(() => setPage((current) => (current + 1) % pages), ROTATE_MS);
  }, [pages]);
  if (!assets.length) {
    const label = data?.error ?? (data ? t.tileChoose : t.tileLoading);
    return (
      <Tile kind="market" label={label} className="market-tile idle">
        <LineChart size={wide ? 22 : 18} strokeWidth={1.8} aria-hidden />
        {wide && <span className="market-idle-text">{data ? t.title : t.tileLoading}</span>}
      </Tile>
    );
  }

  const current = page % pages;
  const shown = assets.slice(current * perPage, (current + 1) * perPage);
  const label = shown
    .map((asset) => t.tileLabel(asset.symbol, asset.name, formatQuote(asset.price), formatChange(asset.changePercent)))
    .join(" · ");
  return (
    <Tile kind="market" label={label} className={`market-tile ${wide ? "wide" : "small"}`}>
      {shown.map((asset) => (
        <AssetCell key={`${current}-${asset.symbol}`} asset={asset} />
      ))}
    </Tile>
  );
}
