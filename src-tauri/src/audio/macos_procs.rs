//! Processos que o CoreAudio conhece (`kAudioHardwarePropertyProcessObjectList`, macOS 14.2+),
//! agrupados pelo app regular a que pertencem.

use std::ffi::{c_char, c_void, CStr};
use std::sync::OnceLock;

use objc2_core_audio::{
    kAudioHardwarePropertyProcessObjectList, kAudioProcessPropertyBundleID, kAudioProcessPropertyIsRunningOutput, kAudioProcessPropertyPID,
    AudioObjectID,
};

use super::{get, get_ids, get_string, global, SYSTEM};
use crate::platform::macos::workspace::{self, RunningApp};

/// Um app regular com os objetos de processo do CoreAudio que tocam por ele (o próprio app e os
/// helpers, como o processo de GPU do WebKit ou o de áudio do Chrome).
#[derive(Debug)]
pub struct AppGroup {
    pub app: RunningApp,
    pub objects: Vec<AudioObjectID>,
    pub playing: bool,
}

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

/// Símbolo resolvido em tempo de execução: os que só existem em versões novas do macOS não podem
/// ser ligados direto, ou o dyld recusaria abrir o Pier nas antigas.
pub fn symbol(name: &CStr) -> Option<*mut c_void> {
    let ptr = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) };
    (!ptr.is_null()).then_some(ptr)
}

/// O pid "responsável" por um processo (privada, da libsystem): é o que liga o processo de GPU do
/// WebKit ao Safari e os helpers do Chrome ao Chrome.
fn responsible_pid(pid: i32) -> Option<i32> {
    type Responsible = unsafe extern "C" fn(i32) -> i32;
    static FN: OnceLock<Option<Responsible>> = OnceLock::new();
    let f = (*FN.get_or_init(|| symbol(c"responsibility_get_pid_responsible_for_pid").map(|p| unsafe { std::mem::transmute::<*mut c_void, Responsible>(p) })))?;
    let responsible = unsafe { f(pid) };
    (responsible > 0).then_some(responsible)
}

/// Sem o pid responsável, um helper com bundle id derivado do app (`com.google.Chrome.helper`).
fn owner<'a>(pid: i32, bundle_id: Option<&str>, apps: &'a [RunningApp]) -> Option<&'a RunningApp> {
    let by_pid = |pid: i32| apps.iter().find(|a| a.pid == pid);
    by_pid(pid).or_else(|| responsible_pid(pid).and_then(by_pid)).or_else(|| {
        let bundle_id = bundle_id?;
        apps.iter().find(|a| {
            a.bundle_id
                .as_deref()
                .is_some_and(|app| bundle_id.strip_prefix(app).is_some_and(|rest| rest.starts_with('.')))
        })
    })
}

/// Os apps com algum processo no CoreAudio. Processos sem um app regular por trás (serviços do
/// sistema, o próprio Pier) ficam de fora.
pub fn groups() -> Vec<AppGroup> {
    let apps = workspace::running_apps();
    let mut groups: Vec<AppGroup> = Vec::new();
    for object in get_ids(SYSTEM, global(kAudioHardwarePropertyProcessObjectList)) {
        let Some(pid) = get::<i32>(object, global(kAudioProcessPropertyPID)) else { continue };
        let bundle_id = get_string(object, kAudioProcessPropertyBundleID);
        let Some(app) = owner(pid, bundle_id.as_deref(), &apps) else { continue };
        let Some(key) = app.bundle_id.clone() else { continue };
        let playing = get::<u32>(object, global(kAudioProcessPropertyIsRunningOutput)).unwrap_or(0) != 0;
        match groups.iter_mut().find(|g| g.app.bundle_id.as_deref() == Some(key.as_str())) {
            Some(group) => {
                group.objects.push(object);
                group.playing |= playing;
            }
            None => groups.push(AppGroup { app: app.clone(), objects: vec![object], playing }),
        }
    }
    groups
}
