import type { AppEntry, Config } from "../../lib/ipc";
import { labels } from "../../lib/platform";
import { shortDate } from "../../lib/format";
import { useIcon } from "../../hooks/useIcon";
import { useNow } from "../../hooks/useSystem";
import { iconKeyForEntry } from "../../dock/apps";
import { ACCENTS } from "../../theme/accents";
import { Card, Row, SectionLabel } from "../../ui/Card";
import { Segmented } from "../../ui/Segmented";
import { Switch } from "../../ui/Switch";
import type { TabProps } from "../shared";
import { locale, useT } from "../../i18n";

/** Magnificação quando o usuário liga "ampliar" sem ter escolhido um valor. */
const DEFAULT_MAGNIFICATION = 1.6;

function Slider({
  id,
  label,
  value,
  display,
  min,
  max,
  step,
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  display: string;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}) {
  return (
    <div className="slider-field">
      <div className="slider-head">
        <label htmlFor={id}>{label}</label>
        <span className="mono-value">{display}</span>
      </div>
      <input id={id} type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </div>
  );
}

function PreviewIcon({ entry, size, active }: { entry: AppEntry; size: number; active: boolean }) {
  const src = useIcon(iconKeyForEntry(entry));
  return (
    <span className="preview-app">
      {src ? <img src={src} alt="" style={{ width: size, height: size }} /> : <span className="preview-fallback" style={{ width: size, height: size }} />}
      <span className={`indicator on ${active ? "active" : ""}`} />
    </span>
  );
}

/** A dock em miniatura sobre um fundo colorido, com o material, a opacidade e o destaque atuais. */
function LivePreview({ config }: { config: Config }) {
  const now = useNow(30_000);
  const apps = config.pinnedApps.slice(0, 5);
  return (
    <div className="preview-stage" aria-hidden>
      <span className="preview-window">
        <span />
        <span />
        <span />
      </span>
      <div className="preview-bar dock-like" data-glass>
        {apps.map((app, i) => (
          <PreviewIcon key={app.id} entry={app} size={config.iconSize} active={i === Math.min(apps.length - 1, 3)} />
        ))}
        <span className="divider" />
        <span className="preview-clock">
          <span className="clock-time">{now.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" })}</span>
          <span className="clock-date">{shortDate(now)}</span>
        </span>
      </div>
    </div>
  );
}

export function AppearanceTab({ config, update, platform }: TabProps) {
  const glassAvailable = platform?.glass !== false;
  const glass = config.material === "glass" && glassAvailable;
  const magnify = config.magnification > 1;
  const customHeight = config.dockHeight != null;
  const t = useT().appearance;

  return (
    <div className="split">
      <div className="split-list form">
        <div className="field">
          <SectionLabel>{t.material}</SectionLabel>
          <Segmented
            label={t.material}
            value={glass ? "glass" : "solid"}
            onChange={(material) => update({ material })}
            options={glassAvailable ? [{ value: "glass", label: t.glass }, { value: "solid", label: t.solid }] : [{ value: "solid", label: t.solid }]}
          />
          {!glassAvailable && <p className="field-hint">{t.noGlass}</p>}
        </div>
        {glass && (
          <Slider
            id="opacity"
            label={t.opacity}
            value={Math.round(config.glassOpacity * 100)}
            display={`${Math.round(config.glassOpacity * 100)}%`}
            min={15}
            max={90}
            step={5}
            onChange={(v) => update({ glassOpacity: v / 100 })}
          />
        )}
        <Slider id="icon-size" label={t.iconSize} value={config.iconSize} display={`${config.iconSize} px`} min={32} max={72} step={2} onChange={(iconSize) => update({ iconSize })} />
        <div className="field">
          <span className="field-label" id="accent-label">
            {t.accent}
          </span>
          <div className="swatches" role="radiogroup" aria-labelledby="accent-label">
            {ACCENTS.map((a) => (
              <button
                key={a.key}
                role="radio"
                aria-checked={config.accent === a.key}
                aria-label={t.accents[a.key]}
                title={t.accents[a.key]}
                className="swatch"
                style={{ background: `var(--${a.key})` }}
                onClick={() => update({ accent: a.key })}
              />
            ))}
          </div>
        </div>
        <div className="field">
          <span className="field-label">{t.autoHide}</span>
          <Segmented
            label={t.autoHide}
            value={config.autoHide}
            onChange={(autoHide) => update({ autoHide })}
            options={[
              { value: "smart", label: t.autoHideSmart },
              { value: "always", label: t.autoHideAlways },
              { value: "never", label: t.autoHideNever },
            ]}
          />
          <p className="field-hint">
            {config.autoHide === "never" ? labels().autoHideNever : t.autoHideHint}
          </p>
        </div>
        <div className="field">
          <span className="field-label">{t.dockWidth}</span>
          <Segmented
            label={t.dockWidth}
            value={config.dockWidth}
            onChange={(dockWidth) => update({ dockWidth })}
            options={[
              { value: "fit", label: t.dockWidthFit },
              { value: "full", label: t.dockWidthFull },
            ]}
          />
          <p className="field-hint">{t.dockWidthHint}</p>
        </div>
        <div className="field">
          <span className="field-label">{t.panelHeight}</span>
          <Segmented
            label={t.panelHeight}
            value={customHeight ? "custom" : "auto"}
            onChange={(mode) => update({ dockHeight: mode === "custom" ? config.iconSize + 32 : null })}
            options={[
              { value: "auto", label: t.panelAuto },
              { value: "custom", label: t.panelCustom },
            ]}
          />
          {customHeight && (
            <Slider
              id="dock-height"
              label={t.panelHeight}
              value={config.dockHeight!}
              display={`${config.dockHeight} px`}
              min={config.iconSize + 16}
              max={160}
              step={2}
              onChange={(dockHeight) => update({ dockHeight })}
            />
          )}
          <p className="field-hint">{t.panelHeightHint}</p>
        </div>
        <Card>
          <Row label={t.magnify}>
            <Switch checked={magnify} label={t.magnify} onChange={(on) => update({ magnification: on ? DEFAULT_MAGNIFICATION : 1 })} />
          </Row>
          {magnify && (
            <div className="row">
              <Slider
                id="magnification"
                label={t.magnification}
                value={config.magnification}
                display={`${config.magnification.toFixed(1)}×`}
                min={1.1}
                max={2.2}
                step={0.1}
                onChange={(magnification) => update({ magnification })}
              />
            </div>
          )}
          <Row label={t.showRunning}>
            <Switch checked={config.showRunningUnpinned} label={t.showRunning} onChange={(showRunningUnpinned) => update({ showRunningUnpinned })} />
          </Row>
        </Card>
      </div>
      <div className="split-detail">
        <SectionLabel>{t.preview}</SectionLabel>
        <LivePreview config={config} />
        <p className="field-hint">{t.blurHint}</p>
      </div>
    </div>
  );
}
