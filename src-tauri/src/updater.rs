//! Atualização automática pelo GitHub Releases (o `latest.json` que o workflow de release publica).
//! A versão nova aparece no menu da bandeja e na aba Geral das configurações.

use std::sync::Mutex;
#[cfg(not(debug_assertions))]
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{IsMenuItem, Menu, MenuItem};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::util::ErrStr;

pub const MENU_ID: &str = "update";
#[cfg(not(debug_assertions))]
const CHECK_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Default)]
pub struct UpdateState {
    pending: Mutex<Option<Update>>,
    tray_item: Mutex<Option<MenuItem<Wry>>>,
}

/// O menu da bandeja, guardado para inserir o item de atualização quando houver uma.
pub struct TrayMenu(pub Menu<Wry>);

async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app.updater().err_str()?.check().await.err_str()?;
    let info = update.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() });
    *app.state::<UpdateState>().pending.lock().unwrap() = update;
    if let Some(info) = &info {
        announce(app, info);
    }
    Ok(info)
}

fn announce(app: &AppHandle, info: &UpdateInfo) {
    let label = update_label(app, &info.version);
    let state = app.state::<UpdateState>();
    let mut tray_item = state.tray_item.lock().unwrap();
    match tray_item.as_ref() {
        Some(item) => {
            let _ = item.set_text(&label);
        }
        None => {
            if let (Some(menu), Ok(item)) = (app.try_state::<TrayMenu>(), MenuItem::with_id(app, MENU_ID, &label, true, None::<&str>)) {
                if menu.0.insert(&item as &dyn IsMenuItem<Wry>, 0).is_ok() {
                    *tray_item = Some(item);
                }
            }
        }
    }
    let _ = app.emit_to(crate::panels::SETTINGS_LABEL, "update-available", info.clone());
}

fn update_label(app: &AppHandle, version: &str) -> String {
    format!("{} {version}", crate::i18n::current(app).t("Atualizar para", "Update to"))
}

/// Traduz o item de atualização da bandeja, se houver um, quando o idioma muda.
pub fn relabel(app: &AppHandle) {
    let state = app.state::<UpdateState>();
    let version = state.pending.lock().unwrap().as_ref().map(|u| u.version.clone());
    let item = state.tray_item.lock().unwrap().clone();
    if let (Some(item), Some(version)) = (item, version) {
        let _ = item.set_text(update_label(app, &version));
    }
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let pending = app.state::<UpdateState>().pending.lock().unwrap().take();
    let update = match pending {
        Some(update) => update,
        None => app.updater().err_str()?.check().await.err_str()?.ok_or("Nenhuma atualização disponível")?,
    };
    let bytes = update.download(|_, _| {}, || {}).await.err_str()?;
    // No Windows o instalador encerra o processo sem passar pelo `RunEvent::Exit`: a barra nativa
    // precisa voltar antes.
    crate::taskbar::restore();
    if let Err(e) = update.install(bytes) {
        if app.state::<crate::config::ConfigState>().0.lock().unwrap().hide_taskbar {
            crate::taskbar::hide();
        }
        return Err(e.to_string());
    }
    app.restart();
}

/// Procura atualização um pouco depois de abrir (para não disputar com a inicialização) e depois
/// a cada 12h.
#[cfg(not(debug_assertions))]
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(30));
        loop {
            if let Err(e) = tauri::async_runtime::block_on(check(&app)) {
                log::warn!("falha ao procurar atualização: {e}");
            }
            std::thread::sleep(CHECK_INTERVAL);
        }
    });
}
