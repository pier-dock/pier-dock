import { createContext, memo, useContext, useLayoutEffect, useState, type ReactNode } from "react";
import { createStore, useStore, type StoreApi } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { Config, WidgetKind, WidgetSize } from "../lib/ipc";
import { widgetSupported } from "../lib/platform";
import { useMedia } from "../hooks/useMedia";
import { useGithub } from "../hooks/useGithub";
import { useBluetooth, useKeyboardLayouts, useSystemStatus } from "../hooks/useSystem";
import { useWeather } from "../hooks/useWeather";
import { useCalendar } from "../hooks/useCalendar";
import { useAiUsage } from "../hooks/useAiUsage";
import { useMarket } from "../hooks/useMarket";
import { WidgetView } from "./WidgetView";
import { EMPTY_WIDGET_DATA, selectWidgetData, type WidgetData } from "./widgetData";

const WidgetStore = createContext<StoreApi<WidgetData> | null>(null);

/** Shared polling belongs to this boundary, so data changes do not render dock layout or apps. */
export function DockWidgetsProvider({ config, children }: { config: Config; children: ReactNode }) {
  const [store] = useState(() => createStore<WidgetData>(() => EMPTY_WIDGET_DATA));
  const enabled = (kind: WidgetKind) => config.widgets.some((w) => w.enabled && w.kind === kind && widgetSupported(w.kind));
  const media = useMedia(enabled("media"));
  const github = useGithub(config.widgets.some((w) => w.enabled && w.kind.startsWith("github-")) ? config.githubPollMinutes : undefined);
  const system = useSystemStatus(enabled("network") || enabled("battery"));
  const place = config.weatherLocation;
  const weather = useWeather(enabled("weather"), place ? `${place.latitude},${place.longitude}` : undefined);
  const calendar = useCalendar(enabled("calendar"));
  const [bluetooth] = useBluetooth(enabled("bluetooth"));
  const { keyboard } = useKeyboardLayouts(enabled("keyboard-layout"));
  const ai = useAiUsage(enabled("ai-usage") ? config.aiPollMinutes : undefined, config.aiProvider);
  const marketSymbols = `${config.marketTickers.join(",")}|${config.marketCoins.join(",")}`;
  const market = useMarket(enabled("market") ? config.marketPollMinutes : undefined, marketSymbols);
  const weatherConfigured = !!place;
  useLayoutEffect(() => {
    store.setState({ media, github, system, weather, calendar, bluetooth, keyboard, ai, market, weatherConfigured });
  }, [store, media, github, system, weather, calendar, bluetooth, keyboard, ai, market, weatherConfigured]);
  return <WidgetStore.Provider value={store}>{children}</WidgetStore.Provider>;
}

export const DockWidget = memo(function DockWidget({ kind, size }: { kind: WidgetKind; size: WidgetSize }) {
  const store = useContext(WidgetStore)!;
  const data = useStore(store, useShallow((state) => selectWidgetData(kind, state)));
  return <WidgetView kind={kind} size={size} data={data} />;
});
