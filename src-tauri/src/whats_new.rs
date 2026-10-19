//! Janela de novidades: abre uma vez na primeira execução depois de uma atualização, com as seções
//! do `CHANGELOG.md` desde a última versão aberta. A última versão fica num arquivo próprio, fora
//! da config, para não disputar com o `save_config` do frontend.

use std::path::PathBuf;

use semver::Version;
use tauri::AppHandle;

use crate::util::data_dir;

const CHANGELOG: &str = include_str!("../../CHANGELOG.md");

fn last_version_path() -> PathBuf {
    data_dir().join("last-version")
}

fn has_section(changelog: &str, version: &Version) -> bool {
    let version = version.to_string();
    changelog
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .any(|heading| heading.trim().trim_start_matches('v').split_whitespace().next() == Some(version.as_str()))
}

/// `Some(since)` abre a janela com as versões depois de `since` (vazio: só a atual).
///
/// Sem `last-version`, quem já tem uma config está vindo de uma versão anterior a este recurso e vê
/// só a atual; sem config, é uma instalação nova e não vê nada.
fn since_to_show(last: Option<&str>, existing_user: bool, current: &Version, changelog: &str) -> Option<String> {
    if !has_section(changelog, current) {
        return None;
    }
    match last.and_then(|l| Version::parse(l.trim()).ok()) {
        Some(last) if last < *current => Some(last.to_string()),
        Some(_) => None,
        None => existing_user.then(String::new),
    }
}

/// Fora do build de release não faz nada: o `tauri dev` abriria a janela a cada execução.
pub fn start(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let current = &app.package_info().version;
    let last = std::fs::read_to_string(last_version_path()).ok();
    let existing_user = data_dir().join("config.json").exists();
    let since = since_to_show(last.as_deref(), existing_user, current, CHANGELOG);
    if last.as_deref().map(str::trim) != Some(current.to_string().as_str()) {
        if let Err(e) = std::fs::write(last_version_path(), current.to_string()) {
            log::warn!("falha ao gravar a última versão aberta: {e}");
        }
    }
    if let Some(since) = since {
        if let Err(e) = crate::panels::open_whats_new_window(app, &since) {
            log::error!("falha ao abrir as novidades: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANGELOG: &str = "# Changelog\n\n## Unreleased\n\n## 0.8.0\n\n### A\n\n## v0.7.0\n";

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn finds_version_sections() {
        assert!(has_section(CHANGELOG, &v("0.8.0")));
        assert!(has_section(CHANGELOG, &v("0.7.0")));
        assert!(!has_section(CHANGELOG, &v("0.9.0")));
    }

    #[test]
    fn shows_once_after_an_update() {
        assert_eq!(since_to_show(Some("0.7.0\n"), true, &v("0.8.0"), CHANGELOG), Some("0.7.0".into()));
        assert_eq!(since_to_show(Some("0.8.0"), true, &v("0.8.0"), CHANGELOG), None);
        assert_eq!(since_to_show(Some("0.9.0"), true, &v("0.8.0"), CHANGELOG), None);
    }

    #[test]
    fn first_run_with_this_feature() {
        assert_eq!(since_to_show(None, true, &v("0.8.0"), CHANGELOG), Some(String::new()));
        assert_eq!(since_to_show(None, false, &v("0.8.0"), CHANGELOG), None);
        assert_eq!(since_to_show(Some("garbage"), true, &v("0.8.0"), CHANGELOG), Some(String::new()));
    }

    #[test]
    fn skips_versions_without_notes() {
        assert_eq!(since_to_show(Some("0.8.0"), true, &v("0.9.0"), CHANGELOG), None);
    }
}
