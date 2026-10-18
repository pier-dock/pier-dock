import type { BluetoothState, CalendarData, ClaudeUsage, GithubData, MediaState, SystemStatus, WeatherData, WidgetKind, WidgetSize } from "../lib/ipc";
import { MediaWidget } from "./MediaWidget";
import { ContributionsWidget, GithubCommitsWidget, GithubMineWidget, GithubReviewWidget } from "./GithubWidgets";
import { BatteryWidget, BluetoothWidget, ClockWidget, NetworkWidget, TrayWidget, VolumeWidget } from "./SystemWidgets";
import { WeatherWidget } from "./WeatherWidget";
import { PomodoroWidget } from "./PomodoroWidget";
import { UsageWidget } from "./UsageWidget";
import { CalendarWidget } from "./CalendarWidget";
import { ClaudeWidget } from "./ClaudeWidget";

/** Dados que os widgets mostram, lidos uma vez por quem os desenha (a dock ou as configurações). */
export interface WidgetData {
  media: MediaState;
  github: GithubData | null;
  system: SystemStatus | null;
  bluetooth: BluetoothState | null;
  weather: WeatherData | null;
  weatherConfigured: boolean;
  calendar: CalendarData | null;
  claude: ClaudeUsage | null;
}

/** Widgets com formato largo e compacto. */
export const RESIZABLE = new Set<WidgetKind>(["media", "github-contrib", "weather", "pomodoro", "system-usage", "calendar", "claude-usage"]);

/** Formatos de um widget redimensionável, na ordem do seletor. */
export function formatsOf(kind: WidgetKind): WidgetSize[] {
  return kind === "github-contrib" ? ["wide", "small", "count"] : ["wide", "small"];
}

export function WidgetView({ kind, size, data }: { kind: WidgetKind; size: WidgetSize; data: WidgetData }) {
  const wide = size === "wide";
  switch (kind) {
    case "media":
      return <MediaWidget media={data.media} wide={wide} />;
    case "github-review":
      return <GithubReviewWidget data={data.github} />;
    case "github-mine":
      return <GithubMineWidget data={data.github} />;
    case "github-commits":
      return <GithubCommitsWidget data={data.github} />;
    case "github-contrib":
      return <ContributionsWidget data={data.github} size={size} />;
    case "tray":
      return <TrayWidget />;
    case "network":
      return <NetworkWidget status={data.system} />;
    case "volume":
      return <VolumeWidget />;
    case "bluetooth":
      return <BluetoothWidget bluetooth={data.bluetooth} />;
    case "battery":
      return <BatteryWidget status={data.system} />;
    case "clock":
      return <ClockWidget />;
    case "weather":
      return <WeatherWidget data={data.weather} wide={wide} configured={data.weatherConfigured} />;
    case "pomodoro":
      return <PomodoroWidget wide={wide} />;
    case "system-usage":
      return <UsageWidget wide={wide} />;
    case "calendar":
      return <CalendarWidget data={data.calendar} wide={wide} />;
    case "claude-usage":
      return <ClaudeWidget data={data.claude} wide={wide} />;
  }
}
