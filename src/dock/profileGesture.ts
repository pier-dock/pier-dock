/** Sem eventos por este tempo, o gesto acabou. */
export const QUIET_MS = 140;
/** Scroll acumulado, em px, que decide a troca. */
const COMMIT_AT = 48;
/** Scroll acumulado que equivale a uma troca inteira no acompanhamento visual. */
const TRAVEL = 110;
const LINE_PX = 16;
const PAGE_PX = 400;
/** A partir daqui o evento parece um clique de roda, não um trackpad. */
const NOTCH_PX = 50;
/** Cliques de roda seguidos só trocam de novo depois deste tempo, para a transição ser vista. */
const NOTCH_REPEAT_MS = 320;
/** Um gesto novo no meio da inércia só conta depois deste tempo. */
const RENEWED_AFTER_MS = 160;

export interface WheelSample {
  deltaX: number;
  deltaY: number;
  /** 0 = pixels, 1 = linhas, 2 = páginas, como em `WheelEvent.deltaMode`. */
  deltaMode: number;
  time: number;
}

export type GestureStep = { kind: "ignore" } | { kind: "track"; progress: number } | { kind: "commit"; direction: 1 | -1 };

/**
 * Transforma eventos de roda em no máximo uma troca de perfil por gesto. O trackpad continua
 * mandando eventos de inércia depois que o dedo sai, então a trava só abre com silêncio, com um
 * gesto novo (o delta volta a crescer) ou com cliques seguidos de roda (delta alto e constante).
 */
export function createProfileGesture() {
  let accumulated = 0;
  let lastTime = -Infinity;
  let lastSize = 0;
  let locked = false;
  let committedAt = -Infinity;

  return {
    wheel(sample: WheelSample): GestureStep {
      const unit = sample.deltaMode === 1 ? LINE_PX : sample.deltaMode === 2 ? PAGE_PX : 1;
      const dy = sample.deltaY * unit;
      if (dy === 0 || Math.abs(dy) < Math.abs(sample.deltaX * unit)) return { kind: "ignore" };

      const gap = sample.time - lastTime;
      const previous = lastSize;
      const size = Math.abs(dy);
      lastTime = sample.time;
      lastSize = size;

      if (locked) {
        const since = sample.time - committedAt;
        const quiet = gap > QUIET_MS;
        const renewed = size > previous * 1.8 && size > 6 && since > RENEWED_AFTER_MS;
        const notch = size >= NOTCH_PX && size >= previous * 0.9 && since > NOTCH_REPEAT_MS;
        if (!quiet && !renewed && !notch) return { kind: "ignore" };
        locked = false;
        accumulated = 0;
      } else if (gap > QUIET_MS) {
        accumulated = 0;
      }

      accumulated += dy;
      if (Math.abs(accumulated) >= COMMIT_AT) {
        const direction = accumulated > 0 ? 1 : -1;
        locked = true;
        committedAt = sample.time;
        accumulated = 0;
        return { kind: "commit", direction };
      }
      return { kind: "track", progress: accumulated / TRAVEL };
    },
    /** O dedo parou antes do limiar: o próximo evento começa do zero. */
    release() {
      accumulated = 0;
    },
  };
}
