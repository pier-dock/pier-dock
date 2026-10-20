import { useEffect, useState } from "react";
import { ipc, type AiProvider, type WeatherLocation } from "../../lib/ipc";
import { AI_PROVIDERS, providerName } from "../../lib/aiUsage";
import { labels } from "../../lib/platform";
import { useCalendar } from "../../hooks/useCalendar";
import { cycles } from "../../popovers/Pomodoro";
import { Row } from "../../ui/Card";
import { Segmented } from "../../ui/Segmented";
import type { TabProps } from "../shared";
import { useT } from "../../i18n";

type OptionsProps = Pick<TabProps, "config" | "update">;

const place = (l: WeatherLocation) => (l.admin ? `${l.name} · ${l.admin}` : l.name);

export function WeatherOptions({ config, update }: OptionsProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<WeatherLocation[] | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const { widgetOptions: t, common } = useT();

  const search = async () => {
    setStatus(t.searching);
    try {
      const found = await ipc.weatherSearch(query);
      setResults(found);
      setStatus(found.length ? null : t.noCity);
    } catch (e) {
      setStatus(common.error(String(e)));
    }
  };
  const choose = (location: WeatherLocation) => {
    update({ weatherLocation: location });
    setResults(null);
    setQuery("");
  };

  return (
    <>
      <Row label={t.city} description={config.weatherLocation ? place(config.weatherLocation) : t.noCityChosen} />
      <div className="row row-stack">
        <form
          className="token-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (query.trim().length >= 2) search();
          }}
        >
          <label className="visually-hidden" htmlFor="city">
            {t.searchCity}
          </label>
          <input id="city" className="text-input" placeholder={t.searchCity} value={query} onChange={(e) => setQuery(e.target.value)} />
          <button className="btn" type="submit" disabled={query.trim().length < 2}>
            {t.search}
          </button>
          <p className="field-hint" role="status">
            {status ?? t.forecastHint}
          </p>
        </form>
        {results && results.length > 0 && (
          <ul className="choice-list" aria-label={t.citiesFound}>
            {results.map((r) => (
              <li key={`${r.latitude},${r.longitude}`}>
                <button className="choice" onClick={() => choose(r)}>
                  <span>{r.name}</span>
                  <span className="row-description">{r.admin}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </>
  );
}

function MinutesInput({ label, value, min, max, onCommit }: { label: string; value: number; min: number; max: number; onCommit: (v: number) => void }) {
  const [draft, setDraft] = useState(String(value));
  const commit = () => {
    const n = Math.min(max, Math.max(min, Math.round(Number(draft) || value)));
    setDraft(String(n));
    if (n !== value) onCommit(n);
  };
  return (
    <>
      <input
        className="text-input number-input"
        type="number"
        min={min}
        max={max}
        aria-label={label}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
      />
      <span className="row-value">min</span>
    </>
  );
}

export function PomodoroOptions({ config, update }: OptionsProps) {
  const { pomodoro } = config;
  const set = (patch: Partial<typeof pomodoro>) => update((c) => ({ pomodoro: { ...c.pomodoro, ...patch } }));
  const t = useT().widgetOptions;
  return (
    <>
      <Row label={t.cycle} description={t.cycleHint}>
        <Segmented label={t.cycle} value={pomodoro.preset} onChange={(preset) => set({ preset })} options={cycles()} />
      </Row>
      {pomodoro.preset === "custom" && (
        <>
          <Row label={t.focus}>
            <MinutesInput label={t.focusMinutes} value={pomodoro.focusMinutes} min={5} max={120} onCommit={(focusMinutes) => set({ focusMinutes })} />
          </Row>
          <Row label={t.shortBreak}>
            <MinutesInput label={t.breakMinutes} value={pomodoro.breakMinutes} min={1} max={60} onCommit={(breakMinutes) => set({ breakMinutes })} />
          </Row>
        </>
      )}
      <Row label={t.notifications} description={t.notificationsHint} />
    </>
  );
}

/**
 * Qual IA o widget lê. A escolha sempre existe; as que não têm dados nesta máquina aparecem no
 * aviso, porque aí o widget fica no estado "sem dado" mesmo depois de escolhidas.
 */
export function AiUsageOptions({ config, update }: OptionsProps) {
  const [providers, setProviders] = useState<AiProvider[] | null>(null);
  const { widgetOptions: t } = useT();

  useEffect(() => {
    ipc.aiProviders().then(setProviders).catch(() => setProviders([]));
  }, []);

  // Sem a resposta ainda, tudo conta como presente: o aviso de "não encontrado" só aparece depois
  // de o Rust confirmar.
  const detected = (id: string) => providers?.find((p) => p.id === id)?.detected ?? true;
  const name = providerName;
  const hint =
    config.aiProvider === "auto"
      ? t.providerAutoHint
      : detected(config.aiProvider)
        ? t.providerReading(name(config.aiProvider))
        : t.providerMissing(name(config.aiProvider));

  return (
    <Row label={t.provider} description={hint}>
      <Segmented
        label={t.provider}
        value={config.aiProvider}
        onChange={(aiProvider) => update({ aiProvider })}
        options={[{ value: "auto", label: t.providerAuto }, ...AI_PROVIDERS.map((id) => ({ value: id, label: name(id) }))]}
      />
    </Row>
  );
}

type ClaudeDisplay = "icon" | "both" | "pet";

/** Hooks do Claude Code que alimentam o widget de agents — instalados em `~/.claude/settings.json`. */
export function ClaudeAgentsOptions({ config, update }: OptionsProps) {
  const [installed, setInstalled] = useState<boolean | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const { widgetOptions: t, common } = useT();

  useEffect(() => {
    ipc.claudeHooksInstalled().then(setInstalled).catch(() => setInstalled(false));
  }, []);

  // "Só o Clawd" é o widget desligado com o Clawd ligado: os dois já funcionam separados, aqui só
  // ficam numa escolha só.
  const iconShown = config.widgets.some((w) => w.kind === "claude-agents" && w.enabled);
  const display: ClaudeDisplay = config.claudePetEnabled ? (iconShown ? "both" : "pet") : "icon";
  const setDisplay = (next: ClaudeDisplay) =>
    update((c) => ({
      claudePetEnabled: next !== "icon",
      widgets: c.widgets.map((w) => (w.kind === "claude-agents" ? { ...w, enabled: next !== "pet" } : w)),
    }));

  const run = (action: () => Promise<void>, next: boolean) => async () => {
    try {
      await action();
      setInstalled(next);
      setStatus(null);
    } catch (e) {
      setStatus(common.error(String(e)));
    }
  };

  return (
    <>
      <Row label={t.claudeHooksLabel} description={status ?? (installed ? t.claudeHooksOnHint : t.claudeHooksOffHint)}>
        {installed && (
          <button className="btn btn-danger" onClick={run(ipc.uninstallClaudeHooks, false)}>
            {t.claudeHooksRemove}
          </button>
        )}
        <button className="btn" disabled={installed === null} onClick={run(ipc.installClaudeHooks, true)}>
          {installed ? t.claudeHooksReinstall : t.claudeHooksInstall}
        </button>
      </Row>
      <Row label={t.claudeDisplayLabel} description={t.claudePetHint}>
        <Segmented
          label={t.claudeDisplayLabel}
          value={display}
          onChange={setDisplay}
          options={(["icon", "both", "pet"] as const).map((value) => ({ value, label: t.claudeDisplay[value] }))}
        />
      </Row>
    </>
  );
}

export function CalendarOptions() {
  const data = useCalendar(true);
  const [editing, setEditing] = useState(false);
  const [url, setUrl] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const configured = !!data?.configured;
  const { widgetOptions: t, common } = useT();

  const save = async () => {
    try {
      await ipc.calendarSetUrl(url);
      setUrl("");
      setEditing(false);
      setStatus(common.syncing);
      const fresh = await ipc.calendarFetch();
      setStatus(fresh.error ? common.error(fresh.error) : null);
    } catch (e) {
      setStatus(common.error(String(e)));
    }
  };
  const remove = async () => {
    await ipc.calendarSetUrl(null);
    await ipc.calendarFetch();
    setStatus(null);
  };
  const today = data?.events.filter((e) => new Date(e.startMs).toDateString() === new Date().toDateString()).length ?? 0;
  const description = data?.error
    ? data.error
    : configured
      ? t.calendarConnected(today)
      : t.calendarPaste;

  return (
    <>
      <Row label={t.calendarLabel} description={description}>
        {configured && (
          <button className="btn btn-danger" onClick={remove}>
            {common.remove}
          </button>
        )}
        <button className="btn" onClick={() => setEditing((e) => !e)}>
          {configured ? t.changeLink : t.connect}
        </button>
      </Row>
      {(editing || (!configured && data !== null)) && (
        <div className="row row-stack">
          <form
            className="token-form"
            onSubmit={(e) => {
              e.preventDefault();
              if (url.trim()) save();
            }}
          >
            <label className="visually-hidden" htmlFor="ical">
              {t.icalAddress}
            </label>
            <input id="ical" className="text-input" type="password" placeholder="https://… ou webcal://…" value={url} onChange={(e) => setUrl(e.target.value)} />
            <button className="btn btn-primary" type="submit" disabled={!url.trim()}>
              {common.save}
            </button>
            <p className="field-hint" role="status">
              {status ?? t.calendarHint(labels().keychain)}
            </p>
          </form>
        </div>
      )}
    </>
  );
}
