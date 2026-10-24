use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::util::{data_dir, write_atomic, ErrStr};

const MAX_POKEMON: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    /// O que é aberto ao clicar: caminho de .exe/.lnk/pasta ou `shell:AppsFolder\<AUMID>` no
    /// Windows, arquivo .desktop no Linux, bundle .app no macOS.
    pub target: String,
    /// Executável real, usado para casar o app fixado com janelas abertas.
    pub exe: Option<String>,
    /// Identidade do app, também usada para casar janelas: AppUserModelID (apps da Store, PWAs)
    /// no Windows, desktop id (`firefox.desktop`) no Linux, bundle id no macOS.
    pub aumid: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct WidgetConfig {
    pub kind: String,
    pub enabled: bool,
    pub size: String,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self { kind: String::new(), enabled: true, size: "small".into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct WeatherLocation {
    pub name: String,
    /// Estado ou região, para diferenciar cidades homônimas.
    pub admin: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PomodoroConfig {
    /// "25-5", "50-10" ou "custom" (usa `focus_minutes` e `break_minutes`).
    pub preset: String,
    pub focus_minutes: u32,
    pub break_minutes: u32,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self { preset: "25-5".into(), focus_minutes: 25, break_minutes: 5 }
    }
}

impl PomodoroConfig {
    /// Minutos de foco e de pausa curta do ciclo escolhido.
    pub fn durations(&self) -> (u32, u32) {
        match self.preset.as_str() {
            "50-10" => (50, 10),
            "custom" => (self.focus_minutes.clamp(1, 180), self.break_minutes.clamp(1, 60)),
            _ => (25, 5),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub pinned_apps: Vec<AppEntry>,
    pub widgets: Vec<WidgetConfig>,
    /// "glass" (vidro líquido, onde o sistema permite) ou "solid".
    pub material: String,
    /// Usa material sólido e remove sombras decorativas sem perder o material escolhido.
    pub reduced_effects: bool,
    /// Opacidade da tinta sobre o vidro da dock (0 a 1).
    pub glass_opacity: f32,
    /// Cor de destaque: "coral", "blue", "green", "amber" ou "lilac".
    pub accent: String,
    pub icon_size: u32,
    pub magnification: f32,
    pub show_running_unpinned: bool,
    pub hide_taskbar: bool,
    /// "never", "smart" (esconde quando um app está em foco) ou "always".
    pub auto_hide: String,
    /// "fit" (barra do tamanho do conteúdo, centralizada) ou "full" (ponta a ponta, como um painel).
    pub dock_width: String,
    /// Borda da tela onde a dock fica: "bottom", "top", "left" ou "right".
    pub dock_position: String,
    /// Monitor da dock: "primary" ou o id de um monitor (`dock_window::list_monitors`).
    pub dock_monitor: String,
    /// Altura da barra em pixels; `None` acompanha o tamanho do ícone (`iconSize + 24`).
    pub dock_height: Option<u32>,
    pub github_poll_minutes: u32,
    /// De quantos em quantos minutos o uso das IAs de linha de comando é relido do disco.
    pub ai_poll_minutes: u32,
    /// Qual IA o widget de uso lê: "auto" (a de leitura mais recente), "claude" ou "codex".
    pub ai_provider: String,
    /// Ordem dos itens na dock: `start`, `app:<id>`, `widget:<kind>` e `divider:<id>`. Vazia em
    /// configs antigas; o frontend completa e limpa a lista.
    pub dock_order: Vec<String>,
    pub show_start_button: bool,
    /// Botão do lançador de apps do próprio Pier (a lista de todos os instalados).
    pub show_launcher: bool,
    /// Apps fixados no topo do lançador, em galeria. Independe dos fixados na dock.
    pub launcher_pinned: Vec<AppEntry>,
    pub weather_location: Option<WeatherLocation>,
    pub pomodoro: PomodoroConfig,
    /// Idioma da interface: "system", "pt" ou "en".
    pub language: String,
    /// Instala sozinho a atualização encontrada logo depois de abrir.
    pub auto_update: bool,
    /// Mascote decorativo que anda sobre a borda da dock, reagindo às sessões dos CLIs de IA (ver
    /// `agent_sessions.rs`). Puramente visual; sem hooks instalados fica só parado.
    pub claude_pet_enabled: bool,
    /// Mascote das sessões de IA: "clawd" ou "pokemon". O Pokémon usa `agent_pokemon_id`,
    /// independente do `pokemon_pet_ids` do widget Pokémon.
    pub agent_pet_mascot: String,
    /// Sprite do Pokémon usado como mascote das sessões de IA.
    pub agent_pokemon_id: u32,
    /// Pokémon que andam sobre a borda da dock, com o ícone de time da PokeAPI (ver `pokemon.rs`).
    /// Até `MAX_POKEMON`, sem repetir; o primeiro também é o ícone do widget.
    pub pokemon_pet_enabled: bool,
    pub pokemon_pet_ids: Vec<u32>,
}

fn widget(kind: &str, size: &str) -> WidgetConfig {
    WidgetConfig { kind: kind.into(), enabled: true, size: size.into() }
}

fn disabled(kind: &str, size: &str) -> WidgetConfig {
    WidgetConfig { enabled: false, ..widget(kind, size) }
}

/// O gerenciador de arquivos do sistema, fixado na primeira execução.
fn file_manager() -> AppEntry {
    #[cfg(windows)]
    return AppEntry {
        id: "explorer".into(),
        name: crate::i18n::system().t("Explorador de Arquivos", "File Explorer").into(),
        target: r"C:\Windows\explorer.exe".into(),
        exe: Some(r"C:\Windows\explorer.exe".into()),
        aumid: Some("Microsoft.Windows.Explorer".into()),
    };
    #[cfg(target_os = "linux")]
    return AppEntry {
        id: "files".into(),
        name: crate::i18n::system().t("Arquivos", "Files").into(),
        target: "/usr/share/applications/org.gnome.Nautilus.desktop".into(),
        exe: Some("/usr/bin/nautilus".into()),
        aumid: Some("org.gnome.Nautilus.desktop".into()),
    };
    #[cfg(target_os = "macos")]
    return AppEntry {
        id: "finder".into(),
        name: "Finder".into(),
        target: "/System/Library/CoreServices/Finder.app".into(),
        exe: Some("/System/Library/CoreServices/Finder.app".into()),
        aumid: Some("com.apple.finder".into()),
    };
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pinned_apps: vec![file_manager()],
            widgets: vec![
                widget("media", "wide"),
                widget("github-mine", "small"),
                widget("github-review", "small"),
                disabled("github-commits", "small"),
                widget("github-contrib", "wide"),
                if cfg!(windows) { widget("tray", "small") } else { disabled("tray", "small") },
                widget("network", "small"),
                widget("volume", "small"),
                widget("bluetooth", "small"),
                widget("battery", "small"),
                widget("clock", "small"),
                disabled("weather", "wide"),
                disabled("pomodoro", "wide"),
                disabled("system-usage", "wide"),
                disabled("calendar", "wide"),
                disabled("ai-usage", "small"),
                disabled("ai-agents", "small"),
                disabled("pokemon", "small"),
                disabled("show-desktop", "small"),
            ],
            material: "glass".into(),
            reduced_effects: false,
            glass_opacity: 0.4,
            accent: "coral".into(),
            icon_size: 40,
            magnification: 1.6,
            show_running_unpinned: true,
            hide_taskbar: true,
            auto_hide: "smart".into(),
            dock_width: "fit".into(),
            dock_position: "bottom".into(),
            dock_monitor: "primary".into(),
            dock_height: None,
            github_poll_minutes: 3,
            ai_poll_minutes: 1,
            ai_provider: "auto".into(),
            dock_order: Vec::new(),
            show_start_button: true,
            show_launcher: false,
            launcher_pinned: Vec::new(),
            weather_location: None,
            pomodoro: PomodoroConfig::default(),
            language: "system".into(),
            auto_update: true,
            claude_pet_enabled: false,
            agent_pet_mascot: "clawd".into(),
            agent_pokemon_id: 25,
            pokemon_pet_enabled: false,
            pokemon_pet_ids: vec![25],
        }
    }
}

impl Config {
    pub fn effective_material(&self) -> &str {
        if self.reduced_effects { "solid" } else { &self.material }
    }
}

pub struct ConfigState(pub Mutex<Config>);

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

/// Abriu sem config salva: instalação nova (ou ninguém mexeu em nada ainda).
static FRESH_INSTALL: AtomicBool = AtomicBool::new(false);

/// A config do primeiro uso: a barra do sistema só some depois que o usuário aceita, nas boas-vindas.
pub fn load_first_run_aware() -> (Config, Option<String>) {
    let fresh = !config_path().exists();
    FRESH_INSTALL.store(fresh, Ordering::Relaxed);
    let (config, issue) = load();
    (if fresh { Config { hide_taskbar: false, ..config } } else { config }, issue)
}

/// As boas-vindas aparecem até a primeira config ser gravada.
#[tauri::command]
pub fn first_run() -> bool {
    FRESH_INSTALL.load(Ordering::Relaxed) && !config_path().exists()
}

/// `config.json` + `.suffix`, na mesma pasta.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{suffix}"));
    PathBuf::from(name)
}

/// A config salva e, se o arquivo estava ilegível, o que foi feito com ele. O aviso volta para quem
/// chama porque o log ainda não existe quando a config é lida.
pub fn load() -> (Config, Option<String>) {
    load_from(&config_path())
}

fn load_from(path: &Path) -> (Config, Option<String>) {
    let error = match read_file(path) {
        Ok(Some(config)) => return (config, None),
        Ok(None) => return (Config::default(), None),
        Err(e) => e,
    };
    // Um arquivo ilegível nunca é sobrescrito: fica de lado para o usuário recuperar, e a cópia da
    // gravação anterior entra no lugar.
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let kept = sibling(path, &format!("corrupt-{secs}"));
    let _ = std::fs::rename(path, &kept);
    let problem = format!("{} could not be read ({error}), kept as {}", path.display(), kept.display());
    match read_file(&sibling(path, "bak")) {
        Ok(Some(config)) => {
            let restored = serde_json::to_string_pretty(&config).map_err(|e| e.to_string()).and_then(|json| write_atomic(path, json).err_str());
            (config, Some(format!("{problem}; restored the backup{}", restored.err().map(|e| format!(" (not saved: {e})")).unwrap_or_default())))
        }
        _ => (Config::default(), Some(format!("{problem}; no usable backup, using the defaults"))),
    }
}

/// `Ok(None)`: o arquivo não existe (instalação nova).
fn read_file(path: &Path) -> Result<Option<Config>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    read(serde_json::from_str(&text).err_str()?).map(Some)
}

/// Uma config salva, com as migrações e os widgets que surgiram depois dela.
fn read(value: serde_json::Value) -> Result<Config, String> {
    serde_json::from_value(migrate(value)).map(add_missing_widgets).map(normalize).err_str()
}

/// Uma config importada ou editada à mão pode trazer ids repetidos, fora da PokeAPI ou demais.
fn normalize(mut config: Config) -> Config {
    let mut seen = Vec::new();
    for id in config.pokemon_pet_ids {
        if (1..=crate::pokemon::MAX_ID).contains(&id) && !seen.contains(&id) && seen.len() < MAX_POKEMON {
            seen.push(id);
        }
    }
    if seen.is_empty() {
        seen.push(25);
    }
    config.pokemon_pet_ids = seen;
    config
}

/// Uma config importada de um arquivo. Os campos que faltam ganham o padrão, como no `load`, mas um
/// JSON sem `widgets` nem `pinnedApps` não é uma config do Pier.
pub fn parse(json: &str) -> Result<Config, String> {
    let value: serde_json::Value = serde_json::from_str(json).err_str()?;
    let is_config = value.as_object().is_some_and(|o| o.contains_key("widgets") || o.contains_key("pinnedApps"));
    if !is_config {
        return Err(crate::i18n::tr("o arquivo não é uma configuração do Pier", "the file isn't a Pier configuration").into());
    }
    read(value)
}

/// Widgets que surgiram depois que a config foi salva entram desligados, no fim da lista; sem
/// isso eles nunca apareceriam nas configurações.
fn add_missing_widgets(mut config: Config) -> Config {
    for default in Config::default().widgets {
        if !config.widgets.iter().any(|w| w.kind == default.kind) {
            config.widgets.push(WidgetConfig { enabled: false, ..default });
        }
    }
    config
}

/// Reescritas que a config acumulou:
///
/// - os temas antigos viram material + cor de destaque: "glass" continua vidro; os outros eram
///   opacos e ficam sólidos, com o destaque mais próximo da cor do tema;
/// - o widget "claude-usage" virou "ai-usage" quando ele passou a ler mais de uma IA, na lista de
///   widgets e na ordem da dock, e `claudePollMinutes` virou `aiPollMinutes`;
/// - o widget "claude-agents" virou "ai-agents" quando passou a mostrar sessões de outros CLIs;
/// - `pokemonPetId` virou a lista `pokemonPetIds` quando o widget Pokémon passou a aceitar vários;
/// - o "theme" já aplicado é removido sem tocar no resto.
fn migrate(mut value: serde_json::Value) -> serde_json::Value {
    let Some(obj) = value.as_object_mut() else { return value };

    if let Some(theme) = obj.remove("theme") {
        if !obj.contains_key("material") {
            let (material, accent) = match theme.as_str().unwrap_or("glass") {
                "glass" => ("glass", "coral"),
                "midnight" => ("solid", "lilac"),
                "canarinho" => ("solid", "amber"),
                _ => ("solid", "blue"),
            };
            obj.insert("material".into(), material.into());
            obj.entry("accent").or_insert(accent.into());
        }
    }

    if !obj.contains_key("aiPollMinutes") {
        if let Some(minutes) = obj.remove("claudePollMinutes") {
            obj.insert("aiPollMinutes".into(), minutes);
        }
    }

    if !obj.contains_key("pokemonPetIds") {
        if let Some(id) = obj.remove("pokemonPetId") {
            obj.insert("pokemonPetIds".into(), vec![id].into());
        }
    }

    if let Some(widgets) = obj.get_mut("widgets").and_then(serde_json::Value::as_array_mut) {
        for widget in widgets {
            match widget.get("kind").and_then(serde_json::Value::as_str) {
                Some("claude-usage") => widget["kind"] = "ai-usage".into(),
                Some("claude-agents") => widget["kind"] = "ai-agents".into(),
                _ => {}
            }
        }
    }
    if let Some(order) = obj.get_mut("dockOrder").and_then(serde_json::Value::as_array_mut) {
        for key in order {
            match key.as_str() {
                Some("widget:claude-usage") => *key = "widget:ai-usage".into(),
                Some("widget:claude-agents") => *key = "widget:ai-agents".into(),
                _ => {}
            }
        }
    }
    value
}

fn persist(config: &Config) -> Result<(), String> {
    persist_to(&config_path(), config)
}

fn persist_to(path: &Path, config: &Config) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).err_str()?;
    // A gravação anterior vira o backup que o `load` usa se esta ficar ilegível.
    if path.exists() {
        let _ = std::fs::copy(path, sibling(path, "bak"));
    }
    write_atomic(path, json).err_str()
}

#[tauri::command]
pub fn get_config(state: State<ConfigState>) -> Config {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(app: AppHandle, state: State<ConfigState>, config: Config) -> Result<(), String> {
    apply(&app, &state, config)
}

/// Salva a config atual num arquivo escolhido pelo usuário. `false` se ele cancelar. O token do
/// GitHub e o link da agenda ficam no chaveiro, fora da config, e não vão junto.
#[tauri::command]
pub async fn export_config(app: AppHandle, window: WebviewWindow) -> Result<bool, String> {
    let json = serde_json::to_string_pretty(&*app.state::<ConfigState>().0.lock().unwrap()).err_str()?;
    let title = crate::i18n::current(&app).t("Exportar configuração", "Export settings");
    let dialog = app.dialog().file().set_parent(&window).set_title(title).set_file_name("pier-config.json").add_filter("JSON", &["json"]);
    let Some(path) = tauri::async_runtime::spawn_blocking(move || dialog.blocking_save_file()).await.err_str()? else {
        return Ok(false);
    };
    std::fs::write(path.into_path().err_str()?, json).err_str()?;
    Ok(true)
}

/// Troca a config pela de um arquivo exportado. `false` se o usuário cancelar.
#[tauri::command]
pub async fn import_config(app: AppHandle, window: WebviewWindow) -> Result<bool, String> {
    let title = crate::i18n::current(&app).t("Importar configuração", "Import settings");
    let dialog = app.dialog().file().set_parent(&window).set_title(title).add_filter("JSON", &["json"]);
    let Some(path) = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file()).await.err_str()? else {
        return Ok(false);
    };
    let config = parse(&std::fs::read_to_string(path.into_path().err_str()?).err_str()?)?;
    apply(&app, &app.state::<ConfigState>(), config)?;
    Ok(true)
}

/// Volta todas as configurações ao padrão. As contas (token do GitHub, link da agenda) ficam no
/// chaveiro e continuam.
#[tauri::command]
pub fn reset_config(app: AppHandle, state: State<ConfigState>) -> Result<(), String> {
    apply(&app, &state, Config::default())
}

/// Grava a config e aplica o que muda fora da página (idioma, dock, material, barra nativa); as
/// janelas recebem a nova pelo `config-changed`.
pub fn apply(app: &AppHandle, state: &ConfigState, config: Config) -> Result<(), String> {
    let config = normalize(config);
    persist(&config)?;
    let (old_hide, old_language) = {
        let mut guard = state.0.lock().unwrap();
        let old = (guard.hide_taskbar, guard.language.clone());
        *guard = config.clone();
        old
    };
    if old_language != config.language {
        crate::i18n::apply(app);
    }
    crate::dock_window::set_auto_hide(app, &config.auto_hide);
    crate::dock_window::set_position(app, &config.dock_position);
    crate::dock_window::set_monitor(app, &config.dock_monitor);
    crate::backdrop::set_material(app, config.effective_material());
    crate::pomodoro::config_changed(app);
    if old_hide != config.hide_taskbar {
        if config.hide_taskbar {
            crate::taskbar::hide();
        } else {
            crate::taskbar::restore();
        }
    }
    app.emit("config-changed", &config).err_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pier-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    #[test]
    fn missing_config_is_a_fresh_install() {
        let dir = temp_dir("missing");
        let (config, issue) = load_from(&dir.join("config.json"));
        assert_eq!(config, Config::default());
        assert!(issue.is_none());
        assert!(leftovers(&dir).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn persist_keeps_the_previous_save_as_backup() {
        let dir = temp_dir("persist");
        let path = dir.join("config.json");
        persist_to(&path, &Config { icon_size: 40, ..Config::default() }).unwrap();
        persist_to(&path, &Config { icon_size: 56, ..Config::default() }).unwrap();
        assert_eq!(load_from(&path).0.icon_size, 56);
        assert_eq!(read_file(&sibling(&path, "bak")).unwrap().unwrap().icon_size, 40);
        assert_eq!(leftovers(&dir), ["config.json", "config.json.bak"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unreadable_config_is_kept_aside_and_the_backup_restored() {
        let dir = temp_dir("restore");
        let path = dir.join("config.json");
        persist_to(&path, &Config { icon_size: 40, ..Config::default() }).unwrap();
        persist_to(&path, &Config { icon_size: 56, ..Config::default() }).unwrap();
        std::fs::write(&path, r#"{"iconSize": 56, "widg"#).unwrap();
        let (config, issue) = load_from(&path);
        assert_eq!(config.icon_size, 40);
        assert!(issue.unwrap().contains("restored the backup"));
        assert_eq!(read_file(&path).unwrap().unwrap().icon_size, 40);
        let corrupt = leftovers(&dir).into_iter().find(|n| n.starts_with("config.json.corrupt-")).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join(corrupt)).unwrap(), r#"{"iconSize": 56, "widg"#);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unreadable_config_without_backup_falls_back_to_defaults_without_deleting_it() {
        let dir = temp_dir("nobackup");
        let path = dir.join("config.json");
        std::fs::write(&path, "not json").unwrap();
        let (config, issue) = load_from(&path);
        assert_eq!(config, Config::default());
        assert!(issue.unwrap().contains("using the defaults"));
        assert!(!path.exists());
        let names = leftovers(&dir);
        assert_eq!(names.len(), 1);
        assert_eq!(std::fs::read_to_string(dir.join(&names[0])).unwrap(), "not json");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn config_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<Config>(&json).unwrap(), config);
    }

    #[test]
    fn reduced_effects_override_material_without_losing_the_choice() {
        let old: Config = serde_json::from_str(r#"{"material":"glass"}"#).unwrap();
        assert!(!old.reduced_effects);
        assert_eq!(old.effective_material(), "glass");
        let mut reduced = Config { reduced_effects: true, ..old };
        assert_eq!(reduced.effective_material(), "solid");
        assert_eq!(reduced.material, "glass");
        let json = serde_json::to_string(&reduced).unwrap();
        assert!(serde_json::from_str::<Config>(&json).unwrap().reduced_effects);
        reduced.reduced_effects = false;
        assert_eq!(reduced.effective_material(), "glass");
        reduced.material = "solid".into();
        assert_eq!(reduced.effective_material(), "solid");
    }

    #[test]
    fn partial_config_fills_defaults() {
        let config: Config = serde_json::from_str(r#"{"iconSize":40}"#).unwrap();
        assert_eq!(config.material, "glass");
        assert_eq!(config.icon_size, 40);
        assert_eq!(config.accent, "coral");
        assert_eq!(config.dock_width, "fit");
        assert_eq!(config.dock_position, "bottom");
        assert_eq!(config.ai_provider, "auto");
        assert_eq!(config.agent_pet_mascot, "clawd");
        assert_eq!(config.agent_pokemon_id, 25);
        assert_eq!(config.pokemon_pet_ids, vec![25]);
        assert!(!config.widgets.is_empty());
        assert!(config.dock_order.is_empty());
        assert!(config.show_start_button);
        assert!(config.launcher_pinned.is_empty());
    }

    fn migrated(json: &str) -> Config {
        serde_json::from_value(migrate(serde_json::from_str(json).unwrap())).unwrap()
    }

    #[test]
    fn old_themes_become_material_and_accent() {
        let glass = migrated(r#"{"theme":"glass"}"#);
        assert_eq!((glass.material.as_str(), glass.accent.as_str()), ("glass", "coral"));
        let midnight = migrated(r#"{"theme":"midnight"}"#);
        assert_eq!((midnight.material.as_str(), midnight.accent.as_str()), ("solid", "lilac"));
        let paper = migrated(r#"{"theme":"paper","accent":"green"}"#);
        assert_eq!((paper.material.as_str(), paper.accent.as_str()), ("solid", "green"));
    }

    #[test]
    fn single_pokemon_id_becomes_a_list() {
        assert_eq!(migrated(r#"{"pokemonPetId":6}"#).pokemon_pet_ids, vec![6]);
        assert_eq!(migrated(r#"{"pokemonPetId":6,"pokemonPetIds":[1,2]}"#).pokemon_pet_ids, vec![1, 2]);
    }

    #[test]
    fn pokemon_ids_are_deduped_bounded_and_never_empty() {
        let ids = |json: &str| normalize(serde_json::from_str(json).unwrap()).pokemon_pet_ids;
        assert_eq!(ids(r#"{"pokemonPetIds":[6,25,6,0,810,150]}"#), vec![6, 25, 150]);
        assert_eq!(ids(r#"{"pokemonPetIds":[]}"#), vec![25]);
        let many = (1..=15).map(|n| n.to_string()).collect::<Vec<_>>().join(",");
        assert_eq!(ids(&format!(r#"{{"pokemonPetIds":[{many}]}}"#)), (1..=10).collect::<Vec<u32>>());
    }

    #[test]
    fn missing_widgets_are_appended_disabled() {
        let config: Config = serde_json::from_str(r#"{"widgets":[{"kind":"clock","enabled":true,"size":"small"}]}"#).unwrap();
        let config = add_missing_widgets(config);
        assert_eq!(config.widgets[0].kind, "clock");
        assert!(config.widgets[0].enabled);
        let weather = config.widgets.iter().find(|w| w.kind == "weather").unwrap();
        assert!(!weather.enabled);
        assert_eq!(weather.size, "wide");
        assert_eq!(config.widgets.len(), Config::default().widgets.len());
    }

    #[test]
    fn parse_migrates_an_exported_config() {
        let config = parse(r#"{"theme":"midnight","widgets":[{"kind":"claude-usage","enabled":true,"size":"small"}]}"#).unwrap();
        assert_eq!((config.material.as_str(), config.accent.as_str()), ("solid", "lilac"));
        assert_eq!(config.widgets[0].kind, "ai-usage");
        assert_eq!(config.widgets.len(), Config::default().widgets.len());
    }

    #[test]
    fn parse_rejects_other_json() {
        assert!(parse(r#"{"name":"package","version":"1.0.0"}"#).is_err());
        assert!(parse("[1, 2]").is_err());
        assert!(parse("not json").is_err());
    }

    #[test]
    fn pomodoro_presets() {
        let custom = PomodoroConfig { preset: "custom".into(), focus_minutes: 0, break_minutes: 90 };
        assert_eq!(custom.durations(), (1, 60));
        assert_eq!(PomodoroConfig { preset: "50-10".into(), ..Default::default() }.durations(), (50, 10));
        assert_eq!(PomodoroConfig::default().durations(), (25, 5));
    }

    #[test]
    fn migration_keeps_new_configs() {
        let config = migrated(r#"{"material":"solid","accent":"blue"}"#);
        assert_eq!((config.material.as_str(), config.accent.as_str()), ("solid", "blue"));
    }

    #[test]
    fn claude_agents_widget_becomes_ai_agents() {
        let config = migrated(
            r#"{
                "widgets": [
                    { "kind": "claude-agents", "enabled": true, "size": "small" },
                    { "kind": "clock", "enabled": true, "size": "small" }
                ],
                "dockOrder": ["widget:claude-agents", "divider:sistema", "widget:clock"]
            }"#,
        );
        assert_eq!(config.widgets.iter().map(|w| w.kind.as_str()).collect::<Vec<_>>(), vec!["ai-agents", "clock"]);
        assert_eq!(config.dock_order, vec!["widget:ai-agents", "divider:sistema", "widget:clock"]);
    }

    #[test]
    fn claude_widget_becomes_ai_usage() {
        let config = migrated(
            r#"{
                "claudePollMinutes": 7,
                "widgets": [
                    { "kind": "claude-usage", "enabled": true, "size": "small" },
                    { "kind": "clock", "enabled": true, "size": "small" }
                ],
                "dockOrder": ["widget:claude-usage", "divider:sistema", "widget:clock"]
            }"#,
        );
        // O tempo de polling escolhido e o lugar do widget na dock sobrevivem à troca de nome.
        assert_eq!(config.ai_poll_minutes, 7);
        assert_eq!(config.ai_provider, "auto");
        assert_eq!(config.widgets.iter().map(|w| w.kind.as_str()).collect::<Vec<_>>(), vec!["ai-usage", "clock"]);
        assert_eq!(config.dock_order, vec!["widget:ai-usage", "divider:sistema", "widget:clock"]);
    }
}
