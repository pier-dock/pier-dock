/** Margem transparente da janela em volta do popover, para a sombra (igual a `--panel-inset`). */
export const PANEL_INSET = 18;

/** Tamanho visível (px lógicos) do popover de cada widget, sem a margem da sombra. */
export const PANEL_SIZES: Record<string, [number, number]> = {
  media: [380, 258],
  "github-review": [440, 400],
  "github-mine": [440, 400],
  "github-commits": [440, 400],
  "github-contrib": [500, 228],
  weather: [360, 426],
  pomodoro: [360, 406],
  "system-usage": [360, 460],
  calendar: [360, 372],
  "ai-usage": [340, 296],
  "claude-agents": [320, 280],
  bluetooth: [340, 340],
  launcher: [720, 500],
  volume: [340, 420],
  network: [340, 400],
};
