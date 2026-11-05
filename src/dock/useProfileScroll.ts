import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import type { DockProfile } from "../lib/ipc";
import { createProfileGesture, QUIET_MS } from "./profileGesture.ts";
import { neighborProfile } from "./profiles.ts";

const SPRING = "cubic-bezier(0.22, 1.25, 0.36, 1)";
const SETTLE_MS = 420;
const HUD_MS = 1100;
/** Deslocamento do conteúdo, em px. Pequeno para não sair da barra: ela não recorta por causa da sombra. */
const TRAVEL_PX = 24;

interface Options {
  barRef: RefObject<HTMLElement | null>;
  enabled: boolean;
  invert: boolean;
  vertical: boolean;
  /** De ponta a ponta a barra não muda de tamanho. */
  animateLength: boolean;
  profiles: DockProfile[];
  currentId: string;
  onSwitch: (id: string) => void;
}

const reducedMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

const inFlow = (el: Element) => {
  const { display, position } = getComputedStyle(el);
  return display !== "none" && position !== "absolute" && position !== "fixed";
};

/** O tamanho que a barra teria sem uma largura imposta. */
function naturalLength(bar: HTMLElement, vertical: boolean) {
  const items = [...bar.children].filter(inFlow) as HTMLElement[];
  const style = getComputedStyle(bar);
  const padding = parseFloat(vertical ? style.paddingTop : style.paddingLeft) + parseFloat(vertical ? style.paddingBottom : style.paddingRight);
  const border = vertical ? bar.offsetHeight - bar.clientHeight : bar.offsetWidth - bar.clientWidth;
  if (!items.length) return padding + border;
  const start = (el: HTMLElement) => (vertical ? el.offsetTop : el.offsetLeft);
  const size = (el: HTMLElement) => (vertical ? el.offsetHeight : el.offsetWidth);
  const last = items[items.length - 1];
  return start(last) + size(last) - start(items[0]) + padding + border;
}

/**
 * Leva a barra do tamanho antigo até o do conteúdo novo. O alvo é medido a cada quadro porque os
 * ícones sob o cursor ainda crescem com a magnificação, e a curva não passa do ponto: a barra só
 * muda de direção se o conteúdo mudar. Devolve como cancelar.
 */
function resizeToContent(bar: HTMLElement, vertical: boolean, from: number) {
  const property = vertical ? "height" : "width";
  const startedAt = performance.now();
  let frame = 0;
  const finish = () => {
    cancelAnimationFrame(frame);
    bar.style.removeProperty(property);
    bar.classList.remove("dock-profile-resizing");
  };
  const step = (now: number) => {
    const t = Math.min(1, (now - startedAt) / SETTLE_MS);
    if (t >= 1) return finish();
    const eased = 1 - (1 - t) ** 3;
    const target = naturalLength(bar, vertical);
    bar.style.setProperty(property, `${from + (target - from) * eased}px`);
    frame = requestAnimationFrame(step);
  };
  bar.classList.add("dock-profile-resizing");
  bar.style.setProperty(property, `${from}px`);
  frame = requestAnimationFrame(step);
  return finish;
}

/** Cópia do conteúdo que sai, para ele sumir enquanto o do perfil novo entra. */
function fadeOutgoing(bar: HTMLElement, shift: [number, number], opacity: number, to: [number, number]) {
  const ghost = document.createElement("div");
  ghost.className = "dock-profile-ghost";
  ghost.inert = true;
  ghost.setAttribute("aria-hidden", "true");
  ghost.style.cssText = bar.style.cssText;
  Object.assign(ghost.style, {
    left: `${bar.offsetLeft}px`,
    top: `${bar.offsetTop}px`,
    right: "auto",
    bottom: "auto",
    width: `${bar.offsetWidth}px`,
    height: `${bar.offsetHeight}px`,
  });
  for (const child of bar.children) if (inFlow(child)) ghost.append(child.cloneNode(true));
  bar.parentElement!.append(ghost);
  const animation = ghost.animate(
    [
      { translate: `${shift[0]}px ${shift[1]}px`, opacity },
      { translate: `${to[0]}px ${to[1]}px`, opacity: 0 },
    ],
    { duration: 200, easing: "ease-out", fill: "forwards" },
  );
  animation.onfinish = animation.oncancel = () => ghost.remove();
}

/**
 * Scroll sobre a dock troca de perfil. O acompanhamento do gesto escreve direto no estilo: são
 * dezenas de eventos por segundo e nenhum muda o que a dock renderiza.
 */
export function useProfileScroll({ barRef, enabled, invert, vertical, animateLength, profiles, currentId, onSwitch }: Options) {
  const [peekId, setPeekId] = useState<string | null>(null);
  const [hudVisible, setHudVisible] = useState(false);
  const pending = useRef<{ direction: 1 | -1; length: number; progress: number } | null>(null);
  const stopResize = useRef<(() => void) | null>(null);
  const latest = useRef({ invert, vertical, animateLength, profiles, currentId, onSwitch });
  latest.current = { invert, vertical, animateLength, profiles, currentId, onSwitch };

  useEffect(() => {
    const bar = barRef.current;
    const root = bar?.parentElement;
    if (!bar || !root || !enabled) return;
    const gesture = createProfileGesture();
    let progress = 0;
    let releaseTimer: ReturnType<typeof setTimeout>;
    let settleTimer: ReturnType<typeof setTimeout>;
    let hudTimer: ReturnType<typeof setTimeout>;

    const hideHudLater = () => {
      clearTimeout(hudTimer);
      hudTimer = setTimeout(() => setHudVisible(false), HUD_MS);
    };
    const paint = (p: number) => {
      progress = p;
      const axis = latest.current.vertical ? "x" : "y";
      const other = axis === "x" ? "y" : "x";
      root.style.setProperty(`--profile-${axis}`, `${-p * TRAVEL_PX}px`);
      root.style.setProperty(`--profile-${other}`, "0px");
      root.style.setProperty("--profile-opacity", String(1 - Math.abs(p) * 1.25));
      root.style.setProperty(`--peek-${axis}`, `${(Math.sign(p) - p) * TRAVEL_PX}px`);
      root.style.setProperty(`--peek-${other}`, "0px");
      root.style.setProperty("--peek-opacity", String(Math.abs(p)));
    };
    const clearGesture = () => {
      delete bar.dataset.profileGesture;
      delete bar.dataset.profileSettling;
      delete root.dataset.profileSettling;
      setPeekId(null);
    };

    const onWheel = (e: WheelEvent) => {
      const { invert, profiles, currentId, onSwitch } = latest.current;
      if (profiles.length < 2) return;
      // O volume tem o scroll dele, e arrastar um item não troca a dock de baixo dele.
      if ((e.target as Element).closest("[data-wheel]") || bar.querySelector(".dragging")) return;
      const step = gesture.wheel({ deltaX: e.deltaX, deltaY: invert ? -e.deltaY : e.deltaY, deltaMode: e.deltaMode, time: e.timeStamp });
      if (step.kind === "ignore") return;
      e.preventDefault();
      clearTimeout(releaseTimer);
      clearTimeout(settleTimer);
      delete bar.dataset.profileSettling;
      delete root.dataset.profileSettling;
      setHudVisible(true);
      clearTimeout(hudTimer);

      if (step.kind === "track") {
        setPeekId(neighborProfile(profiles, currentId, step.progress > 0 ? 1 : -1)?.id ?? null);
        bar.dataset.profileGesture = "";
        paint(step.progress);
        releaseTimer = setTimeout(() => {
          gesture.release();
          bar.dataset.profileSettling = "";
          root.dataset.profileSettling = "";
          const leaving = Math.sign(progress) * TRAVEL_PX;
          paint(0);
          root.style.setProperty(`--peek-${latest.current.vertical ? "x" : "y"}`, `${leaving}px`);
          settleTimer = setTimeout(clearGesture, 320);
          hideHudLater();
        }, QUIET_MS);
        return;
      }

      const next = neighborProfile(profiles, currentId, step.direction);
      if (!next) return;
      const { vertical } = latest.current;
      const offset = (amount: number): [number, number] => (vertical ? [amount, 0] : [0, amount]);
      if (!reducedMotion()) {
        fadeOutgoing(bar, offset(-progress * TRAVEL_PX), 1 - Math.abs(progress) * 1.25, offset(-step.direction * TRAVEL_PX));
      }
      pending.current = { direction: step.direction, length: vertical ? bar.offsetHeight : bar.offsetWidth, progress };
      progress = 0;
      clearGesture();
      onSwitch(next.id);
      hideHudLater();
    };

    bar.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      bar.removeEventListener("wheel", onWheel);
      clearTimeout(releaseTimer);
      clearTimeout(settleTimer);
      clearTimeout(hudTimer);
      clearGesture();
    };
  }, [barRef, enabled]);

  // O conteúdo novo já está na barra: entra de onde a prévia estava, e a barra anima até o tamanho dele.
  useLayoutEffect(() => {
    const bar = barRef.current;
    const change = pending.current;
    pending.current = null;
    if (!bar || !change) return;
    const { vertical } = latest.current;
    const from = (change.direction - change.progress) * TRAVEL_PX;
    const start = vertical ? `${from}px 0` : `0 ${from}px`;
    const motion = !reducedMotion();
    for (const child of bar.children) {
      if (child.classList.contains("dock-profile-hud") || !inFlow(child)) continue;
      child.animate(
        motion
          ? [{ translate: start, opacity: Math.abs(change.progress) }, { translate: "0 0", opacity: 1 }]
          : [{ opacity: 0 }, { opacity: 1 }],
        { duration: motion ? SETTLE_MS : 180, easing: motion ? SPRING : "ease" },
      );
    }
    stopResize.current?.();
    stopResize.current = motion && latest.current.animateLength ? resizeToContent(bar, vertical, change.length) : null;
  }, [barRef, currentId]);

  useEffect(() => () => stopResize.current?.(), []);

  return { peekId, hudVisible };
}
