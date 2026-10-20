/**
 * Pré-visualização no navegador (`npm run dev` fora do Tauri): responde o IPC com os dados do
 * canvas de design para comparar as telas lado a lado. A view vem da URL:
 * `?view=dock`, `?view=panel&kind=media` ou `?view=settings`.
 *
 * O Vite injeta este script só no `vite dev`, antes do `main.tsx`, porque o store de config já
 * chama `listen` ao ser carregado. Dentro do Tauri ele não faz nada.
 */
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { AiUsage, AppEntry, BluetoothState, CalendarData, Config, GithubData, MediaState, PomodoroState, SystemUsage, WeatherData, WindowsSnapshot } from "../lib/ipc";
import { PANEL_INSET, PANEL_SIZES } from "../popovers/sizes";

const insideTauri = "__TAURI_INTERNALS__" in window;
const params = new URLSearchParams(location.search);
const view = params.get("view") ?? "dock";
const kind = params.get("kind") ?? "";

const apps: (AppEntry & { color: string; init: string })[] = [
  { id: "finder", name: "Finder", target: "/System/Library/CoreServices/Finder.app", color: "#2A6FDB", init: "Fi" },
  { id: "spotify", name: "Spotify", target: "/Applications/Spotify.app", color: "#1A9E4B", init: "Sp" },
  { id: "terminal", name: "Terminal", target: "/System/Applications/Utilities/Terminal.app", color: "#2B2B2F", init: ">_" },
  { id: "brave", name: "Brave Browser", target: "/Applications/Brave Browser.app", color: "#E5502B", init: "Br" },
];
const installedNames = ["Activity Monitor", "AirPort Utility", "App Store", "Audio MIDI Setup", "Automator", "Books"];
const installed: AppEntry[] = installedNames.map((name) => ({ id: name, name, target: `/Applications/${name}.app` }));

function iconDataUrl(color: string, init: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80"><rect width="80" height="80" rx="20" fill="${color}"/><text x="40" y="49" font-family="system-ui" font-weight="600" font-size="26" fill="#fff" text-anchor="middle">${init.replace(">", "&gt;")}</text></svg>`;
  return `data:image/svg+xml;base64,${btoa(svg)}`;
}

let bluetooth: BluetoothState = {
  available: true,
  powered: true,
  canToggle: true,
  canConnect: true,
  devices: [
    { id: "a1", name: "AirPods Pro", kind: "audio", battery: 82, connected: true },
    { id: "a2", name: "MX Master 3", kind: "mouse", battery: 54, connected: true },
    { id: "a3", name: "Keychron K2", kind: "keyboard", battery: null, connected: false },
    { id: "a4", name: "iPhone", kind: "phone", battery: null, connected: false },
  ],
};

const config: Config = {
  pinnedApps: apps.map(({ id, name, target }) => ({ id, name, target, exe: target, aumid: null })),
  widgets: [
    { kind: "media", enabled: true, size: "wide" },
    { kind: "github-mine", enabled: true, size: "small" },
    { kind: "github-review", enabled: true, size: "small" },
    { kind: "github-commits", enabled: false, size: "small" },
    { kind: "github-contrib", enabled: true, size: "wide" },
    { kind: "network", enabled: true, size: "small" },
    { kind: "volume", enabled: true, size: "small" },
    { kind: "bluetooth", enabled: true, size: "small" },
    { kind: "battery", enabled: true, size: "small" },
    { kind: "clock", enabled: true, size: "small" },
    { kind: "weather", enabled: params.has("new"), size: "wide" },
    { kind: "pomodoro", enabled: params.has("new"), size: "wide" },
    { kind: "system-usage", enabled: params.has("new"), size: "wide" },
    { kind: "calendar", enabled: params.has("new"), size: "wide" },
    { kind: "ai-usage", enabled: params.has("new"), size: "wide" },
    { kind: "show-desktop", enabled: params.has("new"), size: "small" },
  ],
  material: "glass",
  glassOpacity: 0.4,
  accent: "coral",
  iconSize: 40,
  magnification: 1,
  showRunningUnpinned: true,
  hideTaskbar: true,
  autoHide: "smart",
  dockWidth: "fit",
  dockPosition: "bottom",
  dockMonitor: "primary",
  dockHeight: null,
  githubPollMinutes: 3,
  aiPollMinutes: 1,
  aiProvider: "auto",
  dockOrder: [],
  showStartButton: true,
  showLauncher: false,
  launcherPinned: [...apps.slice(1, 3), installed[2]].map(({ id, name, target }) => ({ id, name, target })),
  weatherLocation: { name: "Campinas", admin: "São Paulo, Brasil", latitude: -22.9, longitude: -47.06 },
  pomodoro: { preset: "25-5", focusMinutes: 25, breakMinutes: 5 },
  language: "system",
  autoUpdate: true,
};

const windows: WindowsSnapshot = {
  windows: apps.map((a, i) => ({ id: i + 1, title: a.name, exe: a.target, minimized: false })),
  foreground: 4,
};

const media: MediaState = {
  sessions: [
    {
      id: "com.spotify.client",
      title: "Money Trees",
      artist: "Kendrick Lamar",
      album: "good kid, m.A.A.d city",
      playing: params.get("playing") === "1",
      positionMs: 147_000,
      durationMs: 386_000,
      updatedAtMs: Date.now(),
      canNext: true,
      canPrev: true,
      canSeek: true,
    },
  ],
};

/** Mesmo gerador pseudoaleatório dos quadros do canvas, para os mapas de contribuição baterem. */
function contributionWeeks(): GithubData["weeks"] {
  const start = new Date();
  start.setDate(start.getDate() - 52 * 7 - start.getDay());
  const weeks: GithubData["weeks"] = [];
  for (let w = 0; w < 53; w++) {
    const week = [];
    for (let d = 0; d < 7; d++) {
      const i = w * 7 + d;
      const date = new Date(start);
      date.setDate(start.getDate() + i);
      if (date > new Date()) break;
      const v = Math.abs(Math.sin(i * 12.9898) * 43758.5453) % 1;
      const level = v < 0.35 ? 0 : v < 0.6 ? 1 : v < 0.8 ? 2 : v < 0.93 ? 3 : 4;
      week.push({ date: date.toISOString().slice(0, 10), count: [0, 2, 5, 9, 14][level] });
    }
    weeks.push(week);
  }
  return weeks;
}

const github: GithubData = {
  login: "nahtanpng",
  avatarUrl: "",
  reviewCount: 12,
  reviewRequests: Array.from({ length: 12 }, (_, i) => ({
    number: 340 + i,
    title: ["Corrige vazamento no cache de ícones", "Adiciona suporte a MPRIS", "Refatora o layout da dock"][i % 3],
    url: `https://github.com/acme/dock/pull/${340 + i}`,
    repo: "acme/dock",
    author: ["ana", "bruno", "carla"][i % 3],
    authorAvatar: "",
    isDraft: i === 2,
    updatedAt: new Date(Date.now() - i * 3_600_000).toISOString(),
    checks: (["SUCCESS", "PENDING", "FAILURE"] as const)[i % 3],
    reviewDecision: "REVIEW_REQUIRED",
  })),
  myPrCount: 0,
  myPrs: [],
  commits: [],
  totalContributions: 2142,
  weeks: contributionWeeks(),
  fetchedAt: Date.now() - 120_000,
};

const MINUTE = 60_000;
const hourIso = (offset: number) => {
  const d = new Date(Date.now() + offset * 3_600_000);
  return `${d.toISOString().slice(0, 11)}${String(d.getHours()).padStart(2, "0")}:00`;
};
const dayIso = (offset: number) => new Date(Date.now() + offset * 86_400_000).toISOString().slice(0, 10);

const weather: WeatherData = {
  location: "Campinas",
  current: { temperature: 24.2, apparentTemperature: 25.1, humidity: 62, code: 2, isDay: true },
  hourly: [
    [2, 24, 10, true],
    [61, 23, 70, true],
    [61, 22, 80, true],
    [3, 21, 20, true],
    [0, 20, 0, false],
    [0, 19, 0, false],
  ].map(([code, temperature, precipitationProbability, isDay], i) => ({
    time: hourIso(i),
    code: code as number,
    temperature: temperature as number,
    precipitationProbability: precipitationProbability as number,
    isDay: isDay as boolean,
  })),
  daily: [
    [61, 18, 27],
    [0, 17, 29],
    [0, 19, 30],
    [3, 16, 24],
    [61, 15, 21],
  ].map(([code, min, max], i) => ({ date: dayIso(i), code, min, max })),
  fetchedAt: Date.now() - 3 * MINUTE,
};

let pomodoro: PomodoroState = {
  phase: "focus",
  running: params.get("pomodoro") !== "paused",
  started: true,
  endsAtMs: Date.now() + 18 * MINUTE + 42_000,
  remainingMs: 18 * MINUTE + 42_000,
  totalMs: 25 * MINUTE,
  session: 2,
  sessions: 4,
  focusTodayMs: 75 * MINUTE,
};

/** Mesma série pseudoaleatória dos quadros do canvas. */
function series(n: number, seed: number, base: number, amp: number): number[] {
  const out: number[] = [];
  let v = base;
  for (let i = 0; i < n; i++) {
    const r = Math.abs(Math.sin((i + 1) * seed) * 43758.5453) % 1;
    v = Math.max(3, Math.min(97, v + (r - 0.5) * amp));
    out.push(v);
  }
  return out;
}
const cpuHistory = series(60, 12.9898, 30, 18);
cpuHistory[59] = params.get("cpu") === "hot" ? 92 : 34;
const GB = 1024 ** 3;
const usage: SystemUsage = {
  cpu: cpuHistory[59],
  mem: 68,
  memUsed: 10.9 * GB,
  memTotal: 16 * GB,
  diskUsed: 412 * GB,
  diskTotal: 512 * GB,
  netRx: 2.4 * 1024 ** 2,
  netTx: 180 * 1024,
  cpuHistory,
  memHistory: series(60, 3.17, 67, 3),
};

const calendarStart = params.get("calendar") === "now" ? -12 * MINUTE : 12 * MINUTE;
const calendar: CalendarData = {
  configured: true,
  events: [
    { title: "Daily do time", start: calendarStart, length: 30, meetingUrl: "https://meet.google.com/abc-defg-hij", attendees: 6, location: "" },
    { title: "Revisão de design", start: 102 * MINUTE, length: 60, location: "Sala 2" },
    { title: "[Evento pessoal]", start: 192 * MINUTE, length: 45, location: "" },
    { title: "Planejamento da sprint", start: 24 * 60 * MINUTE, length: 60, location: "" },
  ].map((e) => ({
    id: e.title,
    title: e.title,
    startMs: Date.now() + e.start,
    endMs: Date.now() + e.start + e.length * MINUTE,
    allDay: false,
    location: e.location,
    meetingUrl: e.meetingUrl ?? null,
    attendees: e.attendees ?? 0,
  })),
  fetchedAt: Date.now() - MINUTE,
};

const claudeUsage: AiUsage = {
  provider: "claude",
  windows: [
    { id: "session", minutes: 300, percent: 42, resetsIn: 229 },
    { id: "weekly", minutes: 10080, percent: 78, resetsIn: 3_128 },
  ],
  ageMin: 4,
  stale: false,
  todayTokens: 12_400_000,
  todayMessages: 210,
  blockTokens: 3_100_000,
  topModel: "claude-opus-5",
  fetchedAt: Date.now(),
};

/** Uma conta Plus: uma janela só, a da semana, e o modelo do Codex. */
const codexUsage: AiUsage = {
  provider: "codex",
  windows: [{ id: "primary", minutes: 10080, percent: 12, resetsIn: 9_545 }],
  ageMin: 35,
  stale: true,
  todayTokens: 1_240_000,
  todayMessages: 48,
  blockTokens: 420_000,
  topModel: "gpt-5.6-luna",
  fetchedAt: Date.now(),
};

const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
  get_config: () => config,
  save_config: (args) => Object.assign(config, args.config),
  export_config: () => true,
  import_config: () => true,
  reset_config: () => undefined,
  permissions: () => [
    { id: "accessibility", status: "granted" },
    { id: "screen-recording", status: params.has("permissions") ? "not-determined" : "granted" },
    { id: "automation", status: "unknown" },
    { id: "location", status: "denied" },
    { id: "bluetooth", status: "granted" },
  ],
  permission_action: () => undefined,
  list_monitors: () => [
    { id: "Built-in Retina Display", name: "Built-in Retina Display", primary: true, width: 1512, height: 982 },
    ...(params.has("monitors") ? [{ id: "DELL U2720Q", name: "DELL U2720Q", primary: false, width: 2560, height: 1440 }] : []),
  ],
  get_windows: () => windows,
  toggle_desktop: () => null,
  list_installed_apps: () => installed,
  get_icon: (args) => {
    const app = apps.find((a) => a.target === args.key);
    return app ? iconDataUrl(app.color, app.init) : iconDataUrl("#2B2B2F", String(args.key).split("/").pop()!.slice(0, 2));
  },
  media_state: () => media,
  media_thumbnail: () => null,
  system_status: () => ({
    battery: { present: true, percent: 79, charging: false },
    network: { connected: true, internet: true, wifi: true, signalBars: 4, name: "Casa" },
  }),
  get_volume: () => ({ level: 0.6, muted: false }),
  audio_status: () => ({
    outputs: [
      { id: "speakers", name: "MacBook Air Speakers", default: true },
      { id: "headphones", name: "WH-1000XM4", default: false },
    ],
    apps: [
      { id: "1", name: "Spotify", icon: null, level: 0.8, muted: false },
      { id: "2", name: "Firefox", icon: null, level: 0.45, muted: true },
    ],
    appsNeedPermission: params.has("audio-permission"),
  }),
  wifi_status: () => ({
    available: true,
    enabled: true,
    canToggle: true,
    canConnect: true,
    needsPermission: false,
    networks: [
      { ssid: "Casa", strength: 82, signalBars: 5, secured: true, known: true, connected: true },
      { ssid: "Escritório", strength: 64, signalBars: 4, secured: true, known: true, connected: false },
      { ssid: "Café do Centro", strength: 45, signalBars: 3, secured: false, known: false, connected: false },
      { ssid: "VIVO-8F21", strength: 22, signalBars: 2, secured: true, known: false, connected: false },
    ],
  }),
  bluetooth_status: () => bluetooth,
  set_bluetooth: (args) => {
    bluetooth = { ...bluetooth, powered: Boolean(args.powered) };
    return null;
  },
  bluetooth_connect: (args) => {
    bluetooth = { ...bluetooth, devices: bluetooth.devices.map((d) => (d.id === args.id ? { ...d, connected: true } : d)) };
    return null;
  },
  bluetooth_disconnect: (args) => {
    bluetooth = { ...bluetooth, devices: bluetooth.devices.map((d) => (d.id === args.id ? { ...d, connected: false } : d)) };
    return null;
  },
  github_cached: () => github,
  github_fetch: () => github,
  github_auth_status: () => ({ source: "gh" }),
  dock_visible: () => true,
  weather_cached: () => weather,
  weather_fetch: () => weather,
  weather_search: () => [config.weatherLocation, { name: "Campinas", admin: "Mato Grosso do Sul, Brasil", latitude: -20.4, longitude: -54.6 }],
  pomodoro_state: () => pomodoro,
  pomodoro_control: (args) => {
    if (args.action === "toggle") {
      const remainingMs = pomodoro.running ? (pomodoro.endsAtMs ?? 0) - Date.now() : pomodoro.remainingMs;
      pomodoro = { ...pomodoro, running: !pomodoro.running, remainingMs, endsAtMs: Date.now() + remainingMs };
    }
    return pomodoro;
  },
  system_usage: () => usage,
  top_apps_usage: () => [
    { name: "Brave Browser", cpu: 18 },
    { name: "Spotify", cpu: 6 },
    { name: "Terminal", cpu: 4 },
    { name: "Pier", cpu: 1 },
  ],
  calendar_cached: () => calendar,
  calendar_fetch: () => calendar,
  // O preview da escolha nas configurações pede cada IA pelo nome.
  ai_usage: (args) => (args.provider === "codex" ? codexUsage : claudeUsage),
  ai_usage_cached: () => claudeUsage,
  ai_providers: () => [
    { id: "claude", detected: true },
    { id: "codex", detected: true },
  ],
  system_language: () => (navigator.language.toLowerCase().startsWith("pt") ? "pt" : "en"),
  platform_info: () => ({ os: "macos", windowList: "needs-accessibility", hideNative: true, shortcuts: true, glass: true }),
  "plugin:autostart|is_enabled": () => true,
  "plugin:app|version": () => "0.1.0",
  check_update: () => ({ version: "0.2.0", notes: null }),
  report_environment: () => ({ version: "0.1.0", os: "macOS 15.6 Sequoia", arch: "aarch64", desktop: null }),
  recent_logs: () => "[2026-10-18][10:02:11][pier][WARN] falha ao procurar atualização: timeout",
  report_issue: () => ({ kind: "created", url: "https://github.com/pier-dock/pier-dock/issues/1" }),
};

if (!insideTauri) {
  window.__PIER_VIEW__ = { view: view as "dock" | "panel" | "settings", kind };
  document.documentElement.dataset.mock = "";
  // O fundo dos quadros do canvas. No app quem borra o que está atrás é o sistema; aqui o
  // `backdrop-filter` faz esse papel.
  const style = document.createElement("style");
  style.textContent = `
    html[data-mock] body {
      background:
        radial-gradient(circle at 22% 10%, rgba(122,46,58,.9), transparent 35%),
        radial-gradient(circle at 60% 80%, rgba(30,58,138,.9), transparent 40%),
        radial-gradient(circle at 85% 5%, rgba(15,82,87,.85), transparent 30%),
        #0E0F14 !important;
    }
    html[data-mock][data-view="panel"] #root {
      position: absolute;
      left: 50%;
      bottom: 40px;
      transform: translateX(-50%);
      width: ${(PANEL_SIZES[kind]?.[0] ?? 380) + 2 * PANEL_INSET}px;
      height: ${(PANEL_SIZES[kind]?.[1] ?? 420) + 2 * PANEL_INSET}px;
    }
    html[data-mock] [data-glass] {
      backdrop-filter: blur(var(--glass-blur)) saturate(180%);
      -webkit-backdrop-filter: blur(var(--glass-blur)) saturate(180%);
    }`;
  document.head.append(style);
  mockWindows(view === "panel" ? "panel-0" : view);
  mockIPC((cmd, args) => handlers[cmd]?.((args ?? {}) as Record<string, unknown>) ?? null, { shouldMockEvents: true });
}
