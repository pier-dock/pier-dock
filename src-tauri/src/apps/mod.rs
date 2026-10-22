//! Apps instalados (a mesma lista do menu de apps do sistema), resolução de arquivos arrastados
//! para a dock e abertura de apps. Cada sistema tem um backend com `installed_apps`,
//! `resolve_path` e `launch`.

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

use crate::config::AppEntry;
use crate::util::{reject_network_path, ErrStr};

fn app_id(seed: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.to_lowercase().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(windows)]
pub(crate) fn installed_apps() -> windows::core::Result<Vec<AppEntry>> {
    backend::installed_apps()
}

#[tauri::command]
pub async fn list_installed_apps() -> Result<Vec<AppEntry>, String> {
    tauri::async_runtime::spawn_blocking(|| backend::installed_apps().err_str()).await.err_str()?
}

#[tauri::command]
pub async fn launch_app(target: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        reject_network_path(&target)?;
        backend::launch(&target)
    })
    .await
    .err_str()?
}

#[tauri::command]
pub async fn resolve_app_path(path: String) -> Result<AppEntry, String> {
    reject_network_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || backend::resolve_path(&path)).await.err_str()
}
