import {
  Bluetooth,
  BluetoothConnected,
  BluetoothOff,
  ChevronUp,
  Globe,
  Monitor,
  WifiHigh,
  WifiLow,
  WifiOff,
} from "lucide-react";
import { Battery, Volume1, Volume2, VolumeX, Wifi } from "../ui/DockIcons";
import { ipc, type BluetoothState, type KeyboardLayouts, type SystemStatus } from "../lib/ipc";
import { useNow, useVolume } from "../hooks/useSystem";
import { shortDate } from "../lib/format";
import { Tile } from "./Tile";
import { locale, useT } from "../i18n";

export function TrayWidget() {
  const t = useT().system;
  const open = (el: HTMLElement) => {
    const rect = el.getBoundingClientRect();
    ipc.openTray(rect.left + rect.width / 2, rect.top + rect.height / 2).catch(console.error);
  };
  return (
    <Tile kind="tray" label={t.trayShow} className="sys-tile" onClick={open}>
      <ChevronUp size={17} />
    </Tile>
  );
}

export function NetworkWidget({ status }: { status: SystemStatus | null }) {
  const net = status?.network;
  const t = useT().system;
  const bars = net?.signalBars ?? 4;
  const Icon = !net?.connected ? WifiOff : !net.wifi ? Globe : bars >= 4 ? Wifi : bars >= 2 ? WifiHigh : WifiLow;
  const state = !net?.connected ? t.offline : `${net.name || t.connected}${net.internet ? "" : t.noInternet}`;
  const label = `${t.network} · ${state}`;
  return (
    <Tile kind="network" label={label} className="sys-tile">
      <Icon size={17} />
    </Tile>
  );
}

export function VolumeWidget() {
  const [volume, setVolume] = useVolume();
  const level = volume?.level ?? 0;
  const t = useT().system;
  const Icon = !volume || volume.muted || level === 0 ? VolumeX : level < 0.5 ? Volume1 : Volume2;
  const label = volume ? t.volumeTile(Math.round(level * 100), volume.muted) : t.volume;
  return (
    <Tile
      kind="volume"
      label={label}
      className="sys-tile"
      onContextMenu={(e) => {
        e.preventDefault();
        if (volume) setVolume({ muted: !volume.muted });
      }}
      onWheel={(e) => volume && setVolume({ level: Math.min(1, Math.max(0, level + (e.deltaY < 0 ? 0.04 : -0.04))) })}
    >
      <Icon size={17} />
    </Tile>
  );
}

export function BluetoothWidget({ bluetooth }: { bluetooth: BluetoothState | null }) {
  const t = useT().bluetooth;
  if (bluetooth && !bluetooth.available) return null;
  const on = !!bluetooth?.powered;
  const names = bluetooth?.devices.filter((d) => d.connected).map((d) => d.name) ?? [];
  const Icon = !on ? BluetoothOff : names.length ? BluetoothConnected : Bluetooth;
  const label = !on ? t.off : names.length ? `Bluetooth · ${names.join(", ")}` : t.tileOn;
  return (
    <Tile kind="bluetooth" label={label} className={`sys-tile bt-tile ${on ? "" : "off"}`}>
      <Icon size={17} />
      {names.length > 0 && <span className="bt-dot" aria-hidden />}
    </Tile>
  );
}

export function KeyboardLayoutWidget({ keyboard }: { keyboard: KeyboardLayouts | null }) {
  const t = useT().system;
  const active = keyboard?.layouts.find((l) => l.id === keyboard.active);
  return (
    <Tile kind="keyboard-layout" label={active ? `${t.keyboardLayout} · ${active.name}` : t.keyboardLayout} className="sys-tile kl-tile">
      <span>{active?.code ?? "…"}</span>
    </Tile>
  );
}

export function BatteryWidget({ status }: { status: SystemStatus | null }) {
  const battery = status?.battery;
  const t = useT().system;
  if (!battery?.present) return null;
  return (
    <Tile
      kind="battery"
      label={t.battery(battery.percent, battery.charging)}
      className={`battery-tile ${battery.percent <= 15 && !battery.charging ? "low" : ""}`}
    >
      <Battery percent={battery.percent} charging={battery.charging} />
      <span className="sys-text">{battery.percent}%</span>
    </Tile>
  );
}

export function ClockWidget() {
  const now = useNow(1000);
  const time = now.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" });
  const date = shortDate(now);
  const full = now.toLocaleDateString(locale(), { weekday: "long", day: "numeric", month: "long", year: "numeric" });
  return (
    <Tile kind="clock" label={full} className="clock-tile">
      <span className="clock-time">{time}</span>
      <span className="clock-date">{date}</span>
    </Tile>
  );
}

/** Minimiza todas as janelas e mostra a área de trabalho; clicar de novo as traz de volta. */
export function ShowDesktopWidget() {
  const t = useT().system;
  return (
    <Tile kind="show-desktop" label={t.showDesktop} className="sys-tile" onClick={() => ipc.toggleDesktop().catch(console.error)}>
      <Monitor size={17} />
    </Tile>
  );
}
