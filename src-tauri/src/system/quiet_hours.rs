//! "Não incomodar" do Windows 11 (Focus Assist no Windows 10). Não há API pública: usa o objeto COM
//! `QuietHoursSettings` que o próprio centro de notificações usa. A definição vem do IDL de Rafael
//! Rivera (MIT): https://gist.github.com/riverar/085d98ffb1343e92225a10817109b2e3
// Os nomes dos métodos COM seguem o IDL (`get_UserSelectedProfile`).
#![allow(non_snake_case)]

use std::ffi::c_void;

use windows::core::{interface, HSTRING, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_LOCAL_SERVER};

use crate::util::{com_init, ErrStr};

const CLSID_QUIET_HOURS_SETTINGS: GUID = GUID::from_u128(0xf53321fa_34f8_4b7f_b9a3_361877cb94cf);

/// Perfil que silencia tudo menos os apps e contatos prioritários: o "Não incomodar" ligado.
const PRIORITY_ONLY: &str = "Microsoft.QuietHoursProfile.PriorityOnly";

/// Só a ordem da vtable importa. Os métodos entre `put_UserSelectedProfile` e `get_OffProfileId`
/// não são usados.
#[interface("6bff4732-81ec-4ffb-ae67-b6c1bc29631f")]
unsafe trait IQuietHoursSettings: IUnknown {
    fn get_UserSelectedProfile(&self, profile_id: *mut PWSTR) -> HRESULT;
    fn put_UserSelectedProfile(&self, profile_id: PCWSTR) -> HRESULT;
    fn GetProfile(&self, profile_id: PCWSTR, profile: *mut *mut c_void) -> HRESULT;
    fn GetAllProfileData(&self, count: *mut u32, data: *mut c_void) -> HRESULT;
    fn GetDisplayNameForProfile(&self, profile_id: PCWSTR, name: *mut PWSTR) -> HRESULT;
    fn get_QuietMomentsManager(&self, manager: *mut *mut c_void) -> HRESULT;
    fn get_OffProfileId(&self, profile_id: *mut PWSTR) -> HRESULT;
}

fn settings() -> windows::core::Result<IQuietHoursSettings> {
    com_init();
    unsafe { CoCreateInstance(&CLSID_QUIET_HOURS_SETTINGS, None, CLSCTX_LOCAL_SERVER) }
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

fn selected_profile(settings: &IQuietHoursSettings) -> windows::core::Result<String> {
    let mut raw = PWSTR::null();
    unsafe { settings.get_UserSelectedProfile(&mut raw).ok()? };
    Ok(take_string(raw))
}

fn off_profile(settings: &IQuietHoursSettings) -> windows::core::Result<String> {
    let mut raw = PWSTR::null();
    unsafe { settings.get_OffProfileId(&mut raw).ok()? };
    Ok(take_string(raw))
}

fn select_profile(settings: &IQuietHoursSettings, profile: &str) -> windows::core::Result<()> {
    let profile = HSTRING::from(profile);
    unsafe { settings.put_UserSelectedProfile(PCWSTR(profile.as_ptr())).ok() }
}

/// Qualquer perfil diferente do "desligado" silencia notificações, inclusive "só alarmes".
fn is_on(selected: &str, off: &str) -> bool {
    !selected.eq_ignore_ascii_case(off)
}

/// `true` com o "Não incomodar" ligado.
pub fn status() -> Result<bool, String> {
    (|| -> windows::core::Result<bool> {
        let settings = settings()?;
        Ok(is_on(&selected_profile(&settings)?, &off_profile(&settings)?))
    })()
    .err_str()
}

pub fn set(enabled: bool) -> Result<(), String> {
    (|| -> windows::core::Result<()> {
        let settings = settings()?;
        if enabled {
            // Já ligado em outro perfil (ex.: só alarmes): preserva a escolha da pessoa.
            if is_on(&selected_profile(&settings)?, &off_profile(&settings)?) {
                return Ok(());
            }
            select_profile(&settings, PRIORITY_ONLY)
        } else {
            let off = off_profile(&settings)?;
            select_profile(&settings, &off)
        }
    })()
    .err_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_off_profile_means_disabled() {
        let off = "Microsoft.QuietHoursProfile.Unrestricted";
        assert!(!is_on(off, off));
        assert!(!is_on("microsoft.quiethoursprofile.unrestricted", off));
        assert!(is_on(PRIORITY_ONLY, off));
        assert!(is_on("Microsoft.QuietHoursProfile.AlarmsOnly", off));
    }

    /// Liga, confere, desliga, confere e devolve o estado original. Conferir também no botão do
    /// centro de notificações (Win+N) enquanto roda com `--nocapture`.
    #[test]
    #[ignore = "smoke test: muda o Não incomodar do sistema"]
    fn toggles_quiet_hours() {
        let settings = settings().expect("QuietHoursSettings indisponível");
        let original = selected_profile(&settings).unwrap();
        let off = off_profile(&settings).unwrap();
        println!("perfil atual: {original:?}, desligado: {off:?}");

        set(true).unwrap();
        println!("ligado: {:?}", selected_profile(&settings).unwrap());
        assert!(status().unwrap(), "deveria estar ligado");
        std::thread::sleep(std::time::Duration::from_secs(3));

        set(false).unwrap();
        println!("desligado: {:?}", selected_profile(&settings).unwrap());
        assert!(!status().unwrap(), "deveria estar desligado");

        select_profile(&settings, &original).unwrap();
    }
}
