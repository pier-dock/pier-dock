import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Activity, GitBranch, LayoutGrid, Menu, Palette } from "lucide-react";
import { ipc, type PlatformInfo } from "../lib/ipc";
import { currentView } from "../lib/view";
import { keys, labels, os } from "../lib/platform";
import { useConfig } from "../store/config";
import { ActionFooter, type Action } from "../ui/ActionFooter";
import { ErrorBoundary } from "../ui/ErrorBoundary";
import { Logo } from "../ui/Logo";
import { PlatformNotice } from "./PlatformNotice";
import { Welcome } from "./Welcome";
import type { TabFooter, TabProps } from "./shared";
import { AppsTab } from "./tabs/AppsTab";
import { WidgetsTab } from "./tabs/WidgetsTab";
import { AppearanceTab } from "./tabs/AppearanceTab";
import { GithubTab } from "./tabs/GithubTab";
import { GeneralTab } from "./tabs/GeneralTab";
import { refreshGithub } from "../popovers/PullRequests";
import { addDivider } from "../dock/order";
import { useT } from "../i18n";

const TABS = [
  { id: "apps", Icon: LayoutGrid, Content: AppsTab },
  { id: "widgets", Icon: Activity, Content: WidgetsTab },
  { id: "appearance", Icon: Palette, Content: AppearanceTab },
  { id: "github", Icon: GitBranch, Content: GithubTab },
  { id: "general", Icon: Menu, Content: GeneralTab },
] as const;
type TabId = (typeof TABS)[number]["id"];

const tabId = (id: string) => TABS.find((t) => t.id === id)?.id;

const OS_NAMES = { windows: "Windows", linux: "Linux", macos: "macOS" };

export function Settings() {
  const config = useConfig((s) => s.config)!;
  const update = useConfig((s) => s.update);
  const t = useT();
  const [tab, setTab] = useState<TabId>(() => tabId(currentView.kind) ?? "apps");
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [footer, setFooter] = useState<TabFooter | null>(null);
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const refreshPlatform = () => void ipc.platformInfo().then(setPlatform).catch(() => {});
  useEffect(refreshPlatform, []);
  const [firstRun, setFirstRun] = useState(false);
  useEffect(() => {
    ipc.firstRun().then(setFirstRun).catch(() => {});
  }, []);
  useEffect(() => {
    const unlisten = listen<string>("settings-tab", (e) => {
      const id = tabId(e.payload);
      if (id) setTab(id);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // Setas e Home/End andam entre as abas, como num tablist nativo.
  const onTabKey = (e: React.KeyboardEvent) => {
    const i = TABS.findIndex((t) => t.id === tab);
    const next = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: TABS.length - 1 }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    const target = TABS[(next + TABS.length) % TABS.length];
    setTab(target.id);
    tabRefs.current[target.id]?.focus();
  };

  const props: TabProps = { config, update: (p) => void update(p), platform, setFooter };
  const { Content } = TABS.find((t) => t.id === tab)!;
  const actions: Record<TabId, Action[]> = {
    apps: [{ label: t.dock.addDivider, onClick: () => update((c) => addDivider(c)) }],
    widgets: [],
    appearance: [],
    github: [{ label: t.settings.syncNow, keys: `${keys.command} R`, onClick: refreshGithub }],
    general: [{ label: labels().quit, onClick: () => ipc.quit() }],
  };

  return (
    <div className="settings" data-glass>
      <header className="settings-header" data-tauri-drag-region>
        {/* No macOS a barra de título é a do app (os semáforos ficam por cima); nos outros, a nativa. */}
        {os === "macos" && (
          <span className="settings-title" data-tauri-drag-region>
            <Logo size={16} />
            {t.settings.title}
          </span>
        )}
        <div className="settings-tabs" role="tablist" aria-label={t.settings.sections} onKeyDown={onTabKey}>
          {TABS.map(({ id, Icon }) => (
            <button
              key={id}
              ref={(el) => (tabRefs.current[id] = el)}
              id={`tab-${id}`}
              role="tab"
              aria-selected={tab === id}
              aria-controls="settings-panel"
              tabIndex={tab === id ? 0 : -1}
              className="settings-tab"
              onClick={() => setTab(id)}
            >
              <Icon size={18} strokeWidth={1.8} aria-hidden />
              {t.settings.tabs[id]}
            </button>
          ))}
        </div>
      </header>
      <PlatformNotice platform={platform} refresh={refreshPlatform} />
      {firstRun && <Welcome platform={platform} update={props.update} done={() => setFirstRun(false)} />}
      <main className="settings-main" id="settings-panel" role="tabpanel" aria-labelledby={`tab-${tab}`}>
        <ErrorBoundary key={tab}>
          <Content {...props} />
        </ErrorBoundary>
      </main>
      <ActionFooter keyboard context={footer?.context ?? `Pier · ${OS_NAMES[os]}`} actions={footer?.actions ?? actions[tab]} />
    </div>
  );
}
