//! Apps no macOS: bundles `.app` das pastas de aplicativos, abertos com `open` (LaunchServices).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use objc2::rc::autoreleasepool;
use objc2_foundation::{NSFileManager, NSString};

use super::app_id;
use crate::config::AppEntry;
use crate::util::spawn_detached;

const APP_DIRS: [&str; 4] = ["/Applications", "/Applications/Utilities", "/System/Applications", "/System/Applications/Utilities"];
/// Fica fora das pastas de aplicativos, mas é o app mais fixado de todos.
const FINDER: &str = "/System/Library/CoreServices/Finder.app";

struct Bundle {
    id: Option<String>,
    /// Apps sem ícone no Dock (`LSUIElement`, `LSBackgroundOnly`): agentes, helpers.
    background: bool,
}

fn plist_flag(value: Option<&plist::Value>) -> bool {
    match value {
        Some(plist::Value::Boolean(b)) => *b,
        Some(plist::Value::String(s)) => matches!(s.to_ascii_lowercase().as_str(), "1" | "yes" | "true"),
        Some(plist::Value::Integer(n)) => n.as_signed() == Some(1),
        _ => false,
    }
}

fn read_bundle(path: &Path) -> Option<Bundle> {
    let info: plist::Dictionary = plist::from_file(path.join("Contents/Info.plist")).ok()?;
    Some(Bundle {
        id: info.get("CFBundleIdentifier").and_then(|v| v.as_string()).map(str::to_string),
        background: plist_flag(info.get("LSUIElement")) || plist_flag(info.get("LSBackgroundOnly")),
    })
}

/// Nome que o Finder mostra, já traduzido ("Calculadora", não "Calculator").
fn display_name(path: &Path) -> String {
    let path = path.to_string_lossy();
    let name = NSFileManager::defaultManager().displayNameAtPath(&NSString::from_str(&path)).to_string();
    name.strip_suffix(".app").map(str::to_string).unwrap_or(name)
}

fn bundle_to_app(path: &Path, bundle: &Bundle) -> AppEntry {
    let target = path.to_string_lossy().into_owned();
    AppEntry {
        id: app_id(bundle.id.as_deref().unwrap_or(&target)),
        name: display_name(path),
        exe: Some(target.clone()),
        target,
        aumid: bundle.id.clone(),
    }
}

fn is_app_bundle(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "app") && path.is_dir()
}

/// Bundles numa pasta e nas subpastas diretas dela (ex.: `/Applications/Setapp`).
fn bundles_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if is_app_bundle(&path) {
            out.push(path);
        } else if path.is_dir() {
            if let Ok(inner) = std::fs::read_dir(&path) {
                out.extend(inner.flatten().map(|e| e.path()).filter(|p| is_app_bundle(p)));
            }
        }
    }
}

pub fn installed_apps() -> Result<Vec<AppEntry>, String> {
    autoreleasepool(|_| list_installed_apps())
}

fn list_installed_apps() -> Result<Vec<AppEntry>, String> {
    let mut dirs: Vec<PathBuf> = APP_DIRS.iter().map(PathBuf::from).collect();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Applications"));
    }
    let mut paths = vec![PathBuf::from(FINDER)];
    for dir in &dirs {
        bundles_in(dir, &mut paths);
    }
    let mut seen = HashSet::new();
    let mut apps: Vec<AppEntry> = paths
        .iter()
        .filter_map(|p| read_bundle(p).filter(|b| !b.background).map(|b| bundle_to_app(p, &b)))
        .filter(|a| seen.insert(a.name.to_lowercase()))
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    Ok(apps)
}

/// Transforma um item arrastado para a dock (.app, pasta, arquivo) num app fixável.
pub fn resolve_path(path: &str) -> AppEntry {
    let p = Path::new(path.trim_end_matches('/'));
    if is_app_bundle(p) {
        if let Some(bundle) = read_bundle(p) {
            return bundle_to_app(p, &bundle);
        }
    }
    AppEntry {
        id: app_id(path),
        name: display_name(p),
        target: p.to_string_lossy().into_owned(),
        exe: None,
        aumid: None,
    }
}

pub fn launch(target: &str) -> Result<(), String> {
    let path = Path::new(target);
    // Um app fixado que mudou de lugar (ex.: movido para uma subpasta) ainda abre pelo nome.
    if !path.exists() && target.ends_with(".app") {
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        return spawn_detached("/usr/bin/open", &["-a", &name]);
    }
    spawn_detached("/usr/bin/open", &[target])
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "smoke test: depende dos apps instalados"]
    fn lists_installed_apps_and_icons() {
        let apps = super::installed_apps().unwrap();
        println!("{} apps", apps.len());
        for a in apps.iter().take(15) {
            println!("{} | {} | {:?}", a.name, a.target, a.aumid);
        }
        assert!(apps.iter().any(|a| a.aumid.as_deref() == Some("com.apple.finder")));
        let icon = crate::icons::icon_data_url(&apps[0].target).unwrap();
        assert!(icon.starts_with("data:image/png"));
    }
}
