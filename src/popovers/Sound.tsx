import { AppWindow, Check, Speaker, Volume1, Volume2, VolumeX } from "lucide-react";
import { ipc, type AppStream } from "../lib/ipc";
import { useAudio, useVolume } from "../hooks/useSystem";
import { useIcon } from "../hooks/useIcon";
import { ActionFooter } from "../ui/ActionFooter";
import { Slider } from "../ui/Slider";
import { useT } from "../i18n";

function VolumeIcon({ level, muted, size = 16 }: { level: number; muted: boolean; size?: number }) {
  const Icon = muted || level === 0 ? VolumeX : level < 0.5 ? Volume1 : Volume2;
  return <Icon size={size} strokeWidth={1.8} />;
}

function MuteButton({ level, muted, label, onToggle }: { level: number; muted: boolean; label: string; onToggle: () => void }) {
  return (
    <button className="icon-button sound-mute" aria-label={label} aria-pressed={muted} onClick={onToggle}>
      <VolumeIcon level={level} muted={muted} />
    </button>
  );
}

function AppRow({ app, onChange }: { app: AppStream; onChange: (patch: Partial<Pick<AppStream, "level" | "muted">>) => void }) {
  const t = useT().sound;
  const icon = useIcon(app.icon);
  return (
    <li className="sound-app">
      <span className="sound-app-icon" aria-hidden>
        {icon ? <img src={icon} alt="" /> : <AppWindow size={16} strokeWidth={1.8} />}
      </span>
      <span className="sound-app-main">
        <span className="sound-app-name">{app.name}</span>
        <Slider value={app.muted ? 0 : app.level} label={t.volumeOf(app.name)} onChange={(level) => onChange({ level, muted: false })} />
      </span>
      <MuteButton level={app.level} muted={app.muted} label={app.muted ? t.unmute : t.mute} onToggle={() => onChange({ muted: !app.muted })} />
    </li>
  );
}

export function SoundPanel() {
  const [volume, setVolume] = useVolume();
  const { audio, setOutput, setApp } = useAudio();
  const t = useT().sound;
  const level = volume?.level ?? 0;
  const muted = volume?.muted ?? false;
  const current = audio?.outputs.find((o) => o.default);

  return (
    <>
      <div className="popover-header">
        <span className="popover-heading sound-heading">{t.title}</span>
        {volume && <span className="popover-meta">{muted ? t.muted : `${Math.round(level * 100)}%`}</span>}
      </div>
      <div className="sound-body">
        <div className="sound-master">
          <MuteButton level={level} muted={muted} label={muted ? t.unmute : t.mute} onToggle={() => setVolume({ muted: !muted })} />
          <Slider value={muted ? 0 : level} label={t.master} disabled={!volume} onChange={(l) => setVolume({ level: l, muted: false })} />
        </div>

        {!!audio?.outputs.length && (
          <>
            <span className="eyebrow sound-eyebrow">{t.output}</span>
            <ul className="list sound-list" role="radiogroup" aria-label={t.output}>
              {audio.outputs.map((o) => (
                <li key={o.id}>
                  <button className="list-row sound-output" role="radio" aria-checked={o.default} onClick={() => !o.default && setOutput(o.id)}>
                    <Speaker size={16} strokeWidth={1.8} aria-hidden />
                    <span className="sound-output-name">{o.name}</span>
                    {o.default && <Check size={15} className="sound-check" aria-hidden />}
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}

        {audio?.apps && (
          <>
            <span className="eyebrow sound-eyebrow">{t.apps}</span>
            {audio.appsNeedPermission && (
              <div className="sound-permission">
                <span>{t.permission}</span>
                <button className="btn" onClick={() => ipc.openShell("audio-capture")}>
                  {t.openPermission}
                </button>
              </div>
            )}
            {audio.apps.length ? (
              <ul className="list sound-list">
                {audio.apps.map((app) => (
                  <AppRow key={app.id} app={app} onChange={(patch) => setApp(app.id, patch)} />
                ))}
              </ul>
            ) : (
              <span className="sound-none">{t.noApps}</span>
            )}
          </>
        )}
      </div>
      <ActionFooter context={current?.name ?? ""} actions={[{ label: t.settings, keys: "↵", onClick: () => ipc.openShell("sound") }]} />
    </>
  );
}
