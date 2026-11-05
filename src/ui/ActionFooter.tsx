import { Fragment, useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { os } from "../lib/platform";
import { dict } from "../i18n";
import { Kbd } from "./Kbd";

export interface Action {
  label: string;
  /** Atalho mostrado em chips e atendido pela janela: "↵", espaço (`keys.space`), "Ctrl <tecla>" ou "⌘ <tecla>". */
  keys?: string;
  /** Mostrado com um spinner enquanto a ação roda. */
  loadingLabel?: string;
  onClick: () => void | Promise<unknown>;
}

// Com uma resposta rápida o spinner piscaria sem ser visto, e o clique pareceria não ter funcionado.
const MIN_LOADING_MS = 500;
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const KEY_NAMES: Record<string, string> = { "⌫": "backspace" };

function matches(keys: string, e: KeyboardEvent): boolean {
  if (keys === "↵") return e.key === "Enter" && !e.ctrlKey && !e.altKey;
  if (keys === dict().keys.space) return e.key === " " && !e.ctrlKey && !e.altKey && !e.metaKey;
  const [mod, key] = keys.split(" ");
  const modifier = mod === "⌘" ? e.metaKey : mod === "Ctrl" && e.ctrlKey;
  return modifier && e.key.toLowerCase() === (KEY_NAMES[key] ?? key.toLowerCase());
}

/**
 * Rodapé de ações no padrão do Raycast: contexto à esquerda, ações à direita (a primeira é a
 * principal).
 */
export function ActionFooter({
  context,
  actions,
  keyboard = os !== "macos",
}: {
  context: ReactNode;
  actions: Action[];
  /** A janela recebe o teclado? Os popovers do macOS não recebem; as configurações, sim. */
  keyboard?: boolean;
}) {
  const [runningLabel, setRunningLabel] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const running = useRef(false);
  const run = useCallback(async (action: Action) => {
    if (running.current) return;
    running.current = true;
    setRunningLabel(action.label);
    setFailed(false);
    try { await Promise.all([action.onClick(), action.loadingLabel && delay(MIN_LOADING_MS)]); }
    catch (error) { console.error(error); setFailed(true); }
    finally { running.current = false; setRunningLabel(null); }
  }, []);
  useEffect(() => {
    if (!keyboard) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as Element).closest("input, textarea, select, button, a, [role=slider]")) return;
      const action = actions.find((a) => a.keys && matches(a.keys, e));
      if (!action) return;
      e.preventDefault();
      void run(action);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [actions, keyboard, run]);

  return (
    <footer className="action-footer">
      <span className="action-context">{failed ? <span role="alert">{dict().system.actionFailed}</span> : context}</span>
      <span className="action-list">
        {actions.map((a, i) => {
          const loading = !!a.loadingLabel && runningLabel === a.label;
          return (
            <Fragment key={a.label}>
              {i > 0 && <span className="action-sep" aria-hidden />}
              <button
                className={`action ${i === 0 ? "primary" : ""} ${loading ? "loading" : ""}`}
                disabled={runningLabel !== null}
                aria-busy={loading}
                onClick={() => void run(a)}
              >
                {loading && <span className="action-spinner" aria-hidden />}
                {loading ? a.loadingLabel : a.label}
                {!loading && keyboard && a.keys && <Kbd keys={a.keys} />}
              </button>
            </Fragment>
          );
        })}
      </span>
    </footer>
  );
}
