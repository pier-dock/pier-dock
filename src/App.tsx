import { useEffect, useState } from "react";
import { ipc } from "./lib/ipc";
import { locale, useLang } from "./i18n";
import { currentView } from "./lib/view";
import { useConfig } from "./store/config";
import { Dock } from "./dock/Dock";
import { Popover } from "./popovers/Popover";
import { Settings } from "./settings/Settings";
import { WhatsNew } from "./whats-new/WhatsNew";

export function App() {
  const config = useConfig((s) => s.config);
  const load = useConfig((s) => s.load);
  // Assinar o idioma aqui re-renderiza a árvore toda, inclusive o que formata fora de hooks.
  const lang = useLang((s) => s.lang);

  useEffect(() => {
    load();
    document.documentElement.dataset.view = currentView.view;
  }, [load]);

  useEffect(() => {
    document.documentElement.lang = locale();
  }, [lang]);

  const [glassSupported, setGlassSupported] = useState<boolean | null>(null);
  useEffect(() => {
    ipc.platformInfo().then((p) => setGlassSupported(p.glass)).catch(() => setGlassSupported(false));
  }, []);

  useEffect(() => {
    if (!config) return;
    const root = document.documentElement;
    root.dataset.accent = config.accent;
    root.dataset.material = config.material === "glass" && glassSupported ? "glass" : "solid";
    root.style.setProperty("--dock-opacity", String(config.glassOpacity));
  }, [config?.accent, config?.material, config?.glassOpacity, glassSupported]);

  if (!config) return null;
  if (currentView.view === "panel") return <Popover kind={currentView.kind} />;
  if (currentView.view === "settings") return <Settings />;
  if (currentView.view === "whats-new") return <WhatsNew since={currentView.kind || null} />;
  return <Dock config={config} />;
}
