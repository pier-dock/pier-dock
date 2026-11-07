let active = true;
const resumers = new Set<() => void>();

/**
 * Só a janela do dock chama isto: com a barra escondida (auto-hide) nada na tela muda, então os
 * polls dela pausam. Nas outras janelas (popovers, ajustes) o estado fica sempre ativo.
 */
export function setDockActive(next: boolean): void {
  if (next === active) return;
  active = next;
  if (next) [...resumers].forEach((resume) => resume());
}

/**
 * `setInterval` que pula os ticks enquanto o dock está escondido. Ao reaparecer, roda `fn` uma vez
 * só se algum tick foi pulado: quem aparece e some toda hora não dispara mais buscas do que o
 * intervalo já permitia.
 */
export function setActiveInterval(fn: () => void, ms: number): () => void {
  let missed = false;
  const resume = () => {
    if (!missed) return;
    missed = false;
    fn();
  };
  const id = setInterval(() => {
    if (active) fn();
    else missed = true;
  }, ms);
  resumers.add(resume);
  return () => {
    clearInterval(id);
    resumers.delete(resume);
  };
}
