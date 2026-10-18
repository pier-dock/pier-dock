/** Altura da faixa de prévias das janelas (px lógicos). */
export const PREVIEW_HEIGHT = 118;
/** Folga entre a barra e a faixa de prévias. */
export const PREVIEW_GAP = 12;

/** Medidas da dock (px lógicos), derivadas do tamanho do ícone, da magnificação e da altura escolhida. */
export function dockMetrics(
  iconSize: number,
  magnification: number,
  fullWidth = false,
  dockHeight: number | null = null,
  previewSpace = 0,
) {
  const padding = 8;
  const margin = 8;
  // 64 com o ícone padrão de 40: botão de 56 e 4 de folga em cima e embaixo.
  const autoHeight = iconSize + 24;
  // A altura automática acompanha o ícone. A escolhida à mão nunca fica menor que o ícone mais a
  // legenda do indicador de app aberto, senão os ícones saem da barra.
  const barHeight = dockHeight == null ? autoHeight : Math.min(Math.max(dockHeight, iconSize + 16), 240);
  // Na largura total a barra encosta na borda de baixo, então o espaço reservado é a própria
  // altura da barra: as janelas maximizadas encostam nela. Sem isso sobram as duas margens (16px)
  // entre a janela e a dock.
  const reservedHeight = fullWidth ? barHeight : barHeight + margin * 2;
  const tooltipSpace = 48;
  const base = Math.max(0, iconSize * (magnification - 1)) + tooltipSpace;
  // A faixa de prévias fica acima da barra: sem folga extra na janela ela seria cortada, e o Rust
  // recortaria a área clicável antes de o cursor alcançá-la.
  const hoverHeadroom = Math.max(base, previewSpace);
  return {
    padding,
    margin,
    barHeight,
    // Sobra da altura escolhida sobre a automática. Metade dela vai embaixo dos ícones, que são
    // presos à base da barra: assim eles continuam centrados nela e nivelados com os widgets.
    itemBottom: 3 + (barHeight - autoHeight) / 2,
    reservedHeight,
    hoverHeadroom,
    windowHeight: reservedHeight + hoverHeadroom,
  };
}

/** Posição (left) da faixa de prévias: centrada no ícone, mas sem sair da janela. */
export function previewLeft(centerX: number, windowWidth: number, stripWidth: number): number {
  const edge = 8;
  // Sem caber com as duas folgas a faixa não tem como ficar inteira na tela: encosta na esquerda.
  if (stripWidth + edge * 2 > windowWidth) return edge;
  return Math.min(Math.max(centerX - stripWidth / 2, edge), windowWidth - stripWidth - edge);
}
