import { invoke } from "@tauri-apps/api/core";
import type { Lang, LanguageSetting } from "../i18n";
import type { Accent } from "../theme/accents";

export interface AppEntry {
  id: string;
  name: string;
  target: string;
  exe?: string | null;
  aumid?: string | null;
}

export type WidgetKind =
  | "media"
  | "github-review"
  | "github-mine"
  | "github-commits"
  | "github-contrib"
  | "tray"
  | "network"
  | "volume"
  | "bluetooth"
  | "battery"
  | "clock"
  | "show-desktop"
  | "weather"
  | "pomodoro"
  | "system-usage"
  | "calendar"
  | "ai-usage"
  | "claude-agents";

export interface WidgetConfig {
  kind: WidgetKind;
  enabled: boolean;
  size: WidgetSize;
}

/** "count" só existe nas contribuições: o total do ano, sem o mapa. */
export type WidgetSize = "small" | "wide" | "count";

export interface WeatherLocation {
  name: string;
  admin: string;
  latitude: number;
  longitude: number;
}

export interface PomodoroConfig {
  preset: "25-5" | "50-10" | "custom";
  focusMinutes: number;
  breakMinutes: number;
}

/** Borda da tela onde a dock fica. Nas laterais a barra é vertical. */
export type DockPosition = "bottom" | "top" | "left" | "right";

export interface Config {
  pinnedApps: AppEntry[];
  widgets: WidgetConfig[];
  material: "glass" | "solid";
  /** Opacidade da tinta sobre o vidro da dock (0 a 1). */
  glassOpacity: number;
  accent: Accent;
  iconSize: number;
  magnification: number;
  showRunningUnpinned: boolean;
  hideTaskbar: boolean;
  autoHide: "never" | "smart" | "always";
  /** "fit" (barra do tamanho do conteúdo) ou "full" (ponta a ponta, como um painel). */
  dockWidth: "fit" | "full";
  /** Altura da barra em pixels; `null` acompanha o tamanho do ícone (`iconSize + 24`). */
  dockHeight: number | null;
  /** Borda da tela onde a dock fica. */
  dockPosition: DockPosition;
  /** `primary` ou o id de um monitor (`MonitorInfo.id`). */
  dockMonitor: string;
  githubPollMinutes: number;
  aiPollMinutes: number;
  /** Qual IA o widget de uso lê: "auto" (a de leitura mais recente), "claude" ou "codex". */
  aiProvider: AiProviderChoice;
  /** Ordem dos itens na dock: `start`, `app:<id>`, `widget:<kind>` e `divider:<id>`. */
  dockOrder: string[];
  showStartButton: boolean;
  /** Botão do lançador de apps do próprio Pier (a lista de todos os instalados). */
  showLauncher: boolean;
  /** Apps fixados no topo do lançador, em galeria. Independe dos fixados na dock. */
  launcherPinned: AppEntry[];
  weatherLocation: WeatherLocation | null;
  pomodoro: PomodoroConfig;
  language: LanguageSetting;
  /** Instala sozinho a atualização encontrada logo depois de abrir. */
  autoUpdate: boolean;
  /** Mascote decorativo que anda sobre a borda da dock, reagindo às sessões do Claude Code. */
  claudePetEnabled: boolean;
}

export interface WindowInfo {
  /** Identificador opaco da janela (HWND no Windows, id do Mutter/X11 no Linux, CGWindowID no macOS). */
  id: number;
  title: string;
  exe: string;
  aumid?: string | null;
  minimized: boolean;
  /** Pid do processo dono da janela. */
  pid: number;
}

export interface WindowsSnapshot {
  windows: WindowInfo[];
  foreground: number;
}

export interface MediaSession {
  id: string;
  title: string;
  artist: string;
  album: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  updatedAtMs: number;
  thumbnailKey?: string | null;
  canNext: boolean;
  canPrev: boolean;
  canSeek: boolean;
}

export interface MediaState {
  sessions: MediaSession[];
  current?: string | null;
}

export interface PullRequest {
  number: number;
  title: string;
  url: string;
  repo: string;
  author: string;
  authorAvatar: string;
  isDraft: boolean;
  updatedAt: string;
  checks?: "SUCCESS" | "FAILURE" | "ERROR" | "PENDING" | "EXPECTED" | null;
  reviewDecision?: string | null;
}

export interface Commit {
  sha: string;
  message: string;
  repo: string;
  url: string;
  date: string;
}

export interface GithubData {
  login: string;
  avatarUrl: string;
  reviewCount: number;
  reviewRequests: PullRequest[];
  myPrCount: number;
  myPrs: PullRequest[];
  commits: Commit[];
  totalContributions: number;
  weeks: { date: string; count: number }[][];
  fetchedAt: number;
  error?: string | null;
}

export interface SystemStatus {
  battery: { present: boolean; percent: number; charging: boolean };
  network: { connected: boolean; internet: boolean; wifi: boolean; signalBars?: number | null; name: string };
}

export interface BluetoothDevice {
  id: string;
  name: string;
  kind: "audio" | "mouse" | "keyboard" | "gamepad" | "phone" | "other";
  battery: number | null;
  connected: boolean;
}

export interface BluetoothState {
  available: boolean;
  powered: boolean;
  canToggle: boolean;
  /** Dá para conectar e desconectar pelo Pier; senão o clique abre as configurações. */
  canConnect: boolean;
  /** Os pareados, com os conectados primeiro. */
  devices: BluetoothDevice[];
}

export interface WeatherData {
  location: string;
  current: { temperature: number; apparentTemperature: number; humidity: number; code: number; isDay: boolean } | null;
  /** A hora atual e as 5 seguintes, no horário da cidade (`2026-10-16T16:00`). */
  hourly: { time: string; temperature: number; code: number; precipitationProbability: number; isDay: boolean }[];
  daily: { date: string; code: number; min: number; max: number }[];
  fetchedAt: number;
  error?: string | null;
}

export interface PomodoroState {
  phase: "focus" | "short-break" | "long-break";
  running: boolean;
  started: boolean;
  endsAtMs?: number | null;
  remainingMs: number;
  totalMs: number;
  session: number;
  sessions: number;
  focusTodayMs: number;
}

export interface SystemUsage {
  cpu: number;
  mem: number;
  memUsed: number;
  memTotal: number;
  diskUsed: number;
  diskTotal: number;
  netRx: number;
  netTx: number;
  cpuHistory: number[];
  memHistory: number[];
}

export interface AppUsage {
  name: string;
  cpu: number;
}

export interface CalendarEvent {
  id: string;
  title: string;
  startMs: number;
  endMs: number;
  allDay: boolean;
  location: string;
  meetingUrl?: string | null;
  attendees: number;
}

export interface CalendarData {
  configured: boolean;
  events: CalendarEvent[];
  fetchedAt: number;
  error?: string | null;
}

export interface AiUsageWindow {
  /** Papel da janela na ferramenta (`session`, `weekly`, `primary`, `secondary`); só uma chave. */
  id: string;
  /** Duração da janela em minutos, quando a ferramenta informa (300 = 5h, 10080 = semana). */
  minutes: number | null;
  /** Percentual já usado. `null` é desconhecido — cache ausente ou janela que já virou. */
  percent: number | null;
  /** Minutos até o reset; negativo se a janela já passou. */
  resetsIn: number | null;
}

/** Uso de uma IA de linha de comando, já normalizado entre os provedores. */
export interface AiUsage {
  /** Qual IA respondeu: "claude" ou "codex". */
  provider: AiProviderId;
  windows: AiUsageWindow[];
  /** Idade da leitura dos limites, em minutos. */
  ageMin: number | null;
  stale: boolean;
  todayTokens: number;
  todayMessages: number;
  blockTokens: number;
  topModel: string | null;
  fetchedAt: number;
  error?: string | null;
}

/** As IAs que o widget sabe ler. */
export type AiProviderId = "claude" | "codex";
/** O que fica salvo na config: "auto" segue a IA com a leitura mais recente. */
export type AiProviderChoice = "auto" | AiProviderId;

export interface AiProvider {
  id: AiProviderId;
  /** Se os arquivos de dados da IA existem nesta máquina. */
  detected: boolean;
}

/** Estado de uma sessão do Claude Code, visto pelos hooks instalados em `~/.claude/settings.json`. */
export type ClaudeAgentState = "running" | "permission" | "error" | "done";

export interface ClaudeAgent {
  sessionId: string;
  status: ClaudeAgentState;
  cwd: string;
  project: string;
  updatedAt: number;
  /** Janela do terminal/IDE que lançou a sessão, se o Pier conseguiu resolvê-la. */
  windowId: number | null;
}

export interface Volume {
  level: number;
  muted: boolean;
}

export interface MonitorInfo {
  /** Nome do sistema; monitores com o mesmo nome ganham " 2", " 3"… da esquerda para a direita. */
  id: string;
  name: string;
  primary: boolean;
  width: number;
  height: number;
}

export interface Permission {
  id: "accessibility" | "screen-recording" | "automation" | "location" | "bluetooth";
  /** `unknown`: o sistema só responde com o app envolvido aberto (Automação do Spotify e do Música). */
  status: "granted" | "denied" | "not-determined" | "unknown";
}

export interface PlatformInfo {
  os: "windows" | "linux" | "macos";
  windowList: "ok" | "needs-extension" | "needs-relogin" | "needs-accessibility" | "x11-only" | "unsupported";
  hideNative: boolean;
  shortcuts: boolean;
  /** O sistema desenha o vidro; sem ele o material é sempre sólido. */
  glass: boolean;
  /** Dá para ler o cursor em qualquer monitor (a dock pode seguir o cursor). */
  followCursor: boolean;
  /** A área reservada acompanha a dock fora do monitor principal (falso com a extensão do GNOME antiga). */
  reserveAnyMonitor: boolean;
}

export interface UpdateInfo {
  version: string;
  notes?: string | null;
}

export type ReportKind = "bug" | "feature";

export interface ReportEnvironment {
  version: string;
  os: string;
  arch: string;
  /** Só no Linux: área de trabalho, compositor e sessão. */
  desktop: string | null;
}

/** "fallback": sem conta conectada (ou a API recusou), a página issues/new preenchida. */
export type ReportResult = { kind: "created"; url: string } | { kind: "fallback"; url: string; reason: string | null };

export interface OutputDevice {
  id: string;
  name: string;
  default: boolean;
}

/** Um app tocando áudio; `id` cobre todos os streams dele. */
export interface AppStream {
  id: string;
  name: string;
  /** Chave para o `getIcon` (caminho do executável). */
  icon: string | null;
  level: number;
  muted: boolean;
}

export interface AudioState {
  outputs: OutputDevice[];
  /** `null` quando o sistema não permite o volume por app (macOS antes do 14.2). */
  apps: AppStream[] | null;
  /** No macOS, a Gravação de Áudio do Sistema foi negada e os volumes dos apps não valem. */
  appsNeedPermission: boolean;
}

export interface WifiNetwork {
  ssid: string;
  /** 0–100. */
  strength: number;
  signalBars: number;
  secured: boolean;
  /** Tem perfil salvo: conecta sem pedir a senha. */
  known: boolean;
  connected: boolean;
}

export interface WifiState {
  available: boolean;
  enabled: boolean;
  canToggle: boolean;
  /** Sem isso, clicar numa rede abre as configurações do sistema. */
  canConnect: boolean;
  /** O sistema só mostra as redes com a permissão de Localização. */
  needsPermission: boolean;
  networks: WifiNetwork[];
}

export const ipc = {
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  /** `false` quando o usuário cancela o diálogo. */
  exportConfig: () => invoke<boolean>("export_config"),
  importConfig: () => invoke<boolean>("import_config"),
  resetConfig: () => invoke<void>("reset_config"),
  quit: () => invoke<void>("quit_app"),

  setDockLayout: (layout: { reserved: number; windowSize: number; hoverHeadroom: number }) =>
    invoke<void>("set_dock_layout", { layout }),
  setHitRect: (rect: { x: number; y: number; width: number; height: number; radius: number }) =>
    invoke<void>("set_hit_rect", { rect }),
  setPetRect: (rect: { x: number; y: number; width: number; height: number; radius: number } | null) =>
    invoke<void>("set_pet_rect", { rect }),
  setPreviewGlass: (rect: { x: number; y: number; width: number; height: number; radius: number } | null) =>
    invoke<void>("set_preview_glass", { rect }),
  dockVisible: () => invoke<boolean>("dock_visible"),
  setDockHover: (hovering: boolean) => invoke<void>("set_dock_hover", { hovering }),

  getWindows: () => invoke<WindowsSnapshot>("get_windows"),
  toggleAppWindows: (ids: number[]) => invoke<void>("toggle_app_windows", { ids }),
  activateWindow: (id: number) => invoke<void>("activate_window", { id }),
  closeWindows: (ids: number[]) => invoke<void>("close_windows", { ids }),
  toggleDesktop: () => invoke<void>("toggle_desktop"),
  windowThumbnail: (id: number) => invoke<string | null>("window_thumbnail", { id }),

  listInstalledApps: () => invoke<AppEntry[]>("list_installed_apps"),
  launchApp: (target: string) => invoke<void>("launch_app", { target }),
  resolveAppPath: (path: string) => invoke<AppEntry>("resolve_app_path", { path }),
  getIcon: (key: string) => invoke<string>("get_icon", { key }),

  mediaState: () => invoke<MediaState>("media_state"),
  mediaThumbnail: (key: string) => invoke<string | null>("media_thumbnail", { key }),
  mediaControl: (id: string, action: "toggle" | "next" | "prev" | "seek", positionMs?: number) =>
    invoke<void>("media_control", { id, action, positionMs }),

  systemStatus: () => invoke<SystemStatus>("system_status"),
  bluetoothStatus: () => invoke<BluetoothState>("bluetooth_status"),
  setBluetooth: (powered: boolean) => invoke<void>("set_bluetooth", { powered }),
  bluetoothConnect: (id: string) => invoke<void>("bluetooth_connect", { id }),
  bluetoothDisconnect: (id: string) => invoke<void>("bluetooth_disconnect", { id }),
  getVolume: () => invoke<Volume>("get_volume"),
  setVolume: (args: { level?: number; muted?: boolean }) => invoke<void>("set_volume", args),
  openTray: (anchorX: number, anchorY: number) => invoke<void>("open_tray", { anchorX, anchorY }),
  openShell: (kind: "start" | "quick-settings" | "notifications" | "system-monitor" | "calendar" | "bluetooth" | "sound" | "network" | "audio-capture") =>
    invoke<void>("open_shell", { kind }),
  audioStatus: () => invoke<AudioState>("audio_status"),
  setOutputDevice: (id: string) => invoke<void>("set_output_device", { id }),
  setAppVolume: (id: string, patch: { level?: number; muted?: boolean }) => invoke<void>("set_app_volume", { id, ...patch }),
  wifiStatus: () => invoke<WifiState>("wifi_status"),
  wifiScan: () => invoke<void>("wifi_scan"),
  wifiSetEnabled: (enabled: boolean) => invoke<void>("wifi_set_enabled", { enabled }),
  /** Rejeita com "auth" (senha errada ou faltando) ou "unsupported" (só pelas configurações do sistema). */
  wifiConnect: (ssid: string, password: string | null) => invoke<void>("wifi_connect", { ssid, password }),
  wifiDisconnect: () => invoke<void>("wifi_disconnect"),
  wifiRequestPermission: () => invoke<void>("wifi_request_permission"),

  githubFetch: () => invoke<GithubData>("github_fetch"),
  githubCached: () => invoke<GithubData | null>("github_cached"),
  githubAuthStatus: () => invoke<{ source: "pat" | "gh" | null }>("github_auth_status"),
  githubSetToken: (token: string | null) => invoke<void>("github_set_token", { token }),

  weatherFetch: () => invoke<WeatherData>("weather_fetch"),
  weatherCached: () => invoke<WeatherData | null>("weather_cached"),
  weatherSearch: (query: string) => invoke<WeatherLocation[]>("weather_search", { query }),

  pomodoroState: () => invoke<PomodoroState>("pomodoro_state"),
  pomodoroControl: (action: "toggle" | "skip" | "reset") => invoke<PomodoroState>("pomodoro_control", { action }),

  systemUsage: () => invoke<SystemUsage>("system_usage"),
  topApps: () => invoke<AppUsage[]>("top_apps_usage"),
  quitAppProcesses: (name: string) => invoke<number>("quit_app_processes", { name }),

  calendarFetch: () => invoke<CalendarData>("calendar_fetch"),
  calendarCached: () => invoke<CalendarData | null>("calendar_cached"),
  calendarSetUrl: (url: string | null) => invoke<void>("calendar_set_url", { url }),

  aiUsage: (provider: AiProviderChoice) => invoke<AiUsage>("ai_usage", { provider }),
  aiUsageCached: () => invoke<AiUsage | null>("ai_usage_cached"),
  aiProviders: () => invoke<AiProvider[]>("ai_providers"),

  getClaudeAgents: () => invoke<ClaudeAgent[]>("get_claude_agents"),
  focusClaudeAgent: (sessionId: string) => invoke<void>("focus_claude_agent", { sessionId }),
  claudeHooksInstalled: () => invoke<boolean>("claude_hooks_installed"),
  installClaudeHooks: () => invoke<void>("install_claude_hooks"),
  uninstallClaudeHooks: () => invoke<void>("uninstall_claude_hooks"),

  openPanel: (kind: string, anchorX: number, anchorY: number, width: number, height: number) =>
    invoke<void>("open_panel", { kind, anchorX, anchorY, width, height }),
  closePanel: () => invoke<void>("close_panel"),
  focusPanel: () => invoke<void>("focus_panel"),
  openSettings: () => invoke<void>("open_settings"),
  openWhatsNew: () => invoke<void>("open_whats_new"),

  platformInfo: () => invoke<PlatformInfo>("platform_info"),
  systemLanguage: () => invoke<Lang>("system_language"),
  platformSetup: (action: "install-extension" | "request-accessibility") => invoke<void>("platform_setup", { action }),
  listMonitors: () => invoke<MonitorInfo[]>("list_monitors"),
  permissions: () => invoke<Permission[]>("permissions"),
  permissionAction: (id: Permission["id"]) => invoke<void>("permission_action", { id }),

  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),

  reportEnvironment: () => invoke<ReportEnvironment>("report_environment"),
  recentLogs: () => invoke<string>("recent_logs"),
  reportIssue: (args: { kind: ReportKind; title: string; description: string; logs: string | null }) =>
    invoke<ReportResult>("report_issue", args),
};
