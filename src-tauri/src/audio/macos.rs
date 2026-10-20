//! Saídas de áudio pelo CoreAudio e o volume de cada app pelos process taps (macOS 14.2+). Antes
//! do 14.2 não há API para o volume de cada app, e o mixer fica escondido.

#[path = "macos_procs.rs"]
mod procs;
#[path = "macos_tap.rs"]
mod tap;

use std::ffi::c_void;
use std::ptr::NonNull;

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use objc2_core_audio::{
    kAudioDevicePropertyDeviceUID, kAudioDevicePropertyStreams, kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeOutput,
    kAudioObjectSystemObject, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectSetPropertyData,
};

use super::{AppStream, OutputDevice};

const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

fn address(selector: u32, scope: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress { mSelector: selector, mScope: scope, mElement: kAudioObjectPropertyElementMain }
}

fn global(selector: u32) -> AudioObjectPropertyAddress {
    address(selector, kAudioObjectPropertyScopeGlobal)
}

fn data_size(object: AudioObjectID, mut address: AudioObjectPropertyAddress) -> Option<u32> {
    let mut size = 0u32;
    let status = unsafe { AudioObjectGetPropertyDataSize(object, NonNull::from(&mut address), 0, std::ptr::null(), NonNull::from(&mut size)) };
    (status == 0).then_some(size)
}

/// Lê uma propriedade de tamanho fixo (um id, um ponteiro de CFString).
fn get<T: Default>(object: AudioObjectID, mut address: AudioObjectPropertyAddress) -> Option<T> {
    let mut value = T::default();
    let mut size = std::mem::size_of::<T>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(object, NonNull::from(&mut address), 0, std::ptr::null(), NonNull::from(&mut size), NonNull::from(&mut value).cast())
    };
    (status == 0).then_some(value)
}

/// Lê uma propriedade que é uma lista de ids (dispositivos, processos).
fn get_ids(object: AudioObjectID, mut address: AudioObjectPropertyAddress) -> Vec<AudioObjectID> {
    let Some(size) = data_size(object, address) else { return Vec::new() };
    let mut ids = vec![0 as AudioObjectID; size as usize / std::mem::size_of::<AudioObjectID>()];
    if ids.is_empty() {
        return ids;
    }
    let mut size = (ids.len() * std::mem::size_of::<AudioObjectID>()) as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(object, NonNull::from(&mut address), 0, std::ptr::null(), NonNull::from(&mut size), NonNull::new_unchecked(ids.as_mut_ptr().cast()))
    };
    if status != 0 {
        return Vec::new();
    }
    ids.truncate(size as usize / std::mem::size_of::<AudioObjectID>());
    ids
}

/// Lê uma propriedade CFString (nome, UID, bundle id).
fn get_string(object: AudioObjectID, selector: u32) -> Option<String> {
    let raw: usize = get(object, global(selector))?;
    if raw == 0 {
        return None;
    }
    // O CoreAudio entrega a string com +1 de referência (regra de "create").
    let string = unsafe { CFString::wrap_under_create_rule(raw as CFStringRef) };
    Some(string.to_string())
}

fn has_output(device: AudioObjectID) -> bool {
    data_size(device, address(kAudioDevicePropertyStreams, kAudioObjectPropertyScopeOutput)).is_some_and(|size| size > 0)
}

fn default_output() -> Option<AudioObjectID> {
    get(SYSTEM, global(kAudioHardwarePropertyDefaultOutputDevice))
}

fn device_uid(device: AudioObjectID) -> Option<String> {
    get_string(device, kAudioDevicePropertyDeviceUID)
}

pub fn outputs() -> Result<Vec<OutputDevice>, String> {
    let current = default_output();
    Ok(get_ids(SYSTEM, global(kAudioHardwarePropertyDevices))
        .into_iter()
        .filter(|&id| has_output(id) && !device_uid(id).is_some_and(|uid| tap::is_own_device(&uid)))
        .filter_map(|id| Some(OutputDevice { id: id.to_string(), name: get_string(id, kAudioObjectPropertyName)?, default: Some(id) == current }))
        .collect())
}

pub fn set_output(id: &str) -> Result<(), String> {
    let mut device: AudioObjectID = id.parse().map_err(|_| format!("saída inválida: {id}"))?;
    let mut addr = global(kAudioHardwarePropertyDefaultOutputDevice);
    let size = std::mem::size_of::<AudioObjectID>() as u32;
    let status = unsafe {
        AudioObjectSetPropertyData(SYSTEM, NonNull::from(&mut addr), 0, std::ptr::null(), size, NonNull::from(&mut device).cast::<c_void>())
    };
    if status == 0 {
        Ok(())
    } else {
        Err(format!("falha ao trocar a saída de áudio (OSStatus {status})"))
    }
}

pub fn apps() -> Option<Vec<AppStream>> {
    tap::supported().then(tap::apps)
}

pub fn set_app_volume(id: &str, level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    if !tap::supported() {
        return Err("o volume de cada app precisa do macOS 14.2 ou mais novo".into());
    }
    tap::set_volume(id, level, muted);
    Ok(())
}

pub fn start() {
    if tap::supported() {
        tap::start();
    }
}
