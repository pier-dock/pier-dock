import { createContext, useContext, useRef, type ReactNode } from "react";
import { motion, useSpring, useTransform, type MotionValue } from "framer-motion";
import type { DockPosition } from "../lib/ipc";
import { isVertical } from "./layout";

interface MagnifyContext {
  /** Posição do cursor no eixo da barra: x nas bordas de cima e de baixo, y nas laterais. */
  mouse: MotionValue<number>;
  base: number;
  scale: number;
  position: DockPosition;
}

export const MagnifyCtx = createContext<MagnifyContext | null>(null);

/** Item que cresce conforme o cursor se aproxima, como na dock do macOS. */
export function Magnify({ children }: { children: ReactNode }) {
  const ctx = useContext(MagnifyCtx)!;
  const ref = useRef<HTMLDivElement>(null);
  const range = ctx.base * 2.6;
  const distance = useTransform(ctx.mouse, (value) => {
    const r = ref.current?.getBoundingClientRect();
    if (!r) return Infinity;
    return isVertical(ctx.position) ? value - r.top - r.height / 2 : value - r.left - r.width / 2;
  });
  const size = useTransform(distance, [-range, 0, range], [ctx.base, ctx.base * ctx.scale, ctx.base], { clamp: true });
  const spring = useSpring(size, { mass: 0.1, stiffness: 170, damping: 14 });
  return (
    <motion.div ref={ref} className="magnify" style={{ width: spring, height: spring }}>
      {children}
    </motion.div>
  );
}
