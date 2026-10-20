//! Cliente da extensão `pier@pier.dev` (pasta `gnome-extension/`), que roda dentro do GNOME
//! Shell e expõe janelas, overview, menus, área reservada e atalhos em `dev.pier.Shell`.
//!
//! O estado pedido (esconder o Dash, altura reservada, atalhos) fica guardado aqui e é reaplicado
//! sempre que a extensão aparece — ela desfaz tudo quando o Pier some do bus, inclusive num
//! crash, e também é recarregada quando o Shell reinicia.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use zbus::proxy::CacheProperties;

use super::dbus;

const UUID: &str = "pier@pier.dev";
const METADATA_JSON: &str = include_str!("../../../../gnome-extension/pier@pier.dev/metadata.json");
const EXTENSION_JS: &str = include_str!("../../../../gnome-extension/pier@pier.dev/extension.js");

/// Versão da extensão em que o `ToggleDesktop` (mostrar a área de trabalho) apareceu.
pub const SHOW_DESKTOP_VERSION: u32 = 2;

/// Versão da extensão em que o `SetReserved` reserva a área em qualquer borda; antes dele só na
/// base (`SetReservedHeight`), que é o que a versão 2 ainda tem.
const RESERVED_ANY_EDGE_VERSION: u32 = 3;

/// Versão da extensão em que a área é reservada no monitor da dock (`SetReservedAt`), não só no
/// principal, e em que o Pier lê o cursor pelo Shell (`Pointer`).
const ANY_MONITOR_VERSION: u32 = 4;

#[zbus::proxy(
    interface = "dev.pier.Shell",
    default_service = "dev.pier.Shell",
    default_path = "/dev/pier/Shell",
    gen_async = false,
    blocking_name = "ShellProxy"
)]
pub trait Shell {
    fn list_windows(&self) -> zbus::Result<String>;
    fn activate(&self, id: u64) -> zbus::Result<()>;
    fn minimize(&self, id: u64) -> zbus::Result<()>;
    fn close(&self, id: u64) -> zbus::Result<()>;
    fn show_apps(&self) -> zbus::Result<()>;
    fn open_quick_settings(&self) -> zbus::Result<()>;
    fn open_calendar(&self) -> zbus::Result<()>;
    fn toggle_desktop(&self) -> zbus::Result<()>;
    fn set_hide_native(&self, hide: bool) -> zbus::Result<()>;
    fn set_reserved_height(&self, height: u32) -> zbus::Result<()>;
    /// Extensões novas (versão 2 em diante): reserva a faixa na borda escolhida.
    fn set_reserved(&self, edge: &str, height: u32) -> zbus::Result<()>;
    /// Extensões da versão 4 em diante: reserva no monitor que contém (`x`, `y`).
    fn set_reserved_at(&self, edge: &str, height: u32, x: i32, y: i32) -> zbus::Result<()>;
    /// Extensões da versão 4 em diante: o cursor em coordenadas do Shell.
    fn pointer(&self) -> zbus::Result<(i32, i32)>;
    fn set_shortcuts_enabled(&self, enabled: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn windows_changed(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn shortcut_activated(&self, index: u32) -> zbus::Result<()>;

    #[zbus(property)]
    fn version(&self) -> zbus::Result<u32>;
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShellWindow {
    pub id: u64,
    pub title: String,
    /// Desktop id (`firefox.desktop`), quando o Shell sabe a que app a janela pertence.
    pub app_id: Option<String>,
    pub wm_class: Option<String>,
    pub pid: i32,
    pub minimized: bool,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ShellFocus {
    pub id: u64,
    pub pid: i32,
    pub fullscreen: bool,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct ShellSnapshot {
    /// Ordem de uso mais recente primeiro.
    pub windows: Vec<ShellWindow>,
    pub focus: Option<ShellFocus>,
    /// A visão geral (Atividades) está aberta.
    pub overview: bool,
}

#[derive(Clone, Copy)]
struct Desired {
    hide_native: bool,
    /// Borda onde a área é reservada: "bottom", "top", "left" ou "right".
    reserved_edge: &'static str,
    reserved_height: u32,
    /// Um ponto dentro do monitor da dock.
    reserved_at: Option<(i32, i32)>,
    shortcuts: bool,
}

static DESIRED: Mutex<Desired> =
    Mutex::new(Desired { hide_native: false, reserved_edge: "bottom", reserved_height: 0, reserved_at: None, shortcuts: false });

/// Proxy da extensão, se ela estiver carregada e respondendo.
pub fn shell() -> Option<ShellProxy<'static>> {
    let proxy = ShellProxy::builder(dbus::session()?).cache_properties(CacheProperties::No).build().ok()?;
    proxy.version().ok()?;
    Some(proxy)
}

pub fn available() -> bool {
    shell().is_some()
}

pub fn list_windows() -> Option<ShellSnapshot> {
    serde_json::from_str(&shell()?.list_windows().ok()?).ok()
}

fn apply(desired: Desired) {
    if let Some(shell) = shell() {
        let _ = shell.set_hide_native(desired.hide_native);
        // Numa extensão antiga a reserva só existe no monitor principal, e antes da versão 3 só na
        // base: pedir outra borda não adianta.
        let version = shell.version().unwrap_or(1);
        if let (true, Some((x, y))) = (version >= ANY_MONITOR_VERSION, desired.reserved_at) {
            let _ = shell.set_reserved_at(desired.reserved_edge, desired.reserved_height, x, y);
        } else if version >= RESERVED_ANY_EDGE_VERSION {
            let _ = shell.set_reserved(desired.reserved_edge, desired.reserved_height);
        } else {
            let _ = shell.set_reserved_height(desired.reserved_height);
        }
        let _ = shell.set_shortcuts_enabled(desired.shortcuts);
    }
}

fn update(change: impl FnOnce(&mut Desired)) {
    let desired = {
        let mut guard = DESIRED.lock().unwrap();
        change(&mut guard);
        *guard
    };
    apply(desired);
}

pub fn set_hide_native(hide: bool) {
    update(|d| d.hide_native = hide);
}

/// Espessura (pixels lógicos) que janelas maximizadas não devem ocupar na borda onde a dock está,
/// no monitor que contém `at`.
pub fn set_reserved(edge: &str, height: u32, at: (i32, i32)) {
    let edge = match edge {
        "top" => "top",
        "left" => "left",
        "right" => "right",
        _ => "bottom",
    };
    update(move |d| {
        d.reserved_edge = edge;
        d.reserved_height = height;
        d.reserved_at = Some(at);
    });
}

/// A extensão reserva a área em qualquer monitor? Sem ela, os struts do X11 já reservam.
pub fn reserves_any_monitor() -> bool {
    shell().is_none_or(|s| s.version().unwrap_or(0) >= ANY_MONITOR_VERSION)
}

/// O cursor pelo Shell, que o enxerga em qualquer janela (no XWayland o cursor global só se move
/// sobre janelas X11).
pub fn pointer() -> Option<(i32, i32)> {
    let shell = shell()?;
    if shell.version().ok()? < ANY_MONITOR_VERSION {
        return None;
    }
    shell.pointer().ok()
}

pub fn set_shortcuts_enabled(enabled: bool) {
    update(|d| d.shortcuts = enabled);
}

/// Acompanha a extensão: a cada (re)aparição reaplica o estado e chama `on_available`; enquanto
/// ela está lá, repassa os sinais de janelas e de atalhos.
pub fn watch(on_available: impl Fn() + Send + 'static, on_windows_changed: fn(), on_shortcut: impl Fn(u32) + Send + 'static) {
    std::thread::spawn(move || {
        let mut was_available = false;
        loop {
            let available = available();
            if available && !was_available {
                apply(*DESIRED.lock().unwrap());
                on_available();
            }
            was_available = available;
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    // Os streams de sinal acompanham a troca de dono do nome, então sobrevivem a um reinício do
    // Shell; só precisam esperar a extensão existir para serem criados.
    std::thread::spawn(move || loop {
        if let Some(Ok(signals)) = shell().map(|s| s.receive_windows_changed()) {
            for _ in signals {
                on_windows_changed();
            }
        }
        std::thread::sleep(Duration::from_secs(3));
    });
    std::thread::spawn(move || loop {
        if let Some(Ok(signals)) = shell().map(|s| s.receive_shortcut_activated()) {
            for signal in signals {
                if let Ok(args) = signal.args() {
                    on_shortcut(args.index);
                }
            }
        }
        std::thread::sleep(Duration::from_secs(3));
    });
}

fn extension_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("gnome-shell/extensions").join(UUID))
}

pub fn extension_installed() -> bool {
    extension_dir().is_some_and(|d| d.join("extension.js").exists())
        || PathBuf::from("/usr/share/gnome-shell/extensions").join(UUID).exists()
}

/// Copia a extensão embutida para `~/.local/share/gnome-shell/extensions` e a habilita. No
/// Wayland o Shell só carrega extensões novas no próximo login.
pub fn install_extension() -> Result<(), String> {
    remove_legacy_extension();
    let dir = extension_dir().ok_or("pasta de dados do usuário não encontrada")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("metadata.json"), METADATA_JSON).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("extension.js"), EXTENSION_JS).map_err(|e| e.to_string())?;
    if crate::util::run("gnome-extensions", &["enable", UUID]).is_none() {
        // O Shell ainda não conhece a extensão (Wayland): marca como habilitada para o próximo login.
        enable_in_settings()?;
    }
    Ok(())
}

/// A extensão do WinDock (`windock@windock.dev`) espera um dono de bus que não existe mais: sai
/// do Shell e do disco quando a do Pier é instalada.
fn remove_legacy_extension() {
    const LEGACY_UUID: &str = "windock@windock.dev";
    let _ = crate::util::run("gnome-extensions", &["disable", LEGACY_UUID]);
    if let Some(dir) = dirs::data_dir().map(|d| d.join("gnome-shell/extensions").join(LEGACY_UUID)) {
        let _ = std::fs::remove_dir_all(dir);
    }
}

fn enable_in_settings() -> Result<(), String> {
    let current = crate::util::run("gsettings", &["get", "org.gnome.shell", "enabled-extensions"])
        .ok_or("não foi possível ler as extensões habilitadas")?;
    let mut list = parse_gvariant_strv(&current);
    if !list.iter().any(|u| u == UUID) {
        list.push(UUID.to_string());
    }
    let value = format!("[{}]", list.iter().map(|u| format!("'{u}'")).collect::<Vec<_>>().join(", "));
    crate::util::run("gsettings", &["set", "org.gnome.shell", "enabled-extensions", &value])
        .map(|_| ())
        .ok_or_else(|| "não foi possível habilitar a extensão".into())
}

/// `@as []` ou `['a@b', 'c@d']` → lista.
fn parse_gvariant_strv(text: &str) -> Vec<String> {
    text.trim()
        .trim_start_matches("@as")
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snapshot_json() {
        let json = r#"{"windows":[{"id":12,"title":"Doc","appId":"org.gnome.TextEditor.desktop","wmClass":"gnome-text-editor","pid":42,"minimized":false},
            {"id":13,"title":"x","appId":null,"wmClass":null,"pid":0,"minimized":true}],
            "focus":{"id":12,"pid":42,"fullscreen":false},"overview":false}"#;
        let snap: ShellSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.windows.len(), 2);
        assert_eq!(snap.windows[0].app_id.as_deref(), Some("org.gnome.TextEditor.desktop"));
        assert_eq!(snap.focus.unwrap().pid, 42);
        let empty: ShellSnapshot = serde_json::from_str(r#"{"windows":[],"focus":null,"overview":true}"#).unwrap();
        assert!(empty.focus.is_none() && empty.overview);
    }

    #[test]
    fn parses_enabled_extensions() {
        assert!(parse_gvariant_strv("@as []").is_empty());
        assert_eq!(parse_gvariant_strv("['a@b', 'c@d']\n"), vec!["a@b", "c@d"]);
    }

    #[test]
    #[ignore = "smoke test: precisa da extensão carregada no GNOME"]
    fn lists_windows_through_extension() {
        println!("{:#?}", list_windows().expect("extensão indisponível"));
    }
}
