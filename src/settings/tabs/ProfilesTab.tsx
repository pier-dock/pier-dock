import { useState } from "react";
import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { addEmptyProfile, duplicateProfile, moveProfile, patchForProfile, profileView, removeProfile, renameProfile, switchProfile } from "../../dock/profiles";
import { profileIcon, PROFILE_ICONS } from "../../dock/profileIcons";
import { ACCENTS } from "../../theme/accents";
import { Card, Row, SectionLabel } from "../../ui/Card";
import { Segmented } from "../../ui/Segmented";
import { Switch } from "../../ui/Switch";
import type { TabProps } from "../shared";
import { useT } from "../../i18n";

export function ProfilesTab({ config, update, editProfile }: TabProps) {
  const dict = useT();
  const t = dict.profiles;
  const [selected, setSelected] = useState<string | null>(null);
  const [creating, setCreating] = useState<{ name: string; from: "empty" | "copy" } | null>(null);
  const [confirming, setConfirming] = useState(false);
  const profiles = config.dockProfiles;
  const current = profiles.find((p) => p.id === selected) ?? profiles.find((p) => p.id === config.activeProfile) ?? profiles[0];
  const view = profileView(config, current.id);
  const index = profiles.indexOf(current);
  const widgetsOn = view.widgets.filter((w) => w.enabled).length;
  const allMonitors = config.dockMonitor === "all";
  const CurrentIcon = profileIcon(current.icon);

  const create = () => {
    const name = creating?.name.trim();
    if (!creating || !name) return;
    const made = creating.from === "copy" ? duplicateProfile(config, current.id, name) : addEmptyProfile(config, name);
    if (!made) return;
    update(made.patch);
    setSelected(made.id);
    setCreating(null);
  };

  return (
    <div className="split">
      <div className="split-list">
        <div className="list-scroll">
          <SectionLabel aside={profiles.length}>{t.list}</SectionLabel>
          <div className="profile-list">
          {profiles.map((p) => {
            const Icon = profileIcon(p.icon);
            return (
              <button
                key={p.id}
                className="list-item"
                aria-current={p.id === current.id}
                onClick={() => {
                  setSelected(p.id);
                  setConfirming(false);
                }}
              >
                <span className="row-icon glyph"><Icon size={14} aria-hidden /></span>
                <span className="list-item-name">{p.name}</span>
                {p.id === config.activeProfile && <span className="row-value">{t.active}</span>}
              </button>
            );
          })}
          </div>
          {creating ? (
            <form
              className="profile-new"
              onSubmit={(e) => {
                e.preventDefault();
                create();
              }}
            >
              <label className="field">
                <span className="field-label">{t.name}</span>
                <input className="text-input" autoFocus placeholder={t.namePlaceholder} value={creating.name} onChange={(e) => setCreating({ ...creating, name: e.target.value })} />
              </label>
              <Segmented
                label={t.startFrom}
                value={creating.from}
                onChange={(from) => setCreating({ ...creating, from })}
                options={[
                  { value: "empty", label: t.empty },
                  { value: "copy", label: t.copyOf(current.name) },
                ]}
              />
              <div className="profile-new-actions">
                <button type="button" className="btn" onClick={() => setCreating(null)}>{t.cancel}</button>
                <button type="submit" className="btn btn-primary" disabled={!creating.name.trim()}>{t.create}</button>
              </div>
            </form>
          ) : (
            <button className="btn-text profile-add" onClick={() => setCreating({ name: "", from: "empty" })}>
              <Plus size={14} aria-hidden /> {t.new}
            </button>
          )}
          <div className="section-label profile-behavior-label">{t.behavior}</div>
          <Card>
            <Row label={t.scroll}>
              <Switch checked={config.profileScroll} label={t.scroll} onChange={(profileScroll) => update({ profileScroll })} />
            </Row>
            <Row label={t.invert}>
              <Switch checked={config.profileScrollInvert} label={t.invert} disabled={!config.profileScroll} onChange={(profileScrollInvert) => update({ profileScrollInvert })} />
            </Row>
            <Row label={t.perMonitor} description={allMonitors ? t.perMonitorHint : t.perMonitorOff}>
              <Switch checked={config.profilePerMonitor && allMonitors} label={t.perMonitor} disabled={!allMonitors} onChange={(profilePerMonitor) => update({ profilePerMonitor })} />
            </Row>
          </Card>
        </div>
      </div>
      <div className="split-detail">
        <div className="detail-head">
          <span className="row-icon glyph large"><CurrentIcon size={24} aria-hidden /></span>
          <div className="detail-title">
            <label className="visually-hidden" htmlFor="profile-name">{t.name}</label>
            <input
              id="profile-name"
              className="detail-name profile-name"
              value={current.name}
              onChange={(e) => update(renameProfile(config, current.id, { name: e.target.value }))}
            />
          </div>
          {current.id !== config.activeProfile && (
            <button className="btn btn-primary" onClick={() => update(switchProfile(config, current.id))}>{t.use}</button>
          )}
        </div>
        <Card>
          <Row label={t.icon}>
            <div className="profile-icons" role="radiogroup" aria-label={t.icon}>
              {Object.entries(PROFILE_ICONS).map(([name, Icon]) => {
                const label = t.icons[name as keyof typeof t.icons];
                return (
                  <button key={name} role="radio" aria-checked={current.icon === name} aria-label={label} title={label} onClick={() => update(renameProfile(config, current.id, { icon: name }))}>
                    <Icon size={16} aria-hidden />
                  </button>
                );
              })}
            </div>
          </Row>
          <Row label={t.accent}>
            <div className="swatches" role="radiogroup" aria-label={t.accent}>
              {ACCENTS.map((a) => (
                <button
                  key={a.key}
                  role="radio"
                  aria-checked={view.accent === a.key}
                  aria-label={dict.appearance.accents[a.key]}
                  title={dict.appearance.accents[a.key]}
                  className="swatch"
                  style={{ background: `var(--${a.key})` }}
                  onClick={() => update(patchForProfile(config, current.id, { accent: a.key }))}
                />
              ))}
            </div>
          </Row>
          <Row label={t.pinnedApps} description={t.appCount(view.pinnedApps.length)}>
            <button className="btn" onClick={() => editProfile?.(current.id, "apps")}>{t.editApps}</button>
          </Row>
          <Row label={t.widgets} description={t.widgetCount(widgetsOn)}>
            <button className="btn" onClick={() => editProfile?.(current.id, "widgets")}>{t.editWidgets}</button>
          </Row>
          <Row label={t.position} description={t.positionHint}>
            <span className="row-value">{dict.common.positionOf(index + 1, profiles.length)}</span>
            <button className="icon-button bordered" aria-label={dict.common.moveUp} disabled={index === 0} onClick={() => update(moveProfile(config, current.id, -1) ?? {})}>
              <ArrowUp size={16} />
            </button>
            <button className="icon-button bordered" aria-label={dict.common.moveDown} disabled={index === profiles.length - 1} onClick={() => update(moveProfile(config, current.id, 1) ?? {})}>
              <ArrowDown size={16} />
            </button>
          </Row>
        </Card>
        {confirming ? (
          <div className="detail-actions profile-delete" role="alert">
            <span>{t.confirmDelete(current.name)}</span>
            <button className="btn" onClick={() => setConfirming(false)}>{t.cancel}</button>
            <button
              className="btn btn-danger"
              onClick={() => {
                update(removeProfile(config, current.id) ?? {});
                setSelected(null);
                setConfirming(false);
              }}
            >
              {t.delete}
            </button>
          </div>
        ) : (
          <div className="detail-actions profile-actions">
            <button
              className="btn"
              onClick={() => {
                const made = duplicateProfile(config, current.id, t.copyName(current.name));
                if (!made) return;
                update(made.patch);
                setSelected(made.id);
              }}
            >
              {t.duplicate}
            </button>
            {profiles.length > 1 && (
              <button className="btn btn-danger" onClick={() => setConfirming(true)}>
                <Trash2 size={15} aria-hidden />
                {t.delete}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
