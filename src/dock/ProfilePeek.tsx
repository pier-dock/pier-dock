import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
import { LayoutGrid, Search } from "lucide-react";
import type { AppEntry, Config, WidgetConfig } from "../lib/ipc";
import { widgetSupported } from "../lib/platform";
import { useIcon } from "../hooks/useIcon";
import { iconKeyForEntry } from "./apps";
import { activeGroups, appKey, groupKey, isDivider, LAUNCHER_KEY, resolveOrder, START_KEY, widgetKey } from "./order";
import { profileView } from "./profiles";
import { WidgetGroupSlot } from "./WidgetGroupSlot";

function PeekApp({ entry }: { entry: AppEntry }) {
  const src = useIcon(iconKeyForEntry(entry));
  return <span className="dock-profile-peek-app">{src ? <img src={src} alt="" /> : <span className="placeholder" />}</span>;
}

/**
 * O próximo perfil durante o gesto. Fica fora da barra, que é medida por `useFitIcon` e `Magnify`, e
 * só tem a largura dela: o que passar disso só aparece depois da troca, quando a barra cresce.
 */
export function ProfilePeek({
  config,
  profileId,
  barRef,
  barStyle,
  vertical,
  renderWidget,
}: {
  config: Config;
  profileId: string;
  barRef: RefObject<HTMLElement | null>;
  barStyle: CSSProperties;
  vertical: boolean;
  renderWidget: (widget: WidgetConfig) => ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [length, setLength] = useState(0);
  useLayoutEffect(() => {
    const bar = barRef.current;
    if (bar) setLength(vertical ? bar.offsetHeight : bar.offsetWidth);
    if (ref.current) ref.current.inert = true;
  }, [barRef, vertical, profileId]);

  const view = profileView(config, profileId);
  const apps = new Map(view.pinnedApps.map((app) => [appKey(app.id), app]));
  const groups = new Map(activeGroups(view).map((group) => [groupKey(group.id), group]));
  const widgets = new Map(view.widgets.filter((w) => w.enabled && widgetSupported(w.kind)).map((w) => [widgetKey(w.kind), w]));
  const style = { ...barStyle, [vertical ? "height" : "width"]: length };

  return (
    <div ref={ref} className="dock-profile-peek" style={style} aria-hidden>
      {resolveOrder(view).map((key) => {
        if (key === START_KEY) return <span key={key} className="launcher-button"><Search size={18} strokeWidth={2} /></span>;
        if (key === LAUNCHER_KEY) return <span key={key} className="launcher-button"><LayoutGrid size={18} strokeWidth={2} /></span>;
        if (isDivider(key)) return <span key={key} className="divider-item"><span className="divider" /></span>;
        const app = apps.get(key);
        if (app) return <PeekApp key={key} entry={app} />;
        const group = groups.get(key);
        if (group) return <WidgetGroupSlot key={key} group={group} widgets={view.widgets} vertical={vertical} />;
        const widget = widgets.get(key);
        return widget ? <div key={key} className="widget-slot">{renderWidget(widget)}</div> : null;
      })}
    </div>
  );
}
