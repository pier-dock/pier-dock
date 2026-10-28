/** Margem transparente da janela em volta do popover, para a sombra (igual a `--panel-inset`). */
export const PANEL_INSET = 18;

/** Tamanho visível (px lógicos) do popover de cada widget, sem a margem da sombra. */
export const PANEL_SIZES: Record<string, [number, number]> = {
  clock: [340, 360],
  media: [380, 258],
  "github-review": [440, 400],
  "github-mine": [440, 400],
  "github-commits": [440, 400],
  "github-contrib": [500, 228],
  weather: [360, 426],
  pomodoro: [360, 406],
  "system-usage": [360, 460],
  calendar: [360, 420],
  "ai-usage": [340, 296],
  "ai-agents": [320, 280],
  bluetooth: [340, 340],
  launcher: [720, 500],
  volume: [340, 500],
  network: [340, 400],
  battery: [340, 280],
  "keyboard-layout": [320, 300],
};
