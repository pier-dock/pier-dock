import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, type ClaudeAgent, type DockPosition } from "../lib/ipc";
import { useClaudeAgents } from "../hooks/useClaudeAgents";
import { useT } from "../i18n";
import { leadStatus, openClaudeAgents } from "../widgets/ClaudeAgentsWidget";
import { barRect, isVertical } from "./layout";
import { ClaudeMascotIcon } from "./ClaudeMascotIcon";

type Mood = "idle" | "walking" | "waiting";

/** Lado do sprite, para espelhar/inclinar na direção do passeio. */
type Facing = "left" | "right" | "up" | "down";

const PET = 26;
const EDGE_PAD = 10;
const OVERLAP = 6;
const SPEED = 46;
/** A área clicável que o Rust recebe só é atualizada quando o Clawd anda isso, e sobra isso em
 * volta dele: andando, ele nunca sai da área informada. */
const HIT_STEP = 4;

function moodOf(agents: ClaudeAgent[]): Mood {
  if (agents.some((a) => a.status === "permission")) return "waiting";
  if (agents.some((a) => a.status === "running")) return "walking";
  return "idle";
}

interface Props {
  position: DockPosition;
  full: boolean;
  margin: number;
  visible: boolean;
  barRef: React.RefObject<HTMLDivElement>;
}

/** Mascote que anda sobre a borda da dock, reagindo às sessões do Claude Code (ver
 * `useClaudeAgents`). Com alguma sessão, clicar nele faz o mesmo que o widget; como ele fica fora
 * da barra, informa a própria área ao Rust (`set_pet_rect`), senão o clique atravessa a janela. */
export function ClawdPet({ position, full, margin, visible, barRef }: Props) {
  const agents = useClaudeAgents();
  const vertical = isVertical(position);
  const mood = moodOf(agents);
  const lead = leadStatus(agents);
  const t = useT().claudeAgents;
  const clickable = visible && agents.length > 0;
  const clickableRef = useRef(clickable);
  clickableRef.current = clickable;
  const reportedRef = useRef<{ x: number; y: number } | null>(null);

  const outerRef = useRef<HTMLDivElement>(null);
  const posRef = useRef(0);
  const dirRef = useRef<1 | -1>(1);
  const [facing, setFacing] = useState<Facing>(vertical ? "down" : "right");
  const [barSize, setBarSize] = useState({ width: 0, height: 0 });
  // Fica parado sob o cursor, senão o clique erra o alvo andando.
  const [hovered, setHovered] = useState(false);
  // Também fica parado enquanto o painel das sessões (aberto por ele ou pelo widget) está aberto.
  const [panelOpen, setPanelOpen] = useState(false);
  const walking = mood === "walking" && !(hovered && clickable) && !panelOpen;

  // Ao sair da área dele o Rust liga o click-through e o WebView não recebe o mouseleave.
  useEffect(() => {
    const subs = [
      listen("dock-pointer-left", () => setHovered(false)),
      listen<string | null>("panel-changed", (e) => setPanelOpen(e.payload === "claude-agents")),
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
    const size = PET + HIT_STEP * 2;
    ipc.setPetRect({ x: x - HIT_STEP, y: y - HIT_STEP, width: size, height: size, radius: 0 }).catch(console.error);
  };

  const apply = () => {
    const el = outerRef.current;
    if (!el || !barSize.width || !barSize.height) return;
    const rect = barRect(position, full, margin, barSize, { width: window.innerWidth, height: window.innerHeight });
    const axis = vertical ? rect.height : rect.width;
    const bounds = Math.max(0, axis - EDGE_PAD * 2 - PET);
    if (posRef.current > bounds) posRef.current = bounds;
    el.style.setProperty("--pet-hide", `${(vertical ? barSize.width : barSize.height) + PET + 24}px`);
    let x: number, y: number;
    if (!vertical) {
      y = position === "bottom" ? rect.y - PET + OVERLAP : rect.y + rect.height - OVERLAP;
      x = rect.x + EDGE_PAD + posRef.current;
    } else {
      x = position === "right" ? rect.x - PET + OVERLAP : rect.x + rect.width - OVERLAP;
      y = rect.y + EDGE_PAD + posRef.current;
    }
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
            setFacing(vertical ? "up" : "left");
          } else if (next <= 0) {
            next = 0;
            dirRef.current = 1;
            setFacing(vertical ? "down" : "right");
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

  return (
    <div
      ref={outerRef}
      className={`claude-pet ${visible ? "" : "claude-pet-hidden"} ${clickable ? "clickable" : ""}`}
      role={clickable ? "button" : undefined}
      aria-label={clickable ? t.tile(agents.length, t.status[lead ?? "running"]) : undefined}
      aria-hidden={!clickable}
      onClick={(e) => clickable && openClaudeAgents(agents, e.currentTarget)}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className={`claude-pet-facing facing-${facing}`}>
        <span className={`claude-sprite claude-pet-sprite ${walking ? "walking" : `claude-pet-${mood === "walking" ? "idle" : mood}`} ${lead ? `claude-${lead}` : ""}`}>
          <ClaudeMascotIcon size={18} />
        </span>
      </div>
    </div>
  );
}
