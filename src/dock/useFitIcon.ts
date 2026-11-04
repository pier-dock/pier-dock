import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { MotionValue } from "framer-motion";
import { fitDock, type DockFit } from "./fit.ts";

/**
 * O comprimento da barra sem as áreas dos ícones. Usa medidas de layout, que o `transform` da
 * animação de ocultar não altera, e o tamanho exato dos slots: eles ainda podem estar animando.
 */
function measureBar(bar: HTMLElement, vertical: boolean) {
  const start = (el: HTMLElement) => (vertical ? el.offsetTop : el.offsetLeft);
  const length = (el: HTMLElement) => (vertical ? el.offsetHeight : el.offsetWidth);
  // O `DndContext` põe os textos de acessibilidade dentro da barra, fora do fluxo dela.
  const items = ([...bar.children] as HTMLElement[]).filter((el) => {
    const { display, position } = getComputedStyle(el);
    return display !== "none" && position !== "fixed" && position !== "absolute";
  });
  const style = getComputedStyle(bar);
  const padding = parseFloat(vertical ? style.paddingTop : style.paddingLeft) + parseFloat(vertical ? style.paddingBottom : style.paddingRight);
  const border = vertical ? bar.offsetHeight - bar.clientHeight : bar.offsetWidth - bar.clientWidth;
  const content = items.length ? start(items[items.length - 1]) + length(items[items.length - 1]) - start(items[0]) : 0;
  const slots = [...bar.querySelectorAll<HTMLElement>(".magnify")];
  // Na horizontal o botão tem `min-width` de ícone + 8, que vale na hora, enquanto o slot ainda
  // anima até o tamanho novo: o que conta como ícone é o maior dos dois. Sem isso, um ícone que
  // cresce parece alongar o resto da barra, encolhe na medição seguinte e os dois ficam alternando.
  const icon = parseFloat(style.getPropertyValue("--icon"));
  const icons = slots.reduce((sum, slot) => {
    const slotStyle = getComputedStyle(slot);
    return sum + (vertical ? parseFloat(slotStyle.height) : Math.max(parseFloat(slotStyle.width), icon));
  }, 0);
  return { rest: content + padding + border - icons, count: slots.length };
}

/**
 * Tamanho do ícone e magnificação com que a barra cabe nesta janela. Cada dock calcula o seu,
 * porque o espaço depende do monitor em que ela está.
 */
export function useFitIcon(
  barRef: RefObject<HTMLElement | null>,
  mouse: MotionValue<number>,
  { iconSize, magnification, vertical, margin }: { iconSize: number; magnification: number; vertical: boolean; margin: number },
): DockFit {
  const [fit, setFit] = useState<DockFit>({ icon: iconSize, scale: magnification });
  const latest = useRef(fit);

  const measure = useCallback(() => {
    const bar = barRef.current;
    if (!bar) return;
    const available = (vertical ? window.innerHeight : window.innerWidth) - margin * 2;
    const { rest, count } = measureBar(bar, vertical);
    const current = latest.current;
    const next = fitDock({ iconSize, magnification, available, rest, count, previous: current.icon });
    if (next.icon === current.icon && next.scale === current.scale) return;
    latest.current = next;
    setFit(next);
  }, [barRef, iconSize, magnification, vertical, margin]);

  // Fora do commit: dentro de um layout effect, qualquer oscilação da medida estoura o limite de
  // atualizações do React, e a dock some até o app reiniciar.
  useEffect(() => {
    const frame = requestAnimationFrame(measure);
    return () => cancelAnimationFrame(frame);
  });

  useEffect(() => {
    const bar = barRef.current;
    if (!bar) return;
    // A magnificação redimensiona a barra a cada quadro, sem mudar o que cabe nela. Ao sair, a barra
    // volta ao tamanho de repouso e o observer mede de novo.
    const observer = new ResizeObserver(() => {
      if (!Number.isFinite(mouse.get())) measure();
    });
    observer.observe(bar);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [barRef, mouse, measure]);

  return fit;
}
