import { useEffect, useState } from "react";
import { X } from "lucide-react";
import { ipc, MAX_POKEMON, type AgentIntegration, type AgentProviderId, type AiProvider, type PokemonEntry, type WeatherLocation } from "../../lib/ipc";
import { usePokemonSprite } from "../../hooks/usePokemonSprite";
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

type AgentDisplay = "icon" | "both" | "pet";

const AGENT_PROVIDERS: Exclude<AgentProviderId, "antigravity" | "t3">[] = ["claude", "codex", "opencode", "gemini"];

/** Hooks/plugin de cada CLI que alimentam o widget de agents. */
export function AgentSessionsOptions({ config, update }: OptionsProps) {
  const [integrations, setIntegrations] = useState<AgentIntegration[] | null>(null);
  const [busy, setBusy] = useState<AgentProviderId | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const { widgetOptions: t, common, agentSessions } = useT();

  const refresh = () => ipc.agentIntegrations().then(setIntegrations).catch(() => setIntegrations([]));
  useEffect(() => {
    refresh();
  }, []);

  // "Só o mascote" é o widget desligado com o mascote ligado: os dois já funcionam separados, aqui só
  // ficam numa escolha só.
  const iconShown = config.widgets.some((w) => w.kind === "ai-agents" && w.enabled);
  const display: AgentDisplay = config.claudePetEnabled ? (iconShown ? "both" : "pet") : "icon";
  const setDisplay = (next: AgentDisplay) =>
    update((c) => ({
      claudePetEnabled: next !== "icon",
      widgets: c.widgets.map((w) => (w.kind === "ai-agents" ? { ...w, enabled: next !== "pet" } : w)),
    }));

  const run = (provider: AgentProviderId, action: (id: AgentProviderId) => Promise<void>) => async () => {
    setBusy(provider);
    try {
      await action(provider);
      await refresh();
      setStatus(null);
    } catch (e) {
      setStatus(common.error(String(e)));
    } finally {
      setBusy(null);
    }
  };

  const info = (provider: AgentProviderId) => integrations?.find((i) => i.id === provider);

  return (
    <>
      <Row label={t.agentIntegrationLabel} description={t.agentIntegrationHint} />
      {AGENT_PROVIDERS.map((provider) => {
        const integration = info(provider);
        const installed = integration?.installed ?? false;
        const detected = integration?.detected ?? true;
        const description = installed
          ? t.agentInstalled[provider]
          : detected
            ? (provider === "codex" ? t.agentCodexAutomatic : t.agentAvailable)
            : t.agentNotDetected;
        return (
          <Row key={provider} label={agentSessions.providers[provider]} description={description}>
            {installed && (
              <button
                className="btn btn-danger"
                disabled={busy !== null}
                onClick={run(provider, ipc.uninstallAgentIntegration)}
              >
                {provider === "codex" ? t.agentRemoveHooks : t.agentRemove}
              </button>
            )}
            <button className="btn" disabled={busy !== null || integrations === null} onClick={run(provider, ipc.installAgentIntegration)}>
              {busy === provider ? common.syncing : installed ? t.agentReinstall : t.agentInstall}
            </button>
          </Row>
        );
      })}
      {status && (
        <p className="field-hint" role="status">
          {status}
        </p>
      )}
      <Row label={t.agentMascotLabel} description={t.agentMascotHint}>
        <Segmented
          label={t.agentMascotLabel}
          value={config.agentPetMascot}
          onChange={(agentPetMascot) => update({ agentPetMascot })}
          options={[
            { value: "clawd", label: t.agentMascotClawd },
            { value: "pokemon", label: t.agentMascotPokemon },
          ]}
        />
      </Row>
      {config.agentPetMascot === "pokemon" && (
        <PokemonPicker id={config.agentPokemonId} label={t.agentPokemonChoose} onPick={(agentPokemonId) => update({ agentPokemonId })} />
      )}
      <Row label={t.agentDisplayLabel} description={t.petHint}>
        <Segmented
          label={t.agentDisplayLabel}
          value={display}
          onChange={setDisplay}
          options={(["icon", "both", "pet"] as const).map((value) => ({ value, label: t.agentDisplay[value] }))}
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
            <input id="ical" className="text-input" type="password" placeholder={t.icalPlaceholder} value={url} onChange={(e) => setUrl(e.target.value)} />
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

export const matchPokemon = (list: PokemonEntry[], query: string, limit = 8): PokemonEntry[] => {
  const q = query.trim().toLowerCase().replace(/^#/, "");
  if (!q) return [];
  return list.filter((p) => p.name.includes(q) || String(p.id) === q).slice(0, limit);
};

function PokemonChoice({ pokemon, disabled, onPick }: { pokemon: PokemonEntry; disabled?: boolean; onPick: () => void }) {
  const src = usePokemonSprite(pokemon.id);
  return (
    <button className="choice" disabled={disabled} onClick={onPick}>
      <span>
        {src && <img src={src} alt="" width={40} height={30} style={{ imageRendering: "pixelated", verticalAlign: "middle" }} />} {pokemon.name}
      </span>
      <span className="row-description">#{pokemon.id}</span>
    </button>
  );
}

function usePokemonList() {
  const [list, setList] = useState<PokemonEntry[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const { common } = useT();

  useEffect(() => {
    ipc.pokemonList().then(setList).catch((e) => setStatus(common.error(String(e))));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { list, status };
}

interface PokemonSearchProps {
  list: PokemonEntry[];
  status: string | null;
  /** Texto abaixo do campo no lugar do intervalo de ids, por exemplo quando o limite foi atingido. */
  hint?: string;
  isDisabled?: (id: number) => boolean;
  onPick: (id: number) => void;
}

function PokemonSearch({ list, status, hint, isDisabled, onPick }: PokemonSearchProps) {
  const [query, setQuery] = useState("");
  const { widgetOptions: t } = useT();
  const results = matchPokemon(list, query);

  return (
    <div className="row row-stack">
      <form className="token-form" onSubmit={(e) => e.preventDefault()}>
        <label className="visually-hidden" htmlFor="pokemon">
          {t.pokemonSearch}
        </label>
        <input id="pokemon" className="text-input" placeholder={t.pokemonSearch} value={query} onChange={(e) => setQuery(e.target.value)} />
        <p className="field-hint" role="status">
          {status ?? hint ?? (query.trim() && !results.length ? t.pokemonNone : t.pokemonRange)}
        </p>
      </form>
      {results.length > 0 && (
        <ul className="choice-list" aria-label={t.pokemonSearch}>
          {results.map((p) => (
            <li key={p.id}>
              <PokemonChoice
                pokemon={p}
                disabled={isDisabled?.(p.id)}
                onPick={() => {
                  onPick(p.id);
                  setQuery("");
                }}
              />
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function PokemonPicker({ id, label, onPick }: { id: number; label: string; onPick: (id: number) => void }) {
  const { list, status } = usePokemonList();
  const current = list.find((p) => p.id === id);

  return (
    <>
      <Row label={label} description={current ? `#${current.id} ${current.name}` : `#${id}`} />
      <PokemonSearch list={list} status={status} onPick={onPick} />
    </>
  );
}

function PokemonSelected({ id, entry, removable, onRemove }: { id: number; entry?: PokemonEntry; removable: boolean; onRemove: () => void }) {
  const src = usePokemonSprite(id);
  const { widgetOptions: t } = useT();
  const name = entry?.name ?? `#${id}`;
  return (
    <div className="list-item static">
      {src ? <img src={src} alt="" width={40} height={30} style={{ imageRendering: "pixelated" }} /> : <span style={{ width: 40, height: 30 }} />}
      <span className="list-item-name">{name}</span>
      <span className="row-description">#{id}</span>
      <button className="icon-button" aria-label={t.pokemonRemove(name)} title={t.pokemonRemove(name)} disabled={!removable} onClick={onRemove}>
        <X size={16} />
      </button>
    </div>
  );
}

/** Até `MAX_POKEMON` Pokémon, sem repetir; sempre fica ao menos um. */
function PokemonMultiPicker({ ids, onChange }: { ids: number[]; onChange: (ids: number[]) => void }) {
  const { list, status } = usePokemonList();
  const { widgetOptions: t } = useT();
  const full = ids.length >= MAX_POKEMON;

  return (
    <>
      <Row label={t.pokemonChoose(MAX_POKEMON)} description={t.pokemonSelected(ids.length, MAX_POKEMON)} />
      <div className="row row-stack">
        {ids.map((id) => (
          <PokemonSelected
            key={id}
            id={id}
            entry={list.find((p) => p.id === id)}
            removable={ids.length > 1}
            onRemove={() => onChange(ids.filter((other) => other !== id))}
          />
        ))}
      </div>
      <PokemonSearch
        list={list}
        status={status}
        hint={full ? t.pokemonLimit(MAX_POKEMON) : undefined}
        isDisabled={(id) => full || ids.includes(id)}
        onPick={(id) => onChange([...ids, id])}
      />
    </>
  );
}

/** Ícone na dock, Pokémon andando, ou os dois; como o "Mostrar na dock" das sessões de IA. */
export function PokemonOptions({ config, update }: OptionsProps) {
  const { widgetOptions: t } = useT();

  const iconShown = config.widgets.some((w) => w.kind === "pokemon" && w.enabled);
  const display: AgentDisplay = config.pokemonPetEnabled ? (iconShown ? "both" : "pet") : "icon";
  const setDisplay = (next: AgentDisplay) =>
    update((c) => ({
      pokemonPetEnabled: next !== "icon",
      widgets: c.widgets.map((w) => (w.kind === "pokemon" ? { ...w, enabled: next !== "pet" } : w)),
    }));

  return (
    <>
      <PokemonMultiPicker ids={config.pokemonPetIds} onChange={(pokemonPetIds) => update({ pokemonPetIds })} />
      <Row label={t.agentDisplayLabel} description={t.pokemonHint}>
        <Segmented
          label={t.agentDisplayLabel}
          value={display}
          onChange={setDisplay}
          options={(["icon", "both", "pet"] as const).map((value) => ({ value, label: t.pokemonDisplay[value] }))}
        />
      </Row>
    </>
  );
}
