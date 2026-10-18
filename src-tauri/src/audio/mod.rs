//! Popover de volume: saídas de áudio (e qual é a padrão) e o volume de cada app. O volume geral
//! continua em `system::get_volume`/`set_volume`. Cada sistema tem um backend com `outputs`,
//! `set_output`, `apps` e `set_app_volume`.

#[cfg_attr(target_os = "windows", path = "win32.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
mod backend;

use serde::Serialize;

use crate::util::ErrStr;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub default: bool,
}

/// Um app tocando áudio. Os streams do mesmo app viram uma linha só, e `id` identifica todos eles.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppStream {
    pub id: String,
    pub name: String,
    /// Chave para o `get_icon` (caminho do executável).
    pub icon: Option<String>,
    pub level: f32,
    pub muted: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub outputs: Vec<OutputDevice>,
    /// `None` quando o sistema não deixa mexer no volume de cada app (macOS).
    pub apps: Option<Vec<AppStream>>,
}

#[tauri::command]
pub async fn audio_status() -> Result<AudioState, String> {
    tauri::async_runtime::spawn_blocking(|| AudioState { outputs: backend::outputs().unwrap_or_default(), apps: backend::apps() })
        .await
        .err_str()
}

#[tauri::command]
pub async fn set_output_device(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::set_output(&id)).await.err_str()?
}

#[tauri::command]
pub async fn set_app_volume(id: String, level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || backend::set_app_volume(&id, level.map(|l| l.clamp(0.0, 1.0)), muted))
        .await
        .err_str()?
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "smoke test: lê as saídas e os apps do sistema"]
    fn reads_audio_state() {
        println!("{:?}", super::backend::outputs());
        println!("{:?}", super::backend::apps());
    }
}
