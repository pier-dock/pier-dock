import { useEffect } from "react";
import { X } from "lucide-react";
import { ipc, type WidgetConfig } from "../lib/ipc";
import { widgetSupported } from "../lib/platform";
import { useConfig } from "../store/config";
import { useT } from "../i18n";
import { DockWidget, DockWidgetsProvider } from "../widgets/DockWidgets";
import { FitScale } from "../widgets/FitScale";
import { activeGroups, groupIdOf, isGroupKey } from "../dock/order";
import { removeFromGroup, dissolveGroup } from "../dock/groups";
import { togglePanel } from "../widgets/Tile";

/** Painel de um grupo: uma lista dos widgets de verdade, cada um com o nome embaixo. */
export function WidgetGroupPanel({ kind }: { kind: string }) {
  const config = useConfig((s) => s.config);
  const update = useConfig((s) => s.update);
  const t = useT();
  const id = isGroupKey(kind) ? groupIdOf(kind) : "";

  useEffect(() => {
    if (!config || !id) {
      void ipc.closePanel().catch(console.error);
      return;
    }
    if (!activeGroups(config).some((group) => group.id === id)) void ipc.closePanel().catch(console.error);
  }, [config, id]);

  if (!config || !id) return null;
  const group = activeGroups(config).find((g) => g.id === id);
  if (!group) return null;
  const members = group.members
    .map((member) => config.widgets.find((w) => w.kind === member))
    .filter((w): w is WidgetConfig => !!w && w.enabled && widgetSupported(w.kind));
  if (members.length < 2) return null;

  return (
    <DockWidgetsProvider config={config}>
      <section className="popover popover-group" aria-label={t.dock.groupPanelTitle} data-glass>
        <header className="group-panel-header">
          <div>
            <h1>{t.dock.groupPanelTitle}</h1>
            <span className="group-panel-count">{t.dock.groupPanelCount(members.length)}</span>
          </div>
          <div className="group-panel-actions">
            <button
              className="group-panel-ungroup"
              onClick={() => {
                update((c) => dissolveGroup(c, id));
                void ipc.closePanel().catch(console.error);
              }}
            >
              {t.dock.groupUngroup}
            </button>
            <button className="group-panel-close" aria-label={t.widgets.close} onClick={() => void ipc.closePanel().catch(console.error)}>
              <X size={16} />
            </button>
          </div>
        </header>
        <ul className="group-panel-list">
          {members.map((widget) => {
            const name = t.widgets.info[widget.kind].name;
            return (
              <li key={widget.kind} className="group-panel-item">
                <button className="group-panel-row" onClick={(e) => togglePanel(widget.kind, e.currentTarget)}>
                  <span className="group-panel-icon" aria-hidden>
                    <FitScale className="group-panel-fit">
                      <DockWidget kind={widget.kind} size={widget.kind === "github-contrib" ? "count" : "small"} />
                    </FitScale>
                  </span>
                  <span className="group-panel-name">{name}</span>
                </button>
                <button
                  className="group-panel-remove"
                  aria-label={t.dock.groupRemove(name)}
                  title={t.dock.groupRemove(name)}
                  onClick={() => update((c) => removeFromGroup(c, id, widget.kind) ?? {})}
                >
                  <X size={13} />
                </button>
              </li>
            );
          })}
        </ul>
      </section>
    </DockWidgetsProvider>
  );
}
