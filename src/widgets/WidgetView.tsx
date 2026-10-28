import type { WidgetKind, WidgetSize } from "../lib/ipc";
import { MediaWidget } from "./MediaWidget";
import { ContributionsWidget, GithubCommitsWidget, GithubMineWidget, GithubReviewWidget } from "./GithubWidgets";
import { BatteryWidget, BluetoothWidget, ClockWidget, KeyboardLayoutWidget, NetworkWidget, ShowDesktopWidget, TrayWidget, VolumeWidget } from "./SystemWidgets";
import { WeatherWidget } from "./WeatherWidget";
import { PomodoroWidget } from "./PomodoroWidget";
import { UsageWidget } from "./UsageWidget";
import { CalendarWidget } from "./CalendarWidget";
import { AiUsageWidget } from "./AiUsageWidget";
import { AgentSessionsWidget } from "./AgentSessionsWidget";
import { PokemonWidget } from "./PokemonWidget";

import type { WidgetData } from "./widgetData";
export type { WidgetData } from "./widgetData";

/** Widgets com formato largo e compacto. */
export const RESIZABLE = new Set<WidgetKind>(["media", "github-review", "github-mine", "github-contrib", "weather", "pomodoro", "system-usage", "calendar", "ai-usage"]);

/** Formatos de um widget redimensionável, na ordem do seletor. */
export function formatsOf(kind: WidgetKind): WidgetSize[] {
  if (kind === "github-contrib") return ["wide", "small", "count"];
  if (kind === "github-review" || kind === "github-mine") return ["small", "count"];
  return ["wide", "small"];
}

export function WidgetView({ kind, size, data }: { kind: WidgetKind; size: WidgetSize; data: WidgetData }) {
  const wide = size === "wide";
  switch (kind) {
    case "media":
      return <MediaWidget media={data.media} wide={wide} />;
    case "github-review":
      return <GithubReviewWidget data={data.github} size={size} />;
    case "github-mine":
      return <GithubMineWidget data={data.github} size={size} />;
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
    case "show-desktop":
      return <ShowDesktopWidget />;
    case "keyboard-layout":
      return <KeyboardLayoutWidget keyboard={data.keyboard} />;
    case "weather":
      return <WeatherWidget data={data.weather} wide={wide} configured={data.weatherConfigured} />;
    case "pomodoro":
      return <PomodoroWidget wide={wide} />;
    case "system-usage":
      return <UsageWidget wide={wide} />;
    case "calendar":
      return <CalendarWidget data={data.calendar} wide={wide} />;
    case "ai-usage":
      return <AiUsageWidget data={data.ai} wide={wide} />;
    case "ai-agents":
      return <AgentSessionsWidget />;
    case "pokemon":
      return <PokemonWidget />;
  }
}
