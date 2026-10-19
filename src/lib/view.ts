/** Qual tela esta janela renderiza. Injetado pelo Rust (panels.rs); a janela da dock não recebe nada. */
export interface ViewInfo {
  view: "dock" | "panel" | "settings" | "whats-new";
  kind: string;
}

declare global {
  interface Window {
    __PIER_VIEW__?: ViewInfo;
  }
}

export const currentView: ViewInfo = window.__PIER_VIEW__ ?? { view: "dock", kind: "" };
