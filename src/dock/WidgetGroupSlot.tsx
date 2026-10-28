import { useRef } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Menu } from "@tauri-apps/api/menu";
import type { WidgetConfig, WidgetGroup } from "../lib/ipc";
import { widgetSupported } from "../lib/platform";
import { useT } from "../i18n";
import { useConfig } from "../store/config";
import { DockWidget } from "../widgets/DockWidgets";
import { FitScale } from "../widgets/FitScale";
import { dissolveGroup } from "./groups";
import { groupKey } from "./order";
import { togglePanel, useHover } from "../widgets/Tile";
import { Tooltip } from "../ui/Tooltip";
import { groupPanelSize } from "../popovers/sizes";

interface Props {
  group: WidgetGroup;
  widgets: WidgetConfig[];
  vertical: boolean;
  merge?: "pending" | "ready";
}

/** Cartão 2×2 de um grupo na dock; as prévias são inertes e o clique abre o painel do grupo. */
export function WidgetGroupSlot({ group, widgets, vertical, merge }: Props) {
  const t = useT();
  const update = useConfig((s) => s.update);
  const ref = useRef<HTMLButtonElement>(null);
  const { hover, bind } = useHover();
  const { listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: groupKey(group.id) });
  const members = group.members
    .map((kind) => widgets.find((w) => w.kind === kind))
    .filter((w): w is WidgetConfig => !!w && w.enabled && widgetSupported(w.kind));
  if (members.length < 2) return null;

  const names = members.map((w) => t.widgets.info[w.kind].name).join(", ");
  const label = t.dock.groupLabel(names);
  const previews = members.slice(0, members.length > 4 ? 3 : 4);
  const more = members.length > 4 ? members.length - 3 : 0;
  const cells: (WidgetConfig | number | null)[] = [...previews, ...(more ? [more] : [])];
  while (cells.length < 4) cells.push(null);

  const open = () => ref.current && togglePanel(groupKey(group.id), ref.current, groupPanelSize(members.length));
  const onContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const menu = await Menu.new({
      items: [
        { id: "open", text: t.dock.groupOpen, action: open },
        { id: "ungroup", text: t.dock.groupUngroup, action: () => update((c) => dissolveGroup(c, group.id)) },
      ],
    });
    await menu.popup();
  };

  return (
    <div
      ref={setNodeRef}
      className={`widget-slot ${isDragging ? "dragging" : ""} ${merge ? `merge-${merge}` : ""}`}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      onContextMenu={onContextMenu}
      {...listeners}
    >
      <button
        ref={ref}
        type="button"
        className="widget-group"
        data-widget
        aria-haspopup="dialog"
        aria-label={label}
        onClick={open}
        {...bind}
      >
        {hover && <Tooltip label={label} wrap />}
        <div className="widget-group-grid" aria-hidden ref={(el) => el && (el.inert = true)}>
          {cells.map((cell, i) =>
            cell === null ? (
              <span key={`empty-${i}`} className="widget-group-cell empty" />
            ) : typeof cell === "number" ? (
              <span key="more" className="widget-group-cell widget-group-more">
                +{cell}
              </span>
            ) : (
              <span key={cell.kind} className="widget-group-cell">
                <FitScale>
                  <DockWidget kind={cell.kind} size={vertical ? (cell.kind === "github-contrib" ? "count" : "small") : cell.size} />
                </FitScale>
              </span>
            ),
          )}
        </div>
      </button>
    </div>
  );
}
