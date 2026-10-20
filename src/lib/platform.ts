import { dict } from "../i18n";

/** Sistema em que a dock está rodando, detectado pelo WebView (WebView2, WebKitGTK ou WKWebView). */
export type Os = "windows" | "linux" | "macos";

const ua = navigator.userAgent;
export const os: Os = /Windows/i.test(ua) ? "windows" : /Macintosh|Mac OS X/i.test(ua) ? "macos" : "linux";

/** A bandeja (ícones ocultos) só existe no Windows. */
export const widgetSupported = (kind: string) => kind !== "tray" || os === "windows";

/** Teclas dos atalhos como aparecem nos chips (`Kbd`), com os símbolos do Mac no macOS. */
export const keys = {
  /** Modificadores do atalho global que abre o app N da dock (Ctrl+Alt+N, ⌃⌥N no Mac). */
  appShortcut: os === "macos" ? "⌃ ⌥" : "Ctrl Alt",
  command: os === "macos" ? "⌘" : "Ctrl",
};

/** Textos que mudam conforme o sistema, no idioma atual. */
export function labels() {
  const t = dict().platform;
  return {
    startButton: t.startButton[os],
    autostart: t.autostart[os],
    nativeBar: t.nativeBar[os],
    quit: t.quit[os],
    dropHint: t.dropHint[os],
    autoHideNever: t.autoHideNever[os],
    keychain: t.keychain[os],
    openSettings: t.openSettings[os],
  };
}
