import { useEffect, useRef, useState } from "react";
import type { DockPosition } from "../lib/ipc";
import { usePokemonSprite } from "../hooks/usePokemonSprite";
import { barRect, isVertical } from "./layout";
import { petPose, PET_SIZE as PET, PET_EDGE_PAD as EDGE_PAD } from "./petPose";

const SPEED = 38;

interface Props {
  id: number;
  /** Posição na lista: espalha os Pokémon pela borda e varia a velocidade para não andarem colados. */
  index: number;
  count: number;
  position: DockPosition;
  full: boolean;
  margin: number;
  visible: boolean;
  barRef: React.RefObject<HTMLDivElement>;
}

/** Pokémon que anda sobre a borda da dock com o ícone de time. Só decora: não recebe clique, então
 * não precisa informar a área ao Rust como o Clawd. */
export function PokemonPet({ id, index, count, position, full, margin, visible, barRef }: Props) {
  const src = usePokemonSprite(id);
  const vertical = isVertical(position);
  const outerRef = useRef<HTMLDivElement>(null);
  const posRef = useRef(0);
  const startDir = index % 2 === 0 ? 1 : -1;
  const speed = SPEED * (1 + 0.12 * (index % 4));
  const placedRef = useRef(false);
  const dirRef = useRef<1 | -1>(startDir);
  const [direction, setDirection] = useState<1 | -1>(startDir);
  const [barSize, setBarSize] = useState({ width: 0, height: 0 });
  const walking = visible && !!src;

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
    if (!placedRef.current) {
      placedRef.current = true;
      posRef.current = (bounds * index) / count;
    }
    if (posRef.current > bounds) posRef.current = bounds;
    el.style.setProperty("--pet-hide", `${(vertical ? barSize.width : barSize.height) + PET + 24}px`);
    const { x, y } = petPose(position, rect, posRef.current, dirRef.current);
    el.style.left = `${x}px`;
    el.style.top = `${y}px`;
  };

  useEffect(() => {
    apply();
    const onResize = () => apply();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [position, full, margin, barSize, vertical]);

  useEffect(() => {
    if (!walking) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const rect = barRect(position, full, margin, barSize, { width: window.innerWidth, height: window.innerHeight });
      const axis = vertical ? rect.height : rect.width;
      const bounds = Math.max(0, axis - EDGE_PAD * 2 - PET);
      const dt = (now - last) / 1000;
      last = now;
      if (bounds > 0) {
        let next = posRef.current + dirRef.current * speed * dt;
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
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [walking, position, full, margin, barSize, vertical]);

  const pose = petPose(position, { x: 0, y: 0, width: 0, height: 0 }, 0, direction);
  return (
    <div ref={outerRef} className={`claude-pet pokemon-pet ${visible ? "" : "claude-pet-hidden"}`} aria-hidden>
      <div className="claude-pet-orientation" style={{ transform: `rotate(${pose.angle}deg)` }}>
        {/* O ícone de time olha para a esquerda; o `mirror` da pose vale para o Clawd, que olha para a direita. */}
        <div className={`claude-pet-facing ${pose.mirror ? "facing-right" : "facing-left"}`}>
          {src && <img className={`pokemon-pet-sprite ${walking ? "walking" : ""}`} src={src} alt="" draggable={false} />}
        </div>
      </div>
    </div>
  );
}
