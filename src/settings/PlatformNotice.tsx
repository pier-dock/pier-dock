import { useState } from "react";
import { ipc, type PlatformInfo } from "../lib/ipc";
import { useT } from "../i18n";

/** Faixa âmbar quando falta algo do sistema para a dock enxergar as janelas. */
export function PlatformNotice({ platform, refresh }: { platform: PlatformInfo | null; refresh: () => void }) {
  const [error, setError] = useState<string | null>(null);
  const [installed, setInstalled] = useState(false);
  const t = useT().notice;
  if (!platform || platform.windowList === "ok") return null;
  const run = async (action: "install-extension" | "request-accessibility") => {
    setError(null);
    try {
      await ipc.platformSetup(action);
      if (action === "install-extension") setInstalled(true);
      refresh();
    } catch (e) {
      setError(String(e));
    }
  };
  let text: string;
  let action: { label: string; run: () => void } | null = null;
  switch (platform.windowList) {
    case "needs-extension":
      text = installed ? t.extensionInstalled : t.needsExtension;
      if (!installed) action = { label: t.installExtension, run: () => run("install-extension") };
      break;
    case "needs-relogin":
      text = t.needsRelogin;
      break;
    case "x11-only":
      text = t.x11Only;
      break;
    case "needs-accessibility":
      text = t.needsAccessibility;
      action = { label: t.grant, run: () => run("request-accessibility") };
      break;
    default:
      text = t.unsupported;
  }
  return (
    <div className="notice" role="status">
      <span className="notice-dot" aria-hidden />
      <span className="notice-text">
        {text}
        {error && t.error(error)}
      </span>
      <button className="btn-text" onClick={refresh}>
        {t.checkAgain}
      </button>
      {action && (
        <button className="btn btn-warning" onClick={action.run}>
          {action.label}
        </button>
      )}
    </div>
  );
}
