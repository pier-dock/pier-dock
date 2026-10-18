//! Saídas de áudio pelo CoreAudio. O macOS não tem API pública para o volume de cada app, então o
//! mixer fica escondido.

use std::ffi::c_void;
use std::ptr::NonNull;

use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use objc2_core_audio::{
    kAudioDevicePropertyStreams, kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID, AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};

use super::{AppStream, OutputDevice};

const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

fn address(selector: u32, scope: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress { mSelector: selector, mScope: scope, mElement: kAudioObjectPropertyElementMain }
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

fn device_ids() -> Vec<AudioObjectID> {
    let mut addr = address(kAudioHardwarePropertyDevices, kAudioObjectPropertyScopeGlobal);
    let Some(size) = data_size(SYSTEM, addr) else { return Vec::new() };
    let mut ids = vec![0 as AudioObjectID; size as usize / std::mem::size_of::<AudioObjectID>()];
    let mut size = (ids.len() * std::mem::size_of::<AudioObjectID>()) as u32;
    if ids.is_empty() {
        return ids;
    }
    let status = unsafe {
        AudioObjectGetPropertyData(SYSTEM, NonNull::from(&mut addr), 0, std::ptr::null(), NonNull::from(&mut size), NonNull::new_unchecked(ids.as_mut_ptr().cast()))
    };
    if status != 0 {
        return Vec::new();
    }
    ids.truncate(size as usize / std::mem::size_of::<AudioObjectID>());
    ids
}

fn has_output(device: AudioObjectID) -> bool {
    data_size(device, address(kAudioDevicePropertyStreams, kAudioObjectPropertyScopeOutput)).is_some_and(|size| size > 0)
}

fn name(device: AudioObjectID) -> Option<String> {
    let raw: usize = get(device, address(kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal))?;
    if raw == 0 {
        return None;
    }
    // O CoreAudio entrega a string com +1 de referência (regra de "create").
    let name = unsafe { CFString::wrap_under_create_rule(raw as CFStringRef) };
    Some(name.to_string())
}

fn default_output() -> Option<AudioObjectID> {
    get(SYSTEM, address(kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyScopeGlobal))
}

pub fn outputs() -> Result<Vec<OutputDevice>, String> {
    let current = default_output();
    Ok(device_ids()
        .into_iter()
        .filter(|&id| has_output(id))
        .filter_map(|id| Some(OutputDevice { id: id.to_string(), name: name(id)?, default: Some(id) == current }))
        .collect())
}

pub fn set_output(id: &str) -> Result<(), String> {
    let mut device: AudioObjectID = id.parse().map_err(|_| format!("saída inválida: {id}"))?;
    let mut addr = address(kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyScopeGlobal);
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
    None
}

pub fn set_app_volume(_id: &str, _level: Option<f32>, _muted: Option<bool>) -> Result<(), String> {
    Err("o macOS não permite ajustar o volume de cada app".into())
}
