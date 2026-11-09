import { Suspense, useEffect, useState } from "react";
import { useDockProfile } from "./dock/useDockProfile";
import { ipc } from "./lib/ipc";
import { locale, useLang } from "./i18n";
import { currentView } from "./lib/view";
import { useConfig } from "./store/config";
import { lazyNamed } from "./lib/lazyNamed";
import { LoadError } from "./ui/LoadError";

// Cada janela renderiza uma só destas telas, então as outras nem são baixadas nem parseadas.
const Dock = lazyNamed(() => import("./dock/Dock"), "Dock");
const Popover = lazyNamed(() => import("./popovers/Popover"), "Popover");
const Settings = lazyNamed(() => import("./settings/Settings"), "Settings");
const WhatsNew = lazyNamed(() => import("./whats-new/WhatsNew"), "WhatsNew");

export function App() {
  const config = useConfig((s) => s.config);
  const load = useConfig((s) => s.load);
  const loadError = useConfig((s) => s.loadError);
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

  const dockProfile = useDockProfile(config);
  const accent = currentView.view === "dock" ? dockProfile?.view.accent : config?.accent;
  const material = config?.material;
  const glassOpacity = config?.glassOpacity;
  const reducedEffects = config?.reducedEffects;
  useEffect(() => {
    if (accent === undefined) return;
    const root = document.documentElement;
    root.dataset.accent = accent;
    root.dataset.reducedEffects = String(reducedEffects);
    root.dataset.material = material === "glass" && glassSupported && !reducedEffects ? "glass" : "solid";
    root.style.setProperty("--dock-opacity", String(glassOpacity));
  }, [accent, material, glassOpacity, reducedEffects, glassSupported]);

  if (!config) return loadError ? <LoadError retry={load} /> : null;
  return (
    <Suspense fallback={null}>
      {currentView.view === "panel" ? (
        <Popover kind={currentView.kind} />
      ) : currentView.view === "settings" ? (
        <Settings />
      ) : currentView.view === "whats-new" ? (
        <WhatsNew since={currentView.kind || null} />
      ) : (
        dockProfile && <Dock config={dockProfile.view} profileId={dockProfile.id} monitor={dockProfile.monitor} />
      )}
    </Suspense>
  );
}
