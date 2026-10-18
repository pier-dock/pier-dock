import type { BluetoothState, CalendarData, ClaudeUsage, GithubData, MediaState, SystemStatus, WeatherData, WidgetKind } from "../lib/ipc";
import { MediaWidget } from "./MediaWidget";
import { ContributionsWidget, GithubCommitsWidget, GithubMineWidget, GithubReviewWidget } from "./GithubWidgets";
import { BatteryWidget, BluetoothWidget, ClockWidget, NetworkWidget, ShowDesktopWidget, TrayWidget, VolumeWidget } from "./SystemWidgets";
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

export function WidgetView({ kind, wide, data }: { kind: WidgetKind; wide: boolean; data: WidgetData }) {
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
      return <ContributionsWidget data={data.github} wide={wide} />;
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
    case "show-desktop":
      return <ShowDesktopWidget />;
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
