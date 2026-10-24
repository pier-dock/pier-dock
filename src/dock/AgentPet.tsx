import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type AgentPetMascot, type AgentSession, type DockPosition } from "../lib/ipc";
import { useAgentSessions } from "../hooks/useAgentSessions";
import { usePokemonSprite } from "../hooks/usePokemonSprite";
import { useT } from "../i18n";
import { leadStatus, openAgentSessions } from "../widgets/AgentSessionsWidget";
import { barRect, isVertical } from "./layout";
import { petPose, PET_SIZE as PET, PET_EDGE_PAD as EDGE_PAD } from "./petPose";
import { ClaudeMascotIcon } from "./ClaudeMascotIcon";

type Mood = "idle" | "walking" | "waiting";

const SPEED = 46;
/** A área clicável que o Rust recebe só é atualizada quando o mascote anda isso, e sobra isso em
 * volta dele: andando, ele nunca sai da área informada. */
const HIT_STEP = 4;

/** Caixa de clique de cada sprite, relativa à caixa de 26×26 do pet. O ícone de time tem 40×30,
 * centrado na horizontal e apoiado 4px acima da base (ver `.pokemon-pet-sprite`); os 34px de
 * altura incluem o margin-bottom de 4px, então a área cobre o vão sob o sprite. */
const CLAWD_BOX = { left: 0, top: 0, width: PET, height: PET };
const POKEMON_BOX = { left: 7, top: 8, width: 40, height: 34 };

function moodOf(agents: AgentSession[]): Mood {
  if (agents.some((a) => a.status === "permission")) return "waiting";
  if (agents.some((a) => a.status === "running")) return "walking";
  return "idle";
}

interface Props {
  mascot: AgentPetMascot;
  pokemonId: number;
  position: DockPosition;
  full: boolean;
  margin: number;
  visible: boolean;
  barRef: React.RefObject<HTMLDivElement>;
}

/** Mascote que anda sobre a borda da dock, reagindo às sessões dos CLIs de IA (ver
 * `useAgentSessions`). Pode ser o Clawd ou um Pokémon; com alguma sessão, clicar nele faz o mesmo
 * que o widget. Como ele fica fora da barra, informa a própria área ao Rust (`set_pet_rect`),
 * senão o clique atravessa a janela. */
export function AgentPet({ mascot, pokemonId, position, full, margin, visible, barRef }: Props) {
  const agents = useAgentSessions();
  const vertical = isVertical(position);
  const mood = moodOf(agents);
  const lead = leadStatus(agents);
  const t = useT().agentSessions;
  const clickable = visible && agents.length > 0;
  const sprite = usePokemonSprite(mascot === "pokemon" ? pokemonId : null);
  const showPokemon = mascot === "pokemon" && !!sprite;
  const clickableRef = useRef(clickable);
  clickableRef.current = clickable;
  const reportedRef = useRef<{ x: number; y: number } | null>(null);
  const boxRef = useRef(showPokemon ? POKEMON_BOX : CLAWD_BOX);
  boxRef.current = showPokemon ? POKEMON_BOX : CLAWD_BOX;

  const outerRef = useRef<HTMLDivElement>(null);
  const posRef = useRef(0);
  const dirRef = useRef<1 | -1>(1);
  const [direction, setDirection] = useState<1 | -1>(1);
  const [barSize, setBarSize] = useState({ width: 0, height: 0 });
  // Fica parado sob o cursor, senão o clique erra o alvo andando.
  const [hovered, setHovered] = useState(false);
  // Também fica parado enquanto o painel das sessões (aberto por ele ou pelo widget) está aberto.
  const [panelOpen, setPanelOpen] = useState(false);
  const walking = visible && mood === "walking" && !(hovered && clickable) && !panelOpen;

  // Ao sair da área dele o Rust liga o click-through e o WebView não recebe o mouseleave.
  useEffect(() => {
    const subs = [
      listen("dock-pointer-left", () => setHovered(false)),
      listen<string | null>("panel-changed", (e) => setPanelOpen(e.payload === "ai-agents")),
    ];
    return () => subs.forEach((s) => s.then((unlisten) => unlisten()));
  }, []);

  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    const measure = () => setBarSize({ width: bar.offsetWidth, height: bar.offsetHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(bar);
    return () => observer.disconnect();
  }, [barRef]);

  const report = (x: number, y: number) => {
    const last = reportedRef.current;
    if (last && Math.abs(last.x - x) < HIT_STEP && Math.abs(last.y - y) < HIT_STEP) return;
    reportedRef.current = { x, y };
    const box = boxRef.current;
    ipc
      .setPetRect({
        x: x - box.left - HIT_STEP,
        y: y - box.top - HIT_STEP,
        width: box.width + HIT_STEP * 2,
        height: box.height + HIT_STEP * 2,
        radius: 0,
      })
      .catch(console.error);
  };

  const apply = () => {
    const el = outerRef.current;
    if (!el || !barSize.width || !barSize.height) return;
    const rect = barRect(position, full, margin, barSize, { width: window.innerWidth, height: window.innerHeight });
    const axis = vertical ? rect.height : rect.width;
    const bounds = Math.max(0, axis - EDGE_PAD * 2 - PET);
    if (posRef.current > bounds) posRef.current = bounds;
    el.style.setProperty("--pet-hide", `${(vertical ? barSize.width : barSize.height) + PET + 24}px`);
    const { x, y } = petPose(position, rect, posRef.current, dirRef.current);
    el.style.left = `${x}px`;
    el.style.top = `${y}px`;
    if (clickableRef.current) report(x, y);
  };

  useEffect(() => {
    apply();
    const onResize = () => apply();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [position, full, margin, barSize, vertical]);

  useEffect(() => {
    if (!clickable) return;
    apply();
    return () => {
      reportedRef.current = null;
      ipc.setPetRect(null).catch(console.error);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [clickable]);

  // Trocar de mascote (ou o sprite terminar de carregar) muda a caixa clicável sem mover o pet;
  // força um novo `set_pet_rect` com a geometria atual.
  useEffect(() => {
    reportedRef.current = null;
    apply();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showPokemon]);

  useEffect(() => {
    if (!walking) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const bar = barRef.current;
      if (bar) {
        const rect = barRect(position, full, margin, barSize, { width: window.innerWidth, height: window.innerHeight });
        const axis = vertical ? rect.height : rect.width;
        const bounds = Math.max(0, axis - EDGE_PAD * 2 - PET);
        const dt = (now - last) / 1000;
        last = now;
        if (bounds > 0) {
          let next = posRef.current + dirRef.current * SPEED * dt;
          if (next >= bounds) {
            next = bounds;
            dirRef.current = -1;
            setDirection(-1);
          } else if (next <= 0) {
            next = 0;
            dirRef.current = 1;
            setDirection(1);
          }
          posRef.current = next;
          apply();
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [walking, position, full, margin, barSize, vertical]);

  const pose = petPose(position, { x: 0, y: 0, width: 0, height: 0 }, 0, direction);
  const moodClass = walking ? "" : `claude-pet-${mood === "walking" ? "idle" : mood}`;
  return (
    <div
      ref={outerRef}
      className={`claude-pet ${visible ? "" : "claude-pet-hidden"} ${clickable ? "clickable" : ""} ${showPokemon ? "agent-pet-pokemon" : ""}`}
      role={clickable ? "button" : undefined}
      tabIndex={clickable ? 0 : undefined}
      aria-label={clickable ? t.tile(agents.length, t.status[lead ?? "running"]) : undefined}
      aria-hidden={!clickable}
      onClick={(e) => clickable && openAgentSessions(agents, e.currentTarget)}
      onKeyDown={(e) => {
        if (clickable && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          openAgentSessions(agents, e.currentTarget);
        }
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className="claude-pet-orientation" style={{ transform: `rotate(${pose.angle}deg)` }}>
        {/* O ícone de time olha para a esquerda; o `mirror` da pose vale para o Clawd, que olha para a direita. */}
        <div className={`claude-pet-facing ${showPokemon ? (pose.mirror ? "facing-right" : "facing-left") : (pose.mirror ? "facing-left" : "facing-right")}`}>
          <span className={`claude-sprite claude-pet-sprite ${showPokemon ? moodClass : walking ? "walking" : moodClass} ${lead ? `claude-${lead}` : ""}`}>
            {showPokemon && sprite ? (
              <img className={`pokemon-pet-sprite agent-pet-sprite ${walking ? "walking" : ""}`} src={sprite} alt="" draggable={false} />
            ) : (
              <ClaudeMascotIcon size={18} />
            )}
          </span>
        </div>
        {showPokemon && lead && <span className={`agent-pet-status claude-dot claude-dot-${lead}`} aria-hidden />}
      </div>
    </div>
  );
}
