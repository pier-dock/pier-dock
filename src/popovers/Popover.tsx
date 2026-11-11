import { Suspense, useEffect } from "react";
import { ipc } from "../lib/ipc";
import { lazyNamed } from "../lib/lazyNamed";
import { isGroupKey } from "../dock/order";
import { useT } from "../i18n";

// Cada janela de painel mostra um só: carregar os outros 19 seria parse e memória à toa.
const BluetoothPanel = lazyNamed(() => import("./Bluetooth"), "BluetoothPanel");
const MonthCalendar = lazyNamed(() => import("./MonthCalendar"), "MonthCalendar");
const Calendar = lazyNamed(() => import("./Calendar"), "Calendar");
const AiUsagePanel = lazyNamed(() => import("./AiUsagePanel"), "AiUsagePanel");
const AgentSessionsPanel = lazyNamed(() => import("./AgentSessionsPanel"), "AgentSessionsPanel");
const Commits = lazyNamed(() => import("./Commits"), "Commits");
const Contributions = lazyNamed(() => import("./Contributions"), "Contributions");
const KeyboardLayoutPanel = lazyNamed(() => import("./KeyboardLayout"), "KeyboardLayoutPanel");
const Launcher = lazyNamed(() => import("./Launcher"), "Launcher");
const Market = lazyNamed(() => import("./Market"), "Market");
const Notes = lazyNamed(() => import("./Notes"), "Notes");
const NowPlaying = lazyNamed(() => import("./NowPlaying"), "NowPlaying");
const PowerPanel = lazyNamed(() => import("./Power"), "PowerPanel");
const Pomodoro = lazyNamed(() => import("./Pomodoro"), "Pomodoro");
const PullRequests = lazyNamed(() => import("./PullRequests"), "PullRequests");
const SoundPanel = lazyNamed(() => import("./Sound"), "SoundPanel");
const SystemUsage = lazyNamed(() => import("./SystemUsage"), "SystemUsage");
const Weather = lazyNamed(() => import("./Weather"), "Weather");
const WifiPanel = lazyNamed(() => import("./Wifi"), "WifiPanel");
const WidgetGroupPanel = lazyNamed(() => import("./WidgetGroupPanel"), "WidgetGroupPanel");

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
    case "market":
      return <Market />;
    case "notes":
      return <Notes />;
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
    // A margem transparente em volta do painel (sombra) também recebe o clique.
    const onPointerDown = (e: PointerEvent) => {
      if (!(e.target as Element).closest(".popover")) ipc.closePanel();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointerDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointerDown);
    };
  }, []);
  if (isGroupKey(kind)) {
    return (
      <Suspense fallback={null}>
        <WidgetGroupPanel kind={kind} />
      </Suspense>
    );
  }
  return (
    <section className={`popover popover-${kind}`} aria-label={t.popover.labels[kind]} data-glass>
      <Suspense fallback={null}>
        <Content kind={kind} />
      </Suspense>
    </section>
  );
}
