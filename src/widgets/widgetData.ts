import type { AiUsage, BluetoothState, CalendarData, GithubData, KeyboardLayouts, MarketData, MediaState, Note, SystemStatus, WeatherData, WidgetKind } from "../lib/ipc";

export interface WidgetData {
  media: MediaState;
  github: GithubData | null;
  system: SystemStatus | null;
  bluetooth: BluetoothState | null;
  keyboard: KeyboardLayouts | null;
  weather: WeatherData | null;
  weatherConfigured: boolean;
  calendar: CalendarData | null;
  ai: AiUsage | null;
  market: MarketData | null;
  notes: Note[];
}

export const EMPTY_WIDGET_DATA: WidgetData = {
  media: { sessions: [] }, github: null, system: null, bluetooth: null, keyboard: null,
  weather: null, weatherConfigured: false, calendar: null, ai: null, market: null, notes: [],
};

/** Select only the data consumed by this widget. All other references stay constant. */
export function selectWidgetData(kind: WidgetKind, data: WidgetData): WidgetData {
  switch (kind) {
    case "media": return { ...EMPTY_WIDGET_DATA, media: data.media };
    case "github-review": case "github-mine": case "github-commits": case "github-contrib":
      return { ...EMPTY_WIDGET_DATA, github: data.github };
    case "network": case "battery": return { ...EMPTY_WIDGET_DATA, system: data.system };
    case "bluetooth": return { ...EMPTY_WIDGET_DATA, bluetooth: data.bluetooth };
    case "keyboard-layout": return { ...EMPTY_WIDGET_DATA, keyboard: data.keyboard };
    case "weather": return { ...EMPTY_WIDGET_DATA, weather: data.weather, weatherConfigured: data.weatherConfigured };
    case "calendar": return { ...EMPTY_WIDGET_DATA, calendar: data.calendar };
    case "ai-usage": return { ...EMPTY_WIDGET_DATA, ai: data.ai };
    case "market": return { ...EMPTY_WIDGET_DATA, market: data.market };
    case "notes": return { ...EMPTY_WIDGET_DATA, notes: data.notes };
    default: return EMPTY_WIDGET_DATA;
  }
}
