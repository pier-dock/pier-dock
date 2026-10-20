import { useCallback, useEffect, useRef } from "react";

type Reset = () => void;

const listeners = new Set<Reset>();

/**
 * Avisa os itens da dock que o cursor saiu dela. Com a dock escondida ou com o click-through
 * ligado, o WebView não recebe mouseleave, e o hover (com o tooltip) ficaria preso até a dock
 * aparecer de novo.
 */
export function resetDockHover(except?: Reset) {
  listeners.forEach((reset) => {
    if (reset !== except) reset();
  });
}

/**
 * Roda `reset` quando a dock avisa que o cursor saiu dela (ver `resetDockHover`). Devolve `claim`,
 * para chamar quando o item ganha o hover: só um item fica em hover por vez, então os outros
 * soltam o deles. Depois do click-through, o mouseleave do item anterior às vezes não chega, e
 * sem isso dois tooltips ficavam abertos.
 */
export function useDockHoverReset(reset: Reset) {
  const latest = useRef(reset);
  useEffect(() => {
    latest.current = reset;
  });
  const self = useRef<Reset>(() => latest.current());
  useEffect(() => {
    const run = self.current;
    listeners.add(run);
    return () => {
      listeners.delete(run);
    };
  }, []);
  return useCallback(() => resetDockHover(self.current), []);
}
