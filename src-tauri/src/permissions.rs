//! Permissões do sistema que o Pier usa, para o resumo e o "Revisar" das configurações. Cada uma
//! tem um estado e uma ação: pedir ao sistema (quando ele ainda não perguntou) ou abrir o painel
//! certo dos Ajustes.

use serde::Serialize;
use tauri::AppHandle;

use crate::util::ErrStr;

const GRANTED: &str = "granted";
const DENIED: &str = "denied";
const NOT_DETERMINED: &str = "not-determined";
/// O sistema só diz quando o app envolvido está aberto (a Automação do Spotify e do Música).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const UNKNOWN: &str = "unknown";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub id: &'static str,
    pub status: &'static str,
}

#[tauri::command]
pub async fn permissions() -> Result<Vec<Permission>, String> {
    tauri::async_runtime::spawn_blocking(backend::list).await.err_str()
}

#[tauri::command]
pub fn permission_action(app: AppHandle, id: String) -> Result<(), String> {
    backend::action(&app, &id)
}

#[cfg(target_os = "macos")]
mod backend {
    use std::ffi::c_void;
    use std::sync::Mutex;

    use objc2::runtime::AnyClass;
    use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
    use objc2_core_location::{CLAuthorizationStatus, CLLocationManager};
    use tauri::AppHandle;

    use super::{Permission, DENIED, GRANTED, NOT_DETERMINED, UNKNOWN};
    use crate::platform::macos::ax;
    use crate::util::spawn_detached;

    const PANE: &str = "x-apple.systempreferences:com.apple.preference.security?";
    /// Os players que o widget de mídia controla por AppleScript.
    const PLAYERS: [&str; 2] = ["com.spotify.client", "com.apple.Music"];

    #[repr(C)]
    struct AEDesc {
        descriptor_type: u32,
        data_handle: *mut c_void,
    }

    const TYPE_APPLICATION_BUNDLE_ID: u32 = u32::from_be_bytes(*b"bund");
    const TYPE_WILDCARD: u32 = u32::from_be_bytes(*b"****");
    const ERR_NOT_PERMITTED: i32 = -1743;
    const ERR_NEEDS_CONSENT: i32 = -1744;

    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        fn AECreateDesc(type_code: u32, data: *const c_void, size: isize, result: *mut AEDesc) -> i16;
        fn AEDisposeDesc(desc: *mut AEDesc) -> i16;
        fn AEDeterminePermissionToAutomateTarget(target: *const AEDesc, event_class: u32, event_id: u32, ask_user: u8) -> i32;
    }

    // Só para o `CBManager` existir no runtime do Objective-C; o Pier usa o IOBluetooth.
    #[link(name = "CoreBluetooth", kind = "framework")]
    extern "C" {}

    /// Ids que já tiveram o pedido do sistema nesta execução: na segunda vez o botão abre os
    /// Ajustes, porque o sistema não mostra o pedido de novo.
    static ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn automation(bundle_id: &str) -> i32 {
        let mut target = AEDesc { descriptor_type: 0, data_handle: std::ptr::null_mut() };
        unsafe {
            if AECreateDesc(TYPE_APPLICATION_BUNDLE_ID, bundle_id.as_ptr().cast(), bundle_id.len() as isize, &mut target) != 0 {
                return 0;
            }
            let status = AEDeterminePermissionToAutomateTarget(&target, TYPE_WILDCARD, TYPE_WILDCARD, 0);
            AEDisposeDesc(&mut target);
            status
        }
    }

    /// Concedida para um dos players basta; com os dois fechados o sistema não responde (procNotFound).
    fn automation_status() -> &'static str {
        let results: Vec<i32> = PLAYERS.iter().map(|b| automation(b)).collect();
        if results.contains(&0) {
            GRANTED
        } else if results.contains(&ERR_NOT_PERMITTED) {
            DENIED
        } else if results.contains(&ERR_NEEDS_CONSENT) {
            NOT_DETERMINED
        } else {
            UNKNOWN
        }
    }

    fn location_status() -> &'static str {
        if !unsafe { CLLocationManager::locationServicesEnabled_class() } {
            return DENIED;
        }
        match unsafe { CLLocationManager::new().authorizationStatus() } {
            CLAuthorizationStatus::NotDetermined => NOT_DETERMINED,
            CLAuthorizationStatus::Denied | CLAuthorizationStatus::Restricted => DENIED,
            _ => GRANTED,
        }
    }

    /// `CBManager.authorization` (macOS 10.15+): 0 não perguntado, 1 restrito, 2 negado, 3 permitido.
    fn bluetooth_status() -> &'static str {
        let Some(class) = AnyClass::get(c"CBManager") else { return UNKNOWN };
        let value: isize = unsafe { objc2::msg_send![class, authorization] };
        match value {
            0 => NOT_DETERMINED,
            3 => GRANTED,
            _ => DENIED,
        }
    }

    pub fn list() -> Vec<Permission> {
        // Acessibilidade e Gravação de Tela não distinguem "negada" de "nunca pedida".
        let pending = |granted: bool| if granted { GRANTED } else { NOT_DETERMINED };
        vec![
            Permission { id: "accessibility", status: pending(ax::is_trusted()) },
            Permission { id: "screen-recording", status: pending(CGPreflightScreenCaptureAccess()) },
            Permission { id: "automation", status: automation_status() },
            Permission { id: "location", status: location_status() },
            Permission { id: "bluetooth", status: bluetooth_status() },
        ]
    }

    fn open_pane(anchor: &str) -> Result<(), String> {
        spawn_detached("/usr/bin/open", &[&format!("{PANE}{anchor}")])
    }

    pub fn action(app: &AppHandle, id: &str) -> Result<(), String> {
        let first = {
            let mut asked = ASKED.lock().unwrap();
            let first = !asked.iter().any(|a| a == id);
            if first {
                asked.push(id.to_string());
            }
            first
        };
        match id {
            "accessibility" if first && !ax::is_trusted() => {
                ax::request_trust();
                Ok(())
            }
            "accessibility" => open_pane("Privacy_Accessibility"),
            "screen-recording" if first && !CGPreflightScreenCaptureAccess() => {
                CGRequestScreenCaptureAccess();
                Ok(())
            }
            "screen-recording" => open_pane("Privacy_ScreenCapture"),
            // O pedido da Localização (e a ida aos Ajustes quando já foi negada) é o do Wi-Fi.
            "location" => {
                crate::wifi::wifi_request_permission(app.clone());
                Ok(())
            }
            "automation" => open_pane("Privacy_Automation"),
            "bluetooth" => open_pane("Privacy_Bluetooth"),
            _ => Err(format!("permissão desconhecida: {id}")),
        }
    }
}

#[cfg(windows)]
mod backend {
    use tauri::AppHandle;

    use super::{Permission, DENIED, GRANTED};

    /// O Windows 11 24H2 só mostra os nomes das redes Wi-Fi para apps com acesso à Localização.
    pub fn list() -> Vec<Permission> {
        let status = if crate::wifi::needs_location() { DENIED } else { GRANTED };
        vec![Permission { id: "location", status }]
    }

    pub fn action(app: &AppHandle, id: &str) -> Result<(), String> {
        match id {
            "location" => {
                crate::wifi::wifi_request_permission(app.clone());
                Ok(())
            }
            _ => Err(format!("permissão desconhecida: {id}")),
        }
    }
}

/// No Linux o que falta (a extensão do GNOME) já aparece no aviso das configurações.
#[cfg(target_os = "linux")]
mod backend {
    use tauri::AppHandle;

    use super::Permission;

    pub fn list() -> Vec<Permission> {
        Vec::new()
    }

    pub fn action(_app: &AppHandle, id: &str) -> Result<(), String> {
        Err(format!("permissão desconhecida: {id}"))
    }
}
