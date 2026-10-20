import { useEffect, useRef, useState } from "react";
import type { ClaudeAgent, DockPosition } from "../lib/ipc";
import { useClaudeAgents } from "../hooks/useClaudeAgents";
import { leadStatus } from "../widgets/ClaudeAgentsWidget";
import { barRect, isVertical } from "./layout";
import { ClaudeMascotIcon } from "./ClaudeMascotIcon";

type Mood = "idle" | "walking" | "waiting";

/** Lado do sprite, para espelhar/inclinar na direção do passeio. */
type Facing = "left" | "right" | "up" | "down";

const PET = 26;
const EDGE_PAD = 10;
const OVERLAP = 6;
const SPEED = 46;

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

/** Mascote decorativo que anda sobre a borda da dock, reagindo às sessões do Claude Code (ver
 * `useClaudeAgents`). Puramente visual: `pointer-events: none`, não participa da área clicável
 * (`set_hit_rect`) nem precisa de nenhuma mudança no Rust. */
export function ClawdPet({ position, full, margin, visible, barRef }: Props) {
  const agents = useClaudeAgents();
  const vertical = isVertical(position);
  const mood = moodOf(agents);
  const lead = leadStatus(agents);

  const outerRef = useRef<HTMLDivElement>(null);
  const posRef = useRef(0);
  const dirRef = useRef<1 | -1>(1);
  const [facing, setFacing] = useState<Facing>(vertical ? "down" : "right");
  const [barSize, setBarSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    const measure = () => setBarSize({ width: bar.offsetWidth, height: bar.offsetHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(bar);
    return () => observer.disconnect();
  }, [barRef]);

  const apply = () => {
    const el = outerRef.current;
    if (!el || !barSize.width || !barSize.height) return;
    const rect = barRect(position, full, margin, barSize, { width: window.innerWidth, height: window.innerHeight });
    const axis = vertical ? rect.height : rect.width;
    const bounds = Math.max(0, axis - EDGE_PAD * 2 - PET);
    if (posRef.current > bounds) posRef.current = bounds;
    el.style.setProperty("--pet-hide", `${(vertical ? barSize.width : barSize.height) + PET + 24}px`);
    if (!vertical) {
      el.style.top = `${position === "bottom" ? rect.y - PET + OVERLAP : rect.y + rect.height - OVERLAP}px`;
      el.style.left = `${rect.x + EDGE_PAD + posRef.current}px`;
    } else {
      el.style.left = `${position === "right" ? rect.x - PET + OVERLAP : rect.x + rect.width - OVERLAP}px`;
      el.style.top = `${rect.y + EDGE_PAD + posRef.current}px`;
    }
  };

  useEffect(() => {
    apply();
    const onResize = () => apply();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [position, full, margin, barSize, vertical]);

  useEffect(() => {
    if (mood !== "walking") return;
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
  }, [mood, position, full, margin, barSize, vertical]);

  return (
    <div ref={outerRef} className={`claude-pet ${visible ? "" : "claude-pet-hidden"}`} aria-hidden>
      <div className={`claude-pet-facing facing-${facing}`}>
        <span className={`claude-sprite claude-pet-sprite ${mood === "walking" ? "walking" : `claude-pet-${mood}`} ${lead ? `claude-${lead}` : ""}`}>
          <ClaudeMascotIcon size={18} />
        </span>
      </div>
    </div>
  );
}
