//! Saídas e volume por app pelo Core Audio: os endpoints de render ativos e as sessões de áudio da
//! saída padrão (uma por processo, como no Mixer de Volume).

use std::ffi::c_void;

use windows::core::{Interface, HRESULT, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eConsole, eRender, AudioSessionStateExpired, IAudioSessionControl2, IAudioSessionManager2, IMMDevice,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};

use super::{AppStream, OutputDevice};
use crate::util::{com_init, ErrStr};

/// Interface não documentada que o próprio Windows usa para trocar o dispositivo padrão (não há
/// API pública para isso). Só a ordem da vtable importa; os métodos antes de `SetDefaultEndpoint`
/// não são usados.
#[allow(non_snake_case)]
mod policy {
    use std::ffi::c_void;

    use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR};
    use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia, ERole};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    pub unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self, device: PCWSTR, format: *mut *mut c_void) -> HRESULT;
        fn GetDeviceFormat(&self, device: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
        fn ResetDeviceFormat(&self, device: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(&self, device: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
        fn GetProcessingPeriod(&self, device: PCWSTR, default: i32, default_period: *mut i64, min_period: *mut i64) -> HRESULT;
        fn SetProcessingPeriod(&self, device: PCWSTR, period: *mut i64) -> HRESULT;
        fn GetShareMode(&self, device: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn SetShareMode(&self, device: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn GetPropertyValue(&self, device: PCWSTR, store: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetPropertyValue(&self, device: PCWSTR, store: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetDefaultEndpoint(&self, device: PCWSTR, role: ERole) -> HRESULT;
        fn SetEndpointVisibility(&self, device: PCWSTR, visible: i32) -> HRESULT;
    }

    /// Torna `device` (id do endpoint, terminado em zero) a saída padrão nos três papéis.
    pub fn set_default_endpoint(device: &[u16]) -> windows::core::Result<()> {
        unsafe {
            let policy: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)?;
            for role in [eConsole, eMultimedia, eCommunications] {
                policy.SetDefaultEndpoint(PCWSTR(device.as_ptr()), role).ok()?;
            }
        }
        Ok(())
    }
}

fn enumerator() -> windows::core::Result<IMMDeviceEnumerator> {
    com_init();
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

/// Copia e libera uma string alocada pelo COM.
fn take_string(raw: PWSTR) -> String {
    if raw.is_null() {
        return String::new();
    }
    let text = unsafe { raw.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(raw.0 as *const c_void)) };
    text
}

fn device_id(device: &IMMDevice) -> windows::core::Result<String> {
    Ok(take_string(unsafe { device.GetId()? }))
}

fn friendly_name(device: &IMMDevice) -> windows::core::Result<String> {
    unsafe { Ok(device.OpenPropertyStore(STGM_READ)?.GetValue(&PKEY_Device_FriendlyName)?.to_string()) }
}

pub fn outputs() -> Result<Vec<OutputDevice>, String> {
    (|| -> windows::core::Result<Vec<OutputDevice>> {
        let enumerator = enumerator()?;
        let default = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }.ok().and_then(|d| device_id(&d).ok());
        let devices = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };
        let mut outputs = Vec::new();
        for i in 0..unsafe { devices.GetCount()? } {
            let device = unsafe { devices.Item(i)? };
            let id = device_id(&device)?;
            let name = friendly_name(&device).unwrap_or_else(|_| id.clone());
            outputs.push(OutputDevice { default: default.as_deref() == Some(id.as_str()), id, name });
        }
        Ok(outputs)
    })()
    .err_str()
}

pub fn set_output(id: &str) -> Result<(), String> {
    com_init();
    let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
    policy::set_default_endpoint(&wide).err_str()
}

/// Sessões da saída padrão, menos as expiradas e a dos sons do sistema, com o pid de cada uma.
fn sessions() -> windows::core::Result<Vec<(u32, IAudioSessionControl2)>> {
    let device = unsafe { enumerator()?.GetDefaultAudioEndpoint(eRender, eConsole)? };
    let manager: IAudioSessionManager2 = unsafe { device.Activate(CLSCTX_ALL, None)? };
    let list = unsafe { manager.GetSessionEnumerator()? };
    let mut sessions = Vec::new();
    for i in 0..unsafe { list.GetCount()? } {
        let control: IAudioSessionControl2 = unsafe { list.GetSession(i)? }.cast()?;
        let expired = unsafe { control.GetState() }.map_or(true, |s| s == AudioSessionStateExpired);
        // S_OK quer dizer que é a sessão dos sons do sistema.
        let system = unsafe { control.IsSystemSoundsSession() } == HRESULT(0);
        match unsafe { control.GetProcessId() } {
            Ok(pid) if pid != 0 && !expired && !system => sessions.push((pid, control)),
            _ => {}
        }
    }
    Ok(sessions)
}

pub fn apps() -> Option<Vec<AppStream>> {
    let mut apps: Vec<AppStream> = Vec::new();
    for (pid, control) in sessions().ok()? {
        let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue };
        let level = unsafe { volume.GetMasterVolume() }.unwrap_or(1.0);
        let muted = unsafe { volume.GetMute() }.is_ok_and(|m| m.as_bool());
        let id = pid.to_string();
        // Um app pode ter várias sessões (uma por dispositivo lógico, por exemplo).
        if let Some(app) = apps.iter_mut().find(|a| a.id == id) {
            app.level = app.level.max(level);
            app.muted &= muted;
            continue;
        }
        let exe = crate::windows_list::process_path(pid);
        let display = take_string(unsafe { control.GetDisplayName() }.unwrap_or(PWSTR::null()));
        let process = std::path::Path::new(&exe).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        // O nome de exibição costuma vir vazio ou como recurso ("@%SystemRoot%\…").
        let name = if !display.is_empty() && !display.starts_with('@') { display } else { crate::usage::app_name(&process, None) };
        if name.is_empty() {
            continue;
        }
        apps.push(AppStream { id, name, icon: (!exe.is_empty()).then_some(exe), level, muted });
    }
    Some(apps)
}

pub fn set_app_volume(id: &str, level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    let pid: u32 = id.parse().map_err(|_| format!("app inválido: {id}"))?;
    for (session_pid, control) in sessions().err_str()? {
        if session_pid != pid {
            continue;
        }
        let volume = control.cast::<ISimpleAudioVolume>().err_str()?;
        unsafe {
            if let Some(level) = level {
                volume.SetMasterVolume(level, std::ptr::null()).err_str()?;
            }
            if let Some(muted) = muted {
                volume.SetMute(muted, std::ptr::null()).err_str()?;
            }
        }
    }
    Ok(())
}
