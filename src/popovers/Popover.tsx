import { useEffect } from "react";
import { ipc } from "../lib/ipc";
import { BluetoothPanel } from "./Bluetooth";
import { MonthCalendar } from "./MonthCalendar";
import { Calendar } from "./Calendar";
import { AiUsagePanel } from "./AiUsagePanel";
import { AgentSessionsPanel } from "./AgentSessionsPanel";
import { Commits } from "./Commits";
import { Contributions } from "./Contributions";
import { KeyboardLayoutPanel } from "./KeyboardLayout";
import { Launcher } from "./Launcher";
import { NowPlaying } from "./NowPlaying";
import { PowerPanel } from "./Power";
import { Pomodoro } from "./Pomodoro";
import { PullRequests } from "./PullRequests";
import { SoundPanel } from "./Sound";
import { SystemUsage } from "./SystemUsage";
import { Weather } from "./Weather";
import { WifiPanel } from "./Wifi";
import { WidgetGroupPanel } from "./WidgetGroupPanel";
import { isGroupKey } from "../dock/order";
import { useT } from "../i18n";

function Content({ kind }: { kind: string }) {
  const t = useT();
  switch (kind) {
    case "clock":
      return <MonthCalendar />;
    case "media":
      return <NowPlaying />;
    case "github-mine":
      return <PullRequests initial="mine" />;
    case "github-review":
      return <PullRequests initial="review" />;
    case "github-commits":
      return <Commits />;
    case "github-contrib":
      return <Contributions />;
    case "weather":
      return <Weather />;
    case "pomodoro":
      return <Pomodoro />;
    case "battery":
      return <PowerPanel />;
    case "keyboard-layout":
      return <KeyboardLayoutPanel />;
    case "system-usage":
      return <SystemUsage />;
    case "calendar":
      return <Calendar />;
    case "ai-usage":
      return <AiUsagePanel />;
    case "ai-agents":
      return <AgentSessionsPanel />;
    case "bluetooth":
      return <BluetoothPanel />;
    case "launcher":
      return <Launcher />;
    case "volume":
      return <SoundPanel />;
    case "network":
      return <WifiPanel />;
    default:
      return <div className="popover-empty">{t.popover.unknown}</div>;
  }
}

/** Popover de um widget: janela própria, ancorada acima da dock (ver `panels.rs`). */
export function Popover({ kind }: { kind: string }) {
  const t = useT();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && ipc.closePanel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  if (isGroupKey(kind)) return <WidgetGroupPanel kind={kind} />;
  return (
    <section className={`popover popover-${kind}`} aria-label={t.popover.labels[kind]} data-glass>
      <Content kind={kind} />
    </section>
  );
}
