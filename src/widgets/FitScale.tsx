import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

/** Reduz o conteúdo para caber no espaço disponível, sem passar de `max`. */
export function FitScale({ max = 1, className = "", children }: { max?: number; className?: string; children: ReactNode }) {
  const outer = useRef<HTMLDivElement>(null);
  const inner = useRef<HTMLDivElement>(null);
  const [fit, setFit] = useState({ scale: 1, width: 0, height: 0 });
  useLayoutEffect(() => {
    const o = outer.current!;
    const i = inner.current!;
    const measure = () => {
      // offsetWidth/Height ignoram o transform: é o tamanho natural do widget.
      const width = i.offsetWidth;
      const height = i.offsetHeight;
      if (!width || !height) return;
      const style = getComputedStyle(o);
      const room = (size: number, a: string, b: string) => size - parseFloat(a) - parseFloat(b);
      const roomX = room(o.clientWidth, style.paddingLeft, style.paddingRight);
      const roomY = room(o.clientHeight, style.paddingTop, style.paddingBottom);
      setFit({ scale: Math.min(max, roomX / width, roomY / height), width, height });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(o);
    observer.observe(i);
    measure();
    return () => observer.disconnect();
  }, [max]);
  return (
    <div ref={outer} className={`fit-scale ${className}`.trim()}>
      <div style={{ width: fit.width * fit.scale, height: fit.height * fit.scale }}>
        <div ref={inner} className="fit-scale-content" style={{ transform: `scale(${fit.scale})` }}>
          {children}
        </div>
      </div>
    </div>
  );
}
