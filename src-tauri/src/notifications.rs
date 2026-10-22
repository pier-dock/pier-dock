//! Contagem de notificações por app, lida da Central de Notificações do Windows
//! (`UserNotificationListener`). Os toasts vêm com o AUMID e o nome do app que os enviou; a
//! janela é casada pelo AUMID dela, pelo do processo (apps da Store) ou pelo do atalho do Menu
//! Iniciar do mesmo exe (apps Win32 como Discord e Steam, cujo AUMID não aparece na janela).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus};
use windows::UI::Notifications::NotificationKinds;

use crate::windows_list::mark_dirty;

/// Contagens por AUMID (minúsculo) e por nome de exibição (`name:` + minúsculo).
static COUNTS: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);
/// Exe (minúsculo) → AUMID e nome do atalho do Menu Iniciar correspondente.
static START_MENU: Mutex<Option<HashMap<String, (String, String)>>> = Mutex::new(None);

fn read_counts() -> windows::core::Result<HashMap<String, u32>> {
    let listener = UserNotificationListener::Current()?;
    let mut status = listener.GetAccessStatus()?;
    if status == UserNotificationListenerAccessStatus::Unspecified {
        status = listener.RequestAccessAsync()?.get()?;
    }
    let mut counts = HashMap::new();
    if status != UserNotificationListenerAccessStatus::Allowed {
        return Ok(counts);
    }
    for notification in &listener.GetNotificationsAsync(NotificationKinds::Toast)?.get()? {
        let Ok(info) = notification.AppInfo() else { continue };
        if let Ok(id) = info.AppUserModelId() {
            *counts.entry(id.to_string().to_lowercase()).or_insert(0) += 1;
        }
        if let Ok(name) = info.DisplayInfo().and_then(|d| d.DisplayName()) {
            *counts.entry(format!("name:{}", name.to_string().to_lowercase())).or_insert(0) += 1;
        }
    }
    Ok(counts)
}

pub fn start() {
    std::thread::spawn(|| {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        loop {
            let counts = read_counts().unwrap_or_default();
            let mut current = COUNTS.lock().unwrap();
            if current.as_ref() != Some(&counts) {
                *current = Some(counts);
                drop(current);
                mark_dirty();
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    std::thread::spawn(|| loop {
        if let Ok(apps) = crate::apps::installed_apps() {
            let map = apps
                .into_iter()
                .filter_map(|a| Some((a.exe?.to_lowercase(), (a.aumid?.to_lowercase(), a.name.to_lowercase()))))
                .collect();
            *START_MENU.lock().unwrap() = Some(map);
        }
        std::thread::sleep(Duration::from_secs(60));
    });
}

fn process_aumid(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 130];
        let mut len = buf.len() as u32;
        let err = GetApplicationUserModelId(handle, &mut len, PWSTR(buf.as_mut_ptr()));
        let _ = CloseHandle(handle);
        (err.0 == 0 && len > 1).then(|| String::from_utf16_lossy(&buf[..len as usize - 1]).to_lowercase())
    }
}

/// Notificações pendentes do app dono da janela; 0 quando nenhuma fonte casa.
pub fn badge_for(exe: &str, window_aumid: Option<&str>, pid: u32) -> u32 {
    let guard = COUNTS.lock().unwrap();
    let Some(counts) = guard.as_ref().filter(|c| !c.is_empty()) else { return 0 };
    let mut keys: Vec<String> = window_aumid.map(str::to_lowercase).into_iter().collect();
    keys.extend(process_aumid(pid));
    if let Some((aumid, name)) = START_MENU.lock().unwrap().as_ref().and_then(|m| m.get(&exe.to_lowercase())) {
        keys.push(aumid.clone());
        keys.push(format!("name:{name}"));
    }
    keys.iter().filter_map(|k| counts.get(k)).copied().max().unwrap_or(0)
}
