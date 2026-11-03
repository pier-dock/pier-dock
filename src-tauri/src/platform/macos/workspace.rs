//! Apps em execução (NSWorkspace / NSRunningApplication): o que o Dock nativo mostra como aberto.

use objc2::rc::Retained;
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use objc2_foundation::NSString;

use crate::util::spawn_detached;

#[derive(Clone, Debug)]
pub struct RunningApp {
    pub pid: i32,
    pub bundle_id: Option<String>,
    /// Caminho do bundle `.app`.
    pub path: Option<String>,
    pub name: String,
    pub hidden: bool,
}

fn describe(app: &NSRunningApplication) -> RunningApp {
    let path = app.bundleURL().and_then(|url| url.path()).map(|p| p.to_string());
    let name = app
        .localizedName()
        .map(|n| n.to_string())
        .or_else(|| path.as_deref().and_then(|p| p.rsplit('/').next()).map(|f| f.trim_end_matches(".app").to_string()))
        .unwrap_or_default();
    RunningApp {
        pid: app.processIdentifier(),
        bundle_id: app.bundleIdentifier().map(|b| b.to_string()),
        path,
        name,
        hidden: app.isHidden(),
    }
}

/// Apps "regulares" (com ícone no Dock). O Pier só é regular enquanto uma janela comum está aberta.
pub fn running_apps() -> Vec<RunningApp> {
    NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|a| a.activationPolicy() == NSApplicationActivationPolicy::Regular)
        .map(|a| describe(&a))
        .collect()
}

pub fn frontmost() -> Option<RunningApp> {
    NSWorkspace::sharedWorkspace().frontmostApplication().map(|a| describe(&a))
}

fn by_pid(pid: i32) -> Option<Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
}

pub fn pid_of(bundle_id: &str) -> Option<i32> {
    let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle_id));
    apps.iter().find(|a| !a.isTerminated()).map(|a| a.processIdentifier())
}

pub fn is_hidden(pid: i32) -> bool {
    by_pid(pid).is_some_and(|a| a.isHidden())
}

pub fn activate(pid: i32) {
    if let Some(app) = by_pid(pid) {
        app.unhide();
        // Depreciado no macOS 14 em favor da ativação "cooperativa", que só cede o foco se o app
        // ativo pedir; aqui é o usuário quem pediu, clicando na dock.
        #[allow(deprecated)]
        app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
    }
}

/// Ativa o app e manda o evento de reopen, como o clique no Dock nativo: é o que faz um app aberto
/// sem janelas (o Finder, por exemplo) abrir uma. Quem já tem janelas só vem para a frente.
pub fn reopen(pid: i32) {
    activate(pid);
    if let Some(path) = by_pid(pid).and_then(|app| describe(&app).path) {
        let _ = spawn_detached("/usr/bin/open", &[&path]);
    }
}

pub fn hide(pid: i32) {
    if let Some(app) = by_pid(pid) {
        app.hide();
    }
}

pub fn terminate(pid: i32) {
    if let Some(app) = by_pid(pid) {
        app.terminate();
    }
}
