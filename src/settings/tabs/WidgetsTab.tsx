import { useEffect, useRef, useState, type CSSProperties } from "react";
import { ArrowDown, ArrowUp, Plus, Search, Settings, X } from "lucide-react";
import type { BluetoothState, Config, KeyboardLayouts, WidgetConfig, WidgetGroup, WidgetKind, WidgetSize } from "../../lib/ipc";
import { os, widgetSupported } from "../../lib/platform";
import { activeGroups, moveAmong, widgetKey } from "../../dock/order";
import { createGroupFromKinds, dissolveGroup, removeFromGroup } from "../../dock/groups";
import { useGithub } from "../../hooks/useGithub";
import { useMedia } from "../../hooks/useMedia";
import { useBluetooth, useSystemStatus } from "../../hooks/useSystem";
import { useWeather } from "../../hooks/useWeather";
import { useCalendar } from "../../hooks/useCalendar";
import { useAiUsage } from "../../hooks/useAiUsage";
import { RESIZABLE, WidgetView, formatsOf, type WidgetData } from "../../widgets/WidgetView";
import { FitScale } from "../../widgets/FitScale";
import { AgentSessionsOptions, AiUsageOptions, CalendarOptions, PokemonOptions, PomodoroOptions, WeatherOptions } from "./WidgetOptions";
import { Card, Row } from "../../ui/Card";
import { Segmented } from "../../ui/Segmented";
import { Switch } from "../../ui/Switch";
import type { TabProps } from "../shared";
import { dict, useT, type Dict } from "../../i18n";

type Group = keyof Dict["widgets"]["groups"];

const groupOf = (config: Config, kind: WidgetKind) => (config.widgetGroups ?? []).find((group) => group.members.includes(kind));

const groupTitle = (group: WidgetGroup, t: Dict["widgets"]) => group.members.map((kind) => t.info[kind].name).join(", ");

/** Grupo e as duas cores de fundo do palco de cada widget; nome e textos vêm do dicionário. */
const WIDGETS: Record<WidgetKind, { group: Group; tint: [string, string] }> = {
  media: { group: "nowPlaying", tint: ["#1F6B3A", "#3A2A12"] },
  "github-mine": { group: "github", tint: ["#1E2A22", "#20242C"] },
  "github-review": { group: "github", tint: ["#3A1A1E", "#20242C"] },
  "github-commits": { group: "github", tint: ["#1E2A3A", "#20242C"] },
  "github-contrib": { group: "github", tint: ["#14432A", "#20242C"] },
  tray: { group: "system", tint: ["#2A2A3A", "#1E2A4A"] },
  network: { group: "system", tint: ["#1E2A4A", "#20242C"] },
  volume: { group: "system", tint: ["#2A2A3A", "#20242C"] },
  bluetooth: { group: "system", tint: ["#1E3A6A", "#20242C"] },
  battery: { group: "system", tint: ["#1E3A2A", "#20242C"] },
  clock: { group: "system", tint: ["#2A2A3A", "#3A1E2A"] },
  "show-desktop": { group: "system", tint: ["#2A2A3A", "#1E3A3A"] },
  "keyboard-layout": { group: "system", tint: ["#2A2A3A", "#1E3A4A"] },
  weather: { group: "productivity", tint: ["#1E3A6A", "#6A4A12"] },
  pomodoro: { group: "productivity", tint: ["#6A1E22", "#3A1A2A"] },
  "system-usage": { group: "system", tint: ["#3A1E6A", "#1E2A4A"] },
  calendar: { group: "productivity", tint: ["#1E2A6A", "#2A1E5A"] },
  "ai-usage": { group: "productivity", tint: ["#5A2E1E", "#20242C"] },
  "ai-agents": { group: "productivity", tint: ["#3A2E1E", "#20242C"] },
  pokemon: { group: "fun", tint: ["#3A3A12", "#1E2A4A"] },
};

const isWidget = (key: string) => key.startsWith("widget:");

/** Widgets que também têm um mascote andando pela dock; com só o mascote ligado, o ícone fica
 * desligado (`enabled: false`), mas o widget está em uso. */
const PET_FLAGS: Partial<Record<WidgetKind, "claudePetEnabled" | "pokemonPetEnabled">> = {
  "ai-agents": "claudePetEnabled",
  pokemon: "pokemonPetEnabled",
};

const petOn = (config: Config, kind: WidgetKind) => {
  const flag = PET_FLAGS[kind];
  return !!flag && config[flag];
};

const inUse = (config: Config, widget: WidgetConfig) => widget.enabled || petOn(config, widget.kind);

/** Com o widget desligado o Bluetooth não é consultado (no macOS isso pede permissão). */
const SAMPLE_BLUETOOTH: BluetoothState = { available: true, powered: true, canToggle: false, canConnect: false, devices: [] };

const SAMPLE_KEYBOARD: KeyboardLayouts = { layouts: [{ id: "sample", code: "ENG", name: "English" }], active: "sample" };

const tintStyle = (kind: WidgetKind) => {
  const [a, b] = WIDGETS[kind].tint;
  return { "--tint-a": a, "--tint-b": b } as CSSProperties;
};

/** O widget só aparece onde o sistema tem o recurso (bateria, adaptador Bluetooth). */
function available(kind: WidgetKind, data: WidgetData): boolean {
  if (kind === "battery") return data.system?.battery.present ?? true;
  if (kind === "bluetooth") return data.bluetooth?.available ?? true;
  return true;
}

/** Palco com o widget de verdade, em vidro como na dock, mas inerte (não abre painel nem pega foco). */
function Stage({ kind, sizes, data, large }: { kind: WidgetKind; sizes: WidgetSize[]; data: WidgetData; large?: boolean }) {
  const t = useT().widgets;
  return (
    <div className={`widget-stage ${large ? "large" : ""}`} style={tintStyle(kind)} aria-hidden ref={(el) => el && (el.inert = true)}>
      {large && <span className="preview-label">{t.preview}</span>}
      {available(kind, data) ? (
        <FitScale max={large ? 1.6 : 1}>
          <div className="widget-pills">
            {sizes.map((size) => (
              <div key={size} className="preview-bar widget-pill" data-glass>
                <WidgetView kind={kind} size={size} data={data} />
              </div>
            ))}
          </div>
        </FitScale>
      ) : (
        <span className="widget-stage-note">{t.unavailable}</span>
      )}
    </div>
  );
}

interface WidgetActions {
  toggle: (kind: WidgetKind) => void;
  set: (kind: WidgetKind, patch: Partial<WidgetConfig>) => void;
}

function WidgetCard({ widget, active, data, actions, onConfigure }: { widget: WidgetConfig; active: boolean; data: WidgetData; actions: WidgetActions; onConfigure: () => void }) {
  const t = useT().widgets;
  const info = t.info[widget.kind];
  const sizes = RESIZABLE.has(widget.kind) ? formatsOf(widget.kind) : [widget.size];
  return (
    <article className={`widget-card ${active ? "" : "off"}`} data-kind={widget.kind} aria-label={info.name}>
      <Stage kind={widget.kind} sizes={sizes} data={data} />
      <div className="widget-card-text">
        <span className="widget-card-title">
          <span className="widget-card-name">{info.name}</span>
          <span className="widget-card-group">{t.groups[WIDGETS[widget.kind].group]}</span>
        </span>
        <span className="widget-card-summary">{info.summary}</span>
      </div>
      <div className="widget-card-actions">
        <Switch checked={active} label={t.show(info.name)} onChange={() => actions.toggle(widget.kind)} />
        <span className="widget-card-state" aria-hidden>
          {active ? t.stateOn : t.stateOff}
        </span>
        <button className="icon-button bordered widget-card-configure" aria-label={t.configure(info.name)} onClick={onConfigure}>
          <Settings size={15} />
        </button>
      </div>
    </article>
  );
}

function WidgetDialog({
  widget,
  data,
  actions,
  config,
  update,
  onClose,
}: { widget: WidgetConfig; data: WidgetData; actions: WidgetActions; onClose: () => void } & Pick<TabProps, "config" | "update">) {
  const ref = useRef<HTMLDialogElement>(null);
  const { widgets: t, common, dock } = useT();
  const info = t.info[widget.kind];
  const group = groupOf(config, widget.kind);
  useEffect(() => {
    if (!ref.current?.open) ref.current?.showModal();
  }, []);

  const enabled = config.widgets.filter((w) => w.enabled && widgetSupported(w.kind) && !groupOf(config, w.kind));
  const position = enabled.findIndex((w) => w.kind === widget.kind) + 1;
  const move = (delta: -1 | 1) => {
    const patch = moveAmong(config, widgetKey(widget.kind), delta, isWidget);
    if (patch) update(patch);
  };
  const close = () => ref.current?.close();

  return (
    <dialog
      ref={ref}
      className="widget-dialog"
      aria-labelledby="widget-dialog-title"
      onClose={onClose}
      // O clique fora do conteúdo cai no próprio <dialog> (o fundo escurecido).
      onClick={(e) => e.target === ref.current && close()}
    >
      <div className="widget-dialog-content">
        <header className="widget-dialog-header">
          <span id="widget-dialog-title" className="widget-dialog-name">
            {info.name}
          </span>
          <span className="widget-card-group">{t.groups[WIDGETS[widget.kind].group]}</span>
          <button className="icon-button widget-dialog-close" aria-label={t.close} onClick={close}>
            <X size={16} />
          </button>
        </header>
        <div className="widget-dialog-body">
          <Stage kind={widget.kind} sizes={[widget.size]} data={data} large />
          <p className="widget-dialog-description">{info.description}</p>
          <Card>
            {RESIZABLE.has(widget.kind) && (
              <Row label={t.format}>
                <Segmented
                  label={t.format}
                  value={widget.size}
                  onChange={(size) => actions.set(widget.kind, { size })}
                  options={formatsOf(widget.kind).map((value) => ({ value, label: t.formats[value] }))}
                />
              </Row>
            )}
            {widget.kind === "media" && (
              <Row label={t.source} description={t.mediaSource[os]}>
                <span className="row-value">{t.automatic}</span>
              </Row>
            )}
            {widget.kind === "weather" && <WeatherOptions config={config} update={update} />}
            {widget.kind === "pomodoro" && <PomodoroOptions config={config} update={update} />}
            {widget.kind === "calendar" && <CalendarOptions />}
            {widget.kind === "ai-usage" && <AiUsageOptions config={config} update={update} />}
            {widget.kind === "ai-agents" && <AgentSessionsOptions config={config} update={update} />}
            {widget.kind === "pokemon" && <PokemonOptions config={config} update={update} />}
            {widget.enabled && group && (
              <Row label={t.position} description={dock.groupInGroup(groupTitle(group, t))}>
                <span className="row-value">{groupTitle(group, t)}</span>
              </Row>
            )}
            {widget.enabled && !group && (
              <Row label={t.position} description={common.dragInDock}>
                <span className="row-value">{common.positionOf(position, enabled.length)}</span>
                <button className="icon-button bordered" aria-label={common.moveUp} disabled={position <= 1} onClick={() => move(-1)}>
                  <ArrowUp size={16} />
                </button>
                <button className="icon-button bordered" aria-label={common.moveDown} disabled={position >= enabled.length} onClick={() => move(1)}>
                  <ArrowDown size={16} />
                </button>
              </Row>
            )}
          </Card>
        </div>
        <footer className="widget-dialog-footer">
          <Switch checked={inUse(config, widget)} label={t.show(info.name)} onChange={() => actions.toggle(widget.kind)} />
          <span aria-hidden>{t.showInDock}</span>
          <button className="btn btn-primary" onClick={close}>
            {t.done}
          </button>
        </footer>
      </div>
    </dialog>
  );
}

/** Roda `f` com o widget do card que está com o foco do teclado, para os atalhos do rodapé. */
function withFocused(f: (kind: WidgetKind) => void) {
  const kind = document.activeElement?.closest<HTMLElement>(".widget-card")?.dataset.kind;
  if (kind) f(kind as WidgetKind);
}

function CreateGroupDialog({ config, update, onClose }: Pick<TabProps, "config" | "update"> & { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  const { widgets: t, dock } = useT();
  const [selected, setSelected] = useState<WidgetKind[]>([]);
  useEffect(() => {
    if (!ref.current?.open) ref.current?.showModal();
  }, []);

  const candidates = config.widgets.filter((w) => w.enabled && widgetSupported(w.kind) && !groupOf(config, w.kind));
  const toggle = (kind: WidgetKind) =>
    setSelected((current) => (current.includes(kind) ? current.filter((item) => item !== kind) : [...current, kind]));
  const create = () => {
    if (selected.length < 2) return;
    update((c) => createGroupFromKinds(c, selected) ?? {});
    onClose();
  };

  return (
    <dialog
      ref={ref}
      className="widget-dialog group-dialog"
      aria-labelledby="group-dialog-title"
      onClose={onClose}
      onClick={(e) => e.target === ref.current && ref.current?.close()}
    >
      <div className="widget-dialog-content">
        <header className="widget-dialog-header">
          <span id="group-dialog-title" className="widget-dialog-name">
            {dock.groupCreateTitle}
          </span>
          <button className="icon-button widget-dialog-close" aria-label={t.close} onClick={() => ref.current?.close()}>
            <X size={16} />
          </button>
        </header>
        <div className="widget-dialog-body group-dialog-body">
          <p className="widget-dialog-description">{dock.groupCreateHint}</p>
          {candidates.length ? (
            <div className="group-dialog-options">
              {candidates.map((w) => (
                <label key={w.kind} className="group-dialog-option">
                  <input type="checkbox" checked={selected.includes(w.kind)} onChange={() => toggle(w.kind)} />
                  <span>{t.info[w.kind].name}</span>
                </label>
              ))}
            </div>
          ) : (
            <p className="gallery-empty">{dock.groupHint}</p>
          )}
        </div>
        <footer className="widget-dialog-footer">
          <span>{dock.groupPanelCount(selected.length)}</span>
          <button className="btn" onClick={() => ref.current?.close()}>
            {t.close}
          </button>
          <button className="btn btn-primary" disabled={selected.length < 2} onClick={create}>
            {dock.groupCreateConfirm}
          </button>
        </footer>
      </div>
    </dialog>
  );
}

export function WidgetsTab({ config, update, setFooter }: TabProps) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<WidgetKind | null>(null);
  const [groupOpen, setGroupOpen] = useState(false);
  const { widgets: t, dock } = useT();

  const enabled = (kind: WidgetKind) => config.widgets.some((w) => w.kind === kind && w.enabled);
  const [bluetooth] = useBluetooth(enabled("bluetooth"));
  const data: WidgetData = {
    media: useMedia(),
    github: useGithub(),
    system: useSystemStatus(),
    bluetooth: enabled("bluetooth") ? bluetooth : SAMPLE_BLUETOOTH,
    keyboard: SAMPLE_KEYBOARD,
    weather: useWeather(),
    weatherConfigured: !!config.weatherLocation,
    calendar: useCalendar(),
    ai: useAiUsage(),
  };

  const actions: WidgetActions = {
    toggle: (kind) =>
      update((c) => {
        const flag = PET_FLAGS[kind];
        const on = c.widgets.some((w) => w.kind === kind && inUse(c, w));
        const extra = flag && on ? { [flag]: false } : {};
        if (on) {
          const widgets = c.widgets.map((w) => (w.kind === kind ? { ...w, enabled: false } : w));
          const group = groupOf(c, kind);
          const remaining = group?.members.filter((member) => member !== kind && c.widgets.some((w) => w.kind === member && w.enabled)).length ?? 0;
          return { ...(group && remaining < 2 ? dissolveGroup(c, group.id) : {}), widgets, ...extra };
        }
        // O recém-ligado entra logo depois do último da dock, como na ordem dela (`resolveOrder`).
        const target = c.widgets.find((w) => w.kind === kind);
        if (!target) return {};
        const rest = c.widgets.filter((w) => w.kind !== kind);
        const end = rest.reduce((last, w, i) => (w.enabled ? i : last), -1) + 1;
        return { widgets: [...rest.slice(0, end), { ...target, enabled: true }, ...rest.slice(end)] };
      }),
    set: (kind, patch) => update((c) => ({ widgets: c.widgets.map((w) => (w.kind === kind ? { ...w, ...patch } : w)) })),
  };

  // Os em uso vêm primeiro; a ordenação é estável, então cada grupo mantém a ordem da dock.
  const supported = config.widgets
    .filter((w) => widgetSupported(w.kind))
    .sort((a, b) => Number(inUse(config, b)) - Number(inUse(config, a)));
  const active = supported.filter((w) => inUse(config, w)).length;
  const groups = activeGroups(config).filter(
    (group) => group.members.filter((kind) => config.widgets.some((w) => w.kind === kind && widgetSupported(w.kind))).length >= 2,
  );
  const q = query.trim().toLowerCase();
  const visible = supported.filter((w) => {
    if (!q) return true;
    const info = t.info[w.kind];
    return `${info.name} ${t.groups[WIDGETS[w.kind].group]}`.toLowerCase().includes(q);
  });

  useEffect(() => {
    const context = t.footer(supported.length, active);
    if (open) {
      setFooter({ context, actions: [] });
      return;
    }
    setFooter({
      context,
      actions: [
        { label: t.toggleAction, keys: dict().keys.space, onClick: () => withFocused(actions.toggle) },
        { label: t.configureAction, keys: "↵", onClick: () => withFocused(setOpen) },
      ],
    });
    // `actions` só repassa ao `update` do store, que não muda; o rodapé depende só destes valores.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, supported.length, active, t]);
  useEffect(() => () => setFooter(null), [setFooter]);

  const openWidget = open && config.widgets.find((w) => w.kind === open);

  return (
    <div className="widget-gallery">
      <div className="gallery-toolbar">
        <label className="gallery-search">
          <Search size={15} aria-hidden />
          <span className="visually-hidden">{t.search}</span>
          <input placeholder={t.searchPlaceholder} value={query} onChange={(e) => setQuery(e.target.value)} />
        </label>
        <span className="gallery-count">{t.inDock(active, supported.length)}</span>
        <span className="gallery-hint">{t.dragHint}</span>
      </div>
      <section className="widget-groups" aria-label={dock.groupSection}>
        <header className="widget-groups-head">
          <h2>{dock.groupSection}</h2>
          <button className="btn" onClick={() => setGroupOpen(true)}>
            <Plus size={14} aria-hidden />
            {dock.groupCreate}
          </button>
        </header>
        {groups.length ? (
          <div className="widget-groups-list">
            {groups.map((group) => {
              const members = group.members
                .map((kind) => config.widgets.find((w) => w.kind === kind))
                .filter((w): w is WidgetConfig => !!w && w.enabled && widgetSupported(w.kind));
              return (
                <article key={group.id} className="widget-group-row">
                  <div className="widget-group-row-head">
                    <span className="widget-group-row-name">{groupTitle(group, t)}</span>
                    <button className="btn widget-group-row-action" onClick={() => update((c) => dissolveGroup(c, group.id))}>
                      {dock.groupUngroup}
                    </button>
                  </div>
                  <div className="widget-group-row-members">
                    {members.map((w) => (
                      <span key={w.kind} className="widget-group-chip">
                        {t.info[w.kind].name}
                        <button
                          className="icon-button"
                          aria-label={dock.groupRemove(t.info[w.kind].name)}
                          onClick={() => update((c) => removeFromGroup(c, group.id, w.kind) ?? {})}
                        >
                          <X size={12} />
                        </button>
                      </span>
                    ))}
                  </div>
                </article>
              );
            })}
          </div>
        ) : (
          <p className="widget-groups-empty">{dock.groupHint}</p>
        )}
      </section>
      <div className="gallery-scroll">
        {visible.length ? (
          <div className="gallery-grid">
            {visible.map((w) => (
              <WidgetCard key={w.kind} widget={w} active={inUse(config, w)} data={data} actions={actions} onConfigure={() => setOpen(w.kind)} />
            ))}
          </div>
        ) : (
          <p className="gallery-empty">{t.empty(query.trim())}</p>
        )}
      </div>
      {openWidget && (
        <WidgetDialog widget={openWidget} data={data} actions={actions} config={config} update={update} onClose={() => setOpen(null)} />
      )}
      {groupOpen && <CreateGroupDialog config={config} update={update} onClose={() => setGroupOpen(false)} />}
    </div>
  );
}
