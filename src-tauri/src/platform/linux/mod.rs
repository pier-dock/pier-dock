//! Linux. No GNOME (Wayland ou X11) a extensão `pier@pier.dev` dá acesso às janelas, reserva o
//! espaço da tela e esconde o Dash. No KDE Plasma 6 (X11) o EWMH e os struts bastam: o Pier lista
//! as janelas, reserva a área de trabalho e desenha o vidro com o blur do KWin, sem extensão
//! nenhuma. Numa sessão Plasma Wayland a dock roda no XWayland, então ela vê os apps X11, mas não
//! as janelas dos apps nativos do Wayland. Nas outras DEs, só sessões X11 funcionam (EWMH). A
//! janela da dock roda sempre via X11/XWayland, porque um cliente Wayland comum não pode se
//! posicionar na tela.

pub mod dbus;
pub mod desktop_entry;
pub mod desktop_integration;
pub mod gnome_shell;
pub mod x11;

use super::PlatformInfo;

fn env_is(name: &str, value: &str) -> bool {
    std::env::var(name).map(|v| v.eq_ignore_ascii_case(value)).unwrap_or(false)
}

pub fn is_wayland() -> bool {
    env_is("XDG_SESSION_TYPE", "wayland")
}

/// `XDG_CURRENT_DESKTOP` pode listar mais de um ambiente, separados por `:`.
fn desktop_is(desktop: &str, name: &str) -> bool {
    desktop.split(':').any(|part| part.eq_ignore_ascii_case(name))
}

fn current_desktop_is(name: &str) -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").map(|d| desktop_is(&d, name)).unwrap_or(false)
}

pub fn is_gnome() -> bool {
    current_desktop_is("gnome")
}

/// Sessão do KDE Plasma. No Plasma 6 (X11) a dock é completa: janelas, espaço reservado e o blur
/// do KWin. No Wayland, ela roda no XWayland e vê só os apps X11.
pub fn is_kde() -> bool {
    current_desktop_is("kde")
}

/// Sessão X11 de verdade: só nela o EWMH enxerga as janelas de todos os apps.
pub fn is_x11_session() -> bool {
    !is_wayland() && std::env::var_os("DISPLAY").is_some()
}

/// Tem que rodar antes do GTK iniciar: no Wayland a dock e os painéis vão para o XWayland, onde
/// podem escolher a própria posição e ficar acima das outras janelas.
pub fn prepare_env() {
    if is_wayland() && std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
}

pub fn info() -> PlatformInfo {
    let extension = gnome_shell::available();
    let window_list = if extension || is_x11_session() {
        "ok"
    } else if is_gnome() && gnome_shell::extension_installed() {
        "needs-relogin"
    } else if is_gnome() {
        "needs-extension"
    } else if is_kde() && std::env::var_os("DISPLAY").is_some() {
        // Plasma Wayland: o EWMH do XWayland enxerga os apps X11; as janelas dos apps nativos do
        // Wayland só apareceriam com uma ponte do lado do KWin (um script, como a extensão do GNOME).
        "x11-only"
    } else {
        "unsupported"
    };
    PlatformInfo {
        os: "linux",
        window_list,
        hide_native: extension,
        shortcuts: extension || is_x11_session(),
        glass: crate::backdrop::supported(),
        // No XWayland o cursor global só é atualizado sobre janelas X11; a extensão 4 o lê pelo Shell.
        follow_cursor: is_x11_session() || gnome_shell::pointer().is_some(),
        reserve_any_monitor: gnome_shell::reserves_any_monitor(),
    }
}

#[cfg(test)]
mod tests {
    use super::desktop_is;

    #[test]
    fn reconhece_o_ambiente_na_lista() {
        assert!(desktop_is("KDE", "kde"));
        assert!(desktop_is("GNOME:Unity", "gnome"));
        assert!(desktop_is("ubuntu:GNOME", "gnome"));
        assert!(desktop_is("X-Cinnamon:GNOME", "gnome"));
        assert!(!desktop_is("X-Cinnamon", "gnome"));
        assert!(!desktop_is("KDE", "gnome"));
        assert!(!desktop_is("", "kde"));
    }
}
