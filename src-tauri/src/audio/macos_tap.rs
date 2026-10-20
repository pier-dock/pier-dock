//! Volume de cada app pelos process taps do CoreAudio (macOS 14.2+).
//!
//! Para um app fora dos 100%, o Pier cria um tap privado dos processos dele com
//! `MutedWhenTapped` (o sistema cala a saída original enquanto o tap é lido) e um aggregate device
//! privado que junta esse tap à saída padrão. O IOProc do aggregate copia o áudio do tap para a
//! saída, multiplicado pelo ganho. De volta aos 100% (e sem mudo), o tap é desfeito e o app volta
//! ao caminho normal. Se o Pier fechar ou travar, o sistema desfaz os taps junto com o processo.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant};

use block2::{Block, RcBlock};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use objc2::runtime::{AnyClass, Bool};
use objc2::AllocAnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceMainSubDeviceKey, kAudioAggregateDeviceNameKey, kAudioAggregateDeviceSubDeviceListKey,
    kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioHardwarePropertyProcessObjectList, kAudioSubDeviceUIDKey, kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey,
    AudioDeviceCreateIOProcID, AudioDeviceDestroyIOProcID, AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop,
    AudioHardwareDestroyAggregateDevice, AudioObjectAddPropertyListener, AudioObjectID, AudioObjectPropertyAddress,
    CATapDescription, CATapMuteBehavior,
};
use objc2_core_audio_types::{AudioBuffer, AudioBufferList, AudioTimeStamp};
use objc2_foundation::{NSArray, NSNumber, NSUUID};
use serde::{Deserialize, Serialize};

use super::procs::{self, symbol, AppGroup};
use super::{default_output, device_uid, global, AppStream, SYSTEM};

/// Prefixo do UID dos aggregates do Pier, para não aparecerem na lista de saídas.
const DEVICE_PREFIX: &str = "dev.pier.app.tap.";
/// Reserva para o que os listeners não avisam (um app que começa ou para de tocar).
const TICK: Duration = Duration::from_secs(1);
/// Um app parado há tanto tempo perde o tap, para o dispositivo de saída não ficar acordado à toa.
/// Ao voltar a tocar, o tap volta em até um `TICK`.
const IDLE: Duration = Duration::from_secs(5 * 60);
/// Depois de uma falha ao criar o tap de um app, quanto esperar para tentar de novo.
const RETRY: Duration = Duration::from_secs(30);

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    // A mesma da objc2-core-audio, mas com o dicionário do core-foundation que o resto do Pier usa.
    fn AudioHardwareCreateAggregateDevice(description: CFDictionaryRef, out_device: *mut AudioObjectID) -> i32;
}

extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const RTLD_NOW: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Volume {
    level: f32,
    muted: bool,
}

impl Default for Volume {
    fn default() -> Self {
        Self { level: 1.0, muted: false }
    }
}

impl Volume {
    fn gain(self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.level
        }
    }

    fn is_default(self) -> bool {
        !self.muted && self.level >= 0.999
    }
}

/// Ganho lido pelo IOProc, na thread de tempo real do áudio: só atômicos, nada de trava.
struct Gain {
    target: AtomicU32,
    current: AtomicU32,
}

impl Gain {
    fn new(value: f32) -> Box<Self> {
        Box::new(Self { target: AtomicU32::new(value.to_bits()), current: AtomicU32::new(value.to_bits()) })
    }

    fn set(&self, value: f32) {
        self.target.store(value.to_bits(), Ordering::Relaxed);
    }
}

struct AppTap {
    tap: AudioObjectID,
    aggregate: AudioObjectID,
    io_proc: AudioDeviceIOProcID,
    /// Em caixa para o endereço passado ao IOProc não mudar.
    gain: Box<Gain>,
    objects: Vec<AudioObjectID>,
    output: String,
    idle_since: Option<Instant>,
}

impl Drop for AppTap {
    fn drop(&mut self) {
        unsafe {
            if self.io_proc.is_some() {
                AudioDeviceStop(self.aggregate, self.io_proc);
                AudioDeviceDestroyIOProcID(self.aggregate, self.io_proc);
            }
            if self.aggregate != 0 {
                AudioHardwareDestroyAggregateDevice(self.aggregate);
            }
            if let Some(fns) = tap_fns() {
                (fns.destroy)(self.tap);
            }
        }
    }
}

#[derive(Default)]
struct State {
    volumes: HashMap<String, Volume>,
    taps: HashMap<String, AppTap>,
    failures: HashMap<String, Instant>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Mutex::default);
static WAKE: OnceLock<SyncSender<()>> = OnceLock::new();

type CreateTap = unsafe extern "C" fn(*const CATapDescription, *mut AudioObjectID) -> i32;
type DestroyTap = unsafe extern "C" fn(AudioObjectID) -> i32;

struct TapFns {
    create: CreateTap,
    destroy: DestroyTap,
}

fn tap_fns() -> Option<&'static TapFns> {
    static FNS: OnceLock<Option<TapFns>> = OnceLock::new();
    FNS.get_or_init(|| unsafe {
        Some(TapFns {
            create: std::mem::transmute::<*mut c_void, CreateTap>(symbol(c"AudioHardwareCreateProcessTap")?),
            destroy: std::mem::transmute::<*mut c_void, DestroyTap>(symbol(c"AudioHardwareDestroyProcessTap")?),
        })
    })
    .as_ref()
}

pub fn supported() -> bool {
    tap_fns().is_some() && AnyClass::get(c"CATapDescription").is_some()
}

pub fn is_own_device(uid: &str) -> bool {
    uid.starts_with(DEVICE_PREFIX)
}

type Preflight = unsafe extern "C" fn(CFStringRef, *const c_void) -> i32;
type Request = unsafe extern "C" fn(CFStringRef, *const c_void, *const Block<dyn Fn(Bool)>);

struct Tcc {
    preflight: Preflight,
    request: Request,
}

/// O macOS não tem API pública para consultar a permissão de Gravação de Áudio do Sistema; a do
/// TCC é a que o próprio sistema e apps como o AudioCap usam.
fn tcc() -> Option<&'static Tcc> {
    static TCC: OnceLock<Option<Tcc>> = OnceLock::new();
    TCC.get_or_init(|| unsafe {
        let handle = dlopen(c"/System/Library/PrivateFrameworks/TCC.framework/Versions/A/TCC".as_ptr(), RTLD_NOW);
        if handle.is_null() {
            log::warn!("TCC.framework indisponível: o volume de cada app fica desligado");
            return None;
        }
        let preflight = dlsym(handle, c"TCCAccessPreflight".as_ptr());
        let request = dlsym(handle, c"TCCAccessRequest".as_ptr());
        if preflight.is_null() || request.is_null() {
            log::warn!("TCCAccessPreflight/TCCAccessRequest indisponíveis: o volume de cada app fica desligado");
            return None;
        }
        Some(Tcc {
            preflight: std::mem::transmute::<*mut c_void, Preflight>(preflight),
            request: std::mem::transmute::<*mut c_void, Request>(request),
        })
    })
    .as_ref()
}

const AUDIO_CAPTURE: &str = "kTCCServiceAudioCapture";

/// 0: concedida; 1: negada; outro valor: ainda não perguntada.
fn preflight(tcc: &Tcc) -> i32 {
    let service = CFString::new(AUDIO_CAPTURE);
    unsafe { (tcc.preflight)(service.as_concrete_TypeRef(), std::ptr::null()) }
}

/// O usuário negou a permissão: só as Configurações do sistema liberam de novo.
pub fn permission_denied() -> bool {
    tcc().is_some_and(|tcc| preflight(tcc) == 1)
}

/// Sem a permissão o tap entrega silêncio e o app ficaria mudo, então nenhum tap é criado antes
/// dela. `request` mostra o pedido do sistema, uma vez por execução.
fn capture_allowed(request: bool) -> bool {
    static GRANTED: AtomicBool = AtomicBool::new(false);
    static ASKED: AtomicBool = AtomicBool::new(false);
    if GRANTED.load(Ordering::SeqCst) {
        return true;
    }
    let Some(tcc) = tcc() else { return false };
    if preflight(tcc) == 0 {
        GRANTED.store(true, Ordering::SeqCst);
        return true;
    }
    if request && !ASKED.swap(true, Ordering::SeqCst) {
        let done = RcBlock::new(|granted: Bool| {
            if granted.as_bool() {
                GRANTED.store(true, Ordering::SeqCst);
                wake();
            }
        });
        let service = CFString::new(AUDIO_CAPTURE);
        unsafe { (tcc.request)(service.as_concrete_TypeRef(), std::ptr::null(), &*done) };
    }
    false
}

fn wake() {
    if let Some(tx) = WAKE.get() {
        let _ = tx.try_send(());
    }
}

unsafe fn buffers<'a>(list: NonNull<AudioBufferList>) -> &'a [AudioBuffer] {
    let list = list.as_ptr();
    std::slice::from_raw_parts(std::ptr::addr_of!((*list).mBuffers).cast::<AudioBuffer>(), (*list).mNumberBuffers as usize)
}

/// Copia o tap (o último buffer de entrada: os streams de entrada da própria saída, se ela tiver,
/// vêm antes) para os canais de saída, com o ganho em rampa ao longo do ciclo para não estalar.
unsafe extern "C-unwind" fn io_proc(
    _device: AudioObjectID,
    _now: NonNull<AudioTimeStamp>,
    input: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>,
    output: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>,
    client: *mut c_void,
) -> i32 {
    let gain = &*(client as *const Gain);
    let target = f32::from_bits(gain.target.load(Ordering::Relaxed));
    let start = f32::from_bits(gain.current.load(Ordering::Relaxed));
    gain.current.store(target.to_bits(), Ordering::Relaxed);

    let (source, source_channels) = match buffers(input).last() {
        Some(b) if !b.mData.is_null() && b.mNumberChannels > 0 => {
            (std::slice::from_raw_parts(b.mData as *const f32, b.mDataByteSize as usize / 4), b.mNumberChannels as usize)
        }
        _ => (&[][..], 1),
    };
    let source_frames = source.len() / source_channels;

    let mut channel = 0;
    for buffer in buffers(output) {
        if buffer.mData.is_null() || buffer.mNumberChannels == 0 {
            continue;
        }
        let channels = buffer.mNumberChannels as usize;
        let dest = std::slice::from_raw_parts_mut(buffer.mData as *mut f32, buffer.mDataByteSize as usize / 4);
        let frames = dest.len() / channels;
        for frame in 0..frames {
            let g = start + (target - start) * frame as f32 / frames as f32;
            for c in 0..channels {
                let from = channel + c;
                dest[frame * channels + c] =
                    if frame < source_frames && from < source_channels { source[frame * source_channels + from] * g } else { 0.0 };
            }
        }
        channel += channels;
    }
    0
}

fn cf_key(key: &CStr) -> CFString {
    CFString::new(&key.to_string_lossy())
}

fn create(group: &AppGroup, output: &str, gain: f32) -> Result<AppTap, String> {
    let fns = tap_fns().ok_or("process taps indisponíveis")?;
    let objects: Vec<_> = group.objects.iter().map(|&o| NSNumber::new_u32(o)).collect();
    let uuid = NSUUID::new();
    let description = unsafe {
        let description = CATapDescription::initStereoMixdownOfProcesses(CATapDescription::alloc(), &NSArray::from_retained_slice(&objects));
        description.setPrivate(true);
        description.setMuteBehavior(CATapMuteBehavior::MutedWhenTapped);
        description.setUUID(&uuid);
        description
    };
    let mut tap = AppTap {
        tap: 0,
        aggregate: 0,
        io_proc: None,
        gain: Gain::new(gain),
        objects: group.objects.clone(),
        output: output.to_string(),
        idle_since: None,
    };
    let status = unsafe { (fns.create)(&*description, &mut tap.tap) };
    if status != 0 {
        // Sem tap criado, o `Drop` não tem o que desfazer.
        std::mem::forget(tap);
        return Err(format!("AudioHardwareCreateProcessTap falhou (OSStatus {status})"));
    }

    let tap_uid = uuid.UUIDString().to_string();
    let number = |n: i32| CFNumber::from(n).as_CFType();
    let sub_device: CFDictionary<CFString, CFType> =
        CFDictionary::from_CFType_pairs(&[(cf_key(kAudioSubDeviceUIDKey), CFString::new(output).as_CFType())]);
    let sub_tap: CFDictionary<CFString, CFType> = CFDictionary::from_CFType_pairs(&[
        (cf_key(kAudioSubTapUIDKey), CFString::new(&tap_uid).as_CFType()),
        (cf_key(kAudioSubTapDriftCompensationKey), number(1)),
    ]);
    let aggregate: CFDictionary<CFString, CFType> = CFDictionary::from_CFType_pairs(&[
        (cf_key(kAudioAggregateDeviceUIDKey), CFString::new(&format!("{DEVICE_PREFIX}{tap_uid}")).as_CFType()),
        (cf_key(kAudioAggregateDeviceNameKey), CFString::new(&format!("Pier · {}", group.app.name)).as_CFType()),
        (cf_key(kAudioAggregateDeviceIsPrivateKey), number(1)),
        (cf_key(kAudioAggregateDeviceMainSubDeviceKey), CFString::new(output).as_CFType()),
        (cf_key(kAudioAggregateDeviceSubDeviceListKey), CFArray::from_CFTypes(&[sub_device]).as_CFType()),
        (cf_key(kAudioAggregateDeviceTapListKey), CFArray::from_CFTypes(&[sub_tap]).as_CFType()),
        (cf_key(kAudioAggregateDeviceTapAutoStartKey), number(1)),
    ]);
    let status = unsafe { AudioHardwareCreateAggregateDevice(aggregate.as_concrete_TypeRef(), &mut tap.aggregate) };
    if status != 0 {
        return Err(format!("AudioHardwareCreateAggregateDevice falhou (OSStatus {status})"));
    }

    let client = &*tap.gain as *const Gain as *mut c_void;
    let status = unsafe { AudioDeviceCreateIOProcID(tap.aggregate, Some(io_proc), client, NonNull::from(&mut tap.io_proc)) };
    if status != 0 {
        return Err(format!("AudioDeviceCreateIOProcID falhou (OSStatus {status})"));
    }
    let status = unsafe { AudioDeviceStart(tap.aggregate, tap.io_proc) };
    if status != 0 {
        return Err(format!("AudioDeviceStart falhou (OSStatus {status})"));
    }
    Ok(tap)
}

fn key(group: &AppGroup) -> Option<&str> {
    group.app.bundle_id.as_deref()
}

/// Leva os taps ao estado pedido: um por app fora dos 100% que esteja tocando, na saída padrão
/// atual e com os processos atuais do app (um helper novo ou uma troca de saída recriam o tap).
fn reconcile(state: &mut State, request: bool) {
    if state.volumes.is_empty() && state.taps.is_empty() {
        return;
    }
    let groups = procs::groups();
    let output = default_output().and_then(device_uid);
    let State { volumes, taps, failures } = state;

    taps.retain(|bundle, tap| {
        let (Some(volume), Some(group)) = (volumes.get(bundle), groups.iter().find(|g| key(g) == Some(bundle.as_str()))) else {
            return false;
        };
        if output.as_deref() != Some(tap.output.as_str()) || tap.objects != group.objects {
            return false;
        }
        if group.playing {
            tap.idle_since = None;
        } else if tap.idle_since.get_or_insert_with(Instant::now).elapsed() > IDLE {
            return false;
        }
        tap.gain.set(volume.gain());
        true
    });

    let Some(output) = output else { return };
    let pending: Vec<(&AppGroup, &str, Volume)> = groups
        .iter()
        .filter(|g| g.playing)
        .filter_map(|g| {
            let bundle = key(g)?;
            let volume = *volumes.get(bundle)?;
            let waiting = taps.contains_key(bundle) || failures.get(bundle).is_some_and(|at| at.elapsed() < RETRY);
            (!waiting).then_some((g, bundle, volume))
        })
        .collect();
    // A consulta ao TCC é uma chamada a outro processo: só quando há um tap a criar.
    if pending.is_empty() || !capture_allowed(request) {
        return;
    }
    for (group, bundle, volume) in pending {
        match create(group, &output, volume.gain()) {
            Ok(tap) => {
                failures.remove(bundle);
                taps.insert(bundle.to_string(), tap);
            }
            Err(e) => {
                log::warn!("falha ao criar o tap de áudio de {bundle}: {e}");
                failures.insert(bundle.to_string(), Instant::now());
            }
        }
    }
}

/// Os apps tocando áudio agora, e também os que estão com tap (um app em pausa não some da lista
/// com o volume dele mexido).
pub fn apps() -> Vec<AppStream> {
    let groups = procs::groups();
    let state = STATE.lock().unwrap();
    let mut apps: Vec<AppStream> = groups
        .into_iter()
        .filter_map(|group| {
            let bundle = key(&group)?.to_string();
            if !group.playing && !state.taps.contains_key(&bundle) {
                return None;
            }
            let volume = state.volumes.get(&bundle).copied().unwrap_or_default();
            Some(AppStream { id: bundle, name: group.app.name, icon: group.app.path, level: volume.level, muted: volume.muted })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

pub fn set_volume(bundle: &str, level: Option<f32>, muted: Option<bool>) {
    let mut state = STATE.lock().unwrap();
    let mut volume = state.volumes.get(bundle).copied().unwrap_or_default();
    if let Some(level) = level {
        volume.level = level;
    }
    if let Some(muted) = muted {
        volume.muted = muted;
    }
    if volume.is_default() {
        state.volumes.remove(bundle);
    } else {
        state.volumes.insert(bundle.to_string(), volume);
    }
    save_volumes(&state.volumes);
    state.failures.remove(bundle);
    reconcile(&mut state, true);
}

/// Os volumes ficam num arquivo próprio, fora da config, para não disputar com o `save_config`
/// do frontend. Só os apps fora dos 100% entram; ao abrir o Pier, o tap volta quando o app tocar.
fn volumes_path() -> PathBuf {
    crate::util::data_dir().join("app-volumes.json")
}

fn load_volumes() -> HashMap<String, Volume> {
    std::fs::read_to_string(volumes_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_volumes(volumes: &HashMap<String, Volume>) {
    let result = serde_json::to_string_pretty(volumes).map_err(|e| e.to_string()).and_then(|json| std::fs::write(volumes_path(), json).map_err(|e| e.to_string()));
    if let Err(e) = result {
        log::warn!("falha ao gravar o volume dos apps: {e}");
    }
}

unsafe extern "C-unwind" fn on_change(_object: AudioObjectID, _count: u32, _addresses: NonNull<AudioObjectPropertyAddress>, _client: *mut c_void) -> i32 {
    wake();
    0
}

pub fn start() {
    let (tx, rx) = mpsc::sync_channel(1);
    if WAKE.set(tx).is_err() {
        return;
    }
    STATE.lock().unwrap().volumes = load_volumes();
    for selector in [kAudioHardwarePropertyProcessObjectList, kAudioHardwarePropertyDefaultOutputDevice] {
        let mut address = global(selector);
        unsafe { AudioObjectAddPropertyListener(SYSTEM, NonNull::from(&mut address), Some(on_change), std::ptr::null_mut()) };
    }
    std::thread::spawn(move || loop {
        let _ = rx.recv_timeout(TICK);
        reconcile(&mut STATE.lock().unwrap(), false);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_saved_volumes() {
        let volumes: HashMap<String, Volume> = serde_json::from_str(r#"{"com.spotify.client": {"level": 0.4, "muted": true}}"#).unwrap();
        let spotify = volumes["com.spotify.client"];
        assert_eq!(spotify, Volume { level: 0.4, muted: true });
        assert_eq!(spotify.gain(), 0.0);
    }

    #[test]
    fn full_volume_drops_the_tap() {
        assert!(Volume::default().is_default());
        assert!(!Volume { level: 1.0, muted: true }.is_default());
        assert!(!Volume { level: 0.5, muted: false }.is_default());
    }
}
