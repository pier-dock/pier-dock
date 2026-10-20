//! macOS. Os apps abertos vêm do NSWorkspace; as janelas deles, da API de Acessibilidade, que
//! exige a permissão em Ajustes do Sistema → Privacidade e Segurança → Acessibilidade.

pub mod ax;
pub mod events;
pub mod pointer;
pub mod workspace;

use super::PlatformInfo;

pub fn info() -> PlatformInfo {
    let window_list = if ax::is_trusted() { "ok" } else { "needs-accessibility" };
    PlatformInfo { os: "macos", window_list, hide_native: true, shortcuts: true, glass: true, follow_cursor: true, reserve_any_monitor: true }
}

/// Roda um AppleScript e devolve o resultado (sem a quebra de linha final).
pub fn osascript(script: &str) -> Option<String> {
    crate::util::run("/usr/bin/osascript", &["-e", script]).map(|out| out.trim_end_matches('\n').to_string())
}
