import type { DockPosition } from "../lib/ipc";

/** Altura da faixa de prévias das janelas (px lógicos), nas bordas de cima e de baixo. */
export const PREVIEW_HEIGHT = 118;
/** Largura da faixa de prévias (px lógicos), nas bordas laterais. */
export const PREVIEW_WIDTH = 208;
/** Folga entre a barra e a faixa de prévias. */
export const PREVIEW_GAP = 12;

/** A barra fica na vertical (bordas laterais) em vez de horizontal. */
export function isVertical(position: DockPosition): boolean {
  return position === "left" || position === "right";
}

/** Espaço que a faixa de prévias precisa na janela, saindo da barra (0 com a faixa fechada). */
export function previewSpace(position: DockPosition, open: boolean): number {
  if (!open) return 0;
  return (isVertical(position) ? PREVIEW_WIDTH : PREVIEW_HEIGHT) + PREVIEW_GAP;
}

/**
 * Medidas da dock (px lógicos), derivadas do tamanho do ícone, da magnificação, da espessura
 * escolhida e da borda da tela. As medidas de espessura valem na altura nas bordas de cima e de
 * baixo e na largura nas laterais.
 */
export function dockMetrics(
  iconSize: number,
  magnification: number,
  fullWidth = false,
  dockHeight: number | null = null,
  previewSpace = 0,
  position: DockPosition = "bottom",
) {
  const vertical = isVertical(position);
  const padding = 8;
  const margin = 8;
  // 64 com o ícone padrão de 40: botão de 56 e 4 de folga de cada lado.
  const autoThickness = iconSize + 24;
  // A espessura automática acompanha o ícone. A escolhida à mão nunca fica menor que o ícone mais
  // a legenda do indicador de app aberto, senão os ícones saem da barra.
  const barThickness = dockHeight == null ? autoThickness : Math.min(Math.max(dockHeight, iconSize + 16), 240);
  // De ponta a ponta a barra encosta na borda da tela, então o espaço reservado é a própria
  // espessura dela: as janelas maximizadas encostam na dock. Sem isso sobram as duas margens (16px)
  // entre a janela e a dock.
  const reserved = fullWidth ? barThickness : barThickness + margin * 2;
  // As dicas e a faixa de prévias saem da barra para dentro da tela. Nas laterais elas precisam de
  // mais espaço, e o mesmo espaço nos dois casos evita a janela mudar de tamanho quando a faixa
  // abre e fecha.
  const tooltipSpace = vertical ? PREVIEW_WIDTH + PREVIEW_GAP : 48;
  const base = Math.max(0, iconSize * (magnification - 1)) + tooltipSpace;
  const hoverHeadroom = Math.max(base, previewSpace);
  return {
    padding,
    margin,
    barThickness,
    // Sobra da espessura escolhida sobre a automática. Metade dela vai para o lado de dentro da
    // barra, porque os ícones ficam presos à borda da tela: assim eles continuam centrados nela e
    // nivelados com os widgets.
    itemOffset: 3 + (barThickness - autoThickness) / 2,
    reserved,
    hoverHeadroom,
    windowSize: reserved + hoverHeadroom,
  };
}

/**
 * Retângulo da barra dentro da janela da dock (px lógicos), para a área clicável. De ponta a ponta
 * a barra encosta na borda da tela; presa ao conteúdo ela fica centrada no eixo do comprimento e a
 * uma margem da borda no eixo da espessura.
 */
export function barRect(
  position: DockPosition,
  full: boolean,
  margin: number,
  bar: { width: number; height: number },
  windowSize: { width: number; height: number },
) {
  const vertical = isVertical(position);
  const inset = full ? 0 : margin;
  const x = vertical
    ? position === "left"
      ? inset
      : windowSize.width - bar.width - inset
    : full
      ? 0
      : (windowSize.width - bar.width) / 2;
  const y = vertical
    ? full
      ? 0
      : (windowSize.height - bar.height) / 2
    : position === "top"
      ? inset
      : windowSize.height - bar.height - inset;
  return { x, y, width: bar.width, height: bar.height };
}

/** Posição da faixa de prévias (left na horizontal, top na vertical): centrada, sem sair da tela. */
export function previewAlong(center: number, viewport: number, size: number): number {
  const edge = 8;
  // Sem caber com as duas folgas a faixa não tem como ficar inteira na tela: encosta no começo.
  if (size + edge * 2 > viewport) return edge;
  return Math.min(Math.max(center - size / 2, edge), viewport - size - edge);
}
