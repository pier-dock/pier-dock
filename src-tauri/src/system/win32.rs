//! Bandeja no Windows: bateria, rede (WinRT), volume (Core Audio), Bluetooth (WinRT), flyouts
//! nativos via teclado e a gaveta de ícones ocultos.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice as BtDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Devices::Radios::{Radio, RadioKind, RadioState};
use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern, TreeScope_Descendants, UIA_AutomationIdPropertyId,
    UIA_ClassNamePropertyId, UIA_InvokePatternId,
};
use windows::core::{w, HSTRING, PCWSTR, VARIANT};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect, IsWindowVisible, SendMessageW, SetWindowPos, BM_CLICK, SWP_NOACTIVATE, SWP_NOSIZE,
    SWP_NOZORDER, SW_SHOWNORMAL,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_A, VK_LWIN, VK_N,
};

use super::{kind_from_class, Battery, Bluetooth, BluetoothDevice, Network, Volume};
use crate::util::{com_init, windows_dir, ErrStr};

pub fn battery() -> Battery {
    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err() {
        return Battery::absent();
    }
    // BatteryFlag: 128 = sem bateria, 255 = desconhecido.
    let present = status.BatteryFlag & 128 == 0 && status.BatteryFlag != 255;
    Battery { present, percent: status.BatteryLifePercent.min(100), charging: status.ACLineStatus == 1 }
}

pub fn network() -> Network {
    let Ok(profile) = NetworkInformation::GetInternetConnectionProfile() else { return Network::disconnected() };
    let level = profile.GetNetworkConnectivityLevel().unwrap_or(NetworkConnectivityLevel::None);
    Network {
        connected: level != NetworkConnectivityLevel::None,
        internet: level == NetworkConnectivityLevel::InternetAccess,
        wifi: profile.IsWlanConnectionProfile().unwrap_or(false),
        signal_bars: profile.GetSignalBars().ok().and_then(|r| r.Value().ok()),
        name: profile.ProfileName().map(|s| s.to_string()).unwrap_or_default(),
    }
}

fn endpoint_volume() -> windows::core::Result<IAudioEndpointVolume> {
    com_init();
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate(CLSCTX_ALL, None)
    }
}

fn key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, ..Default::default() },
        },
    }
}

fn press(keys: &[VIRTUAL_KEY]) {
    let mut inputs: Vec<INPUT> = keys.iter().map(|&k| key_input(k, false)).collect();
    inputs.extend(keys.iter().rev().map(|&k| key_input(k, true)));
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn volume() -> Result<Volume, String> {
    (|| -> windows::core::Result<Volume> {
        let vol = endpoint_volume()?;
        unsafe { Ok(Volume { level: vol.GetMasterVolumeLevelScalar()?, muted: vol.GetMute()?.as_bool() }) }
    })()
    .err_str()
}

pub fn set_volume(level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    (|| -> windows::core::Result<()> {
        let vol = endpoint_volume()?;
        unsafe {
            if let Some(level) = level {
                vol.SetMasterVolumeLevelScalar(level, std::ptr::null())?;
            }
            if let Some(muted) = muted {
                vol.SetMute(muted, std::ptr::null())?;
            }
        }
        Ok(())
    })()
    .err_str()
}

fn bluetooth_radio() -> Option<Radio> {
    let radios = Radio::GetRadiosAsync().ok()?.get().ok()?;
    radios.into_iter().find(|r| r.Kind().is_ok_and(|k| k == RadioKind::Bluetooth))
}

/// Tipo pelo Appearance do Bluetooth LE: categoria nos 10 bits altos, subcategoria nos 6 baixos.
fn kind_from_appearance(raw: u16) -> &'static str {
    const CATEGORY_HID: u16 = 0x0F;
    match (raw >> 6, raw & 0x3F) {
        (CATEGORY_HID, 1) => "keyboard",
        (CATEGORY_HID, 2) => "mouse",
        (CATEGORY_HID, 3 | 4) => "gamepad",
        _ => "other",
    }
}

fn connected_devices() -> windows::core::Result<Vec<BluetoothDevice>> {
    let mut devices: Vec<BluetoothDevice> = Vec::new();
    let classic = DeviceInformation::FindAllAsyncAqsFilter(&BtDevice::GetDeviceSelectorFromConnectionStatus(BluetoothConnectionStatus::Connected)?)?.get()?;
    for info in classic {
        let device = BtDevice::FromIdAsync(&info.Id()?)?.get()?;
        let kind = kind_from_class(device.ClassOfDevice()?.RawValue()?);
        devices.push(BluetoothDevice { name: device.Name()?.to_string(), kind, battery: None });
    }
    let le = DeviceInformation::FindAllAsyncAqsFilter(&BluetoothLEDevice::GetDeviceSelectorFromConnectionStatus(BluetoothConnectionStatus::Connected)?)?.get()?;
    for info in le {
        let device = BluetoothLEDevice::FromIdAsync(&info.Id()?)?.get()?;
        let name = device.Name()?.to_string();
        // Dispositivos dual-mode (fones, por exemplo) aparecem nas duas listas.
        if devices.iter().any(|d| d.name == name) {
            continue;
        }
        let kind = kind_from_appearance(device.Appearance()?.RawValue()?);
        devices.push(BluetoothDevice { name, kind, battery: None });
    }
    Ok(devices)
}

pub fn bluetooth() -> Bluetooth {
    let Some(radio) = bluetooth_radio() else { return Bluetooth::absent() };
    let powered = radio.State().is_ok_and(|s| s == RadioState::On);
    let devices = if powered { connected_devices().unwrap_or_default() } else { Vec::new() };
    Bluetooth { available: true, powered, can_toggle: true, devices }
}

pub fn set_bluetooth(powered: bool) -> Result<(), String> {
    let radio = bluetooth_radio().ok_or("sem adaptador Bluetooth")?;
    radio.SetStateAsync(if powered { RadioState::On } else { RadioState::Off }).err_str()?.get().err_str()?;
    Ok(())
}

fn shell_open(target: &str) {
    com_init();
    unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(target), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

/// Abre os flyouts nativos (Menu Iniciar, Configurações Rápidas e Central de Notificações), o
/// Gerenciador de Tarefas e o Calendário do Outlook.
pub fn open_shell(kind: &str) {
    match kind {
        "start" => press(&[VK_LWIN]),
        "quick-settings" => press(&[VK_LWIN, VK_A]),
        "notifications" => press(&[VK_LWIN, VK_N]),
        "system-monitor" => shell_open(&format!(r"{}\System32\Taskmgr.exe", windows_dir())),
        "calendar" => shell_open("outlookcal:"),
        "bluetooth" => shell_open("ms-settings:bluetooth"),
        _ => {}
    }
}

/// Quando a gaveta de ícones ocultos fechou sozinha (clique fora) pela última vez.
static TRAY_CLOSED_AT: Mutex<Option<Instant>> = Mutex::new(None);
/// A gaveta está sendo fechada pela seta da dock: esse fechamento não conta para `TRAY_CLOSED_AT`.
static TRAY_CLOSING: AtomicBool = AtomicBool::new(false);
static TRAY_BUSY: AtomicBool = AtomicBool::new(false);

/// A gaveta de ícones ocultos aberta: a do Windows 11 (XAML) ou a do Windows 10.
fn tray_overflow() -> Option<HWND> {
    [w!("TopLevelWindowForOverflowXamlIsland"), w!("NotifyIconOverflowWindow")]
        .into_iter()
        .filter_map(|class| unsafe { FindWindowW(class, PCWSTR::null()) }.ok())
        .find(|&h| !h.is_invalid() && unsafe { IsWindowVisible(h) }.as_bool())
}

/// Repete `f` a cada 25 ms até ela devolver algo ou o tempo acabar.
fn poll<T>(timeout: Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = f() {
            return Some(value);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Windows 11: a seta é um botão XAML da taskbar, só alcançável pela UI Automation. O id
/// `SystemTrayIcon` se repete na rede, volume, relógio e indicador de idioma; a seta é o primeiro
/// `SystemTray.NormalButton` com ele (o indicador de idioma tem a mesma classe, mas vem depois).
fn find_tray_chevron() -> windows::core::Result<IUIAutomationElement> {
    com_init();
    unsafe {
        let taskbar = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null())?;
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let condition = automation.CreateAndCondition(
            &automation.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("SystemTrayIcon"))?,
            &automation.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("SystemTray.NormalButton"))?,
        )?;
        automation.ElementFromHandle(taskbar)?.FindFirst(TreeScope_Descendants, &condition)
    }
}

fn invoke(element: &IUIAutomationElement) -> windows::core::Result<()> {
    unsafe { element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)?.Invoke() }
}

/// Windows 10: a seta é um `Button` Win32 dentro da área de notificação. No Windows 11 ele não
/// existe.
fn tray_chevron_win10() -> Option<HWND> {
    unsafe {
        let taskbar = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()).ok()?;
        let notify = FindWindowExW(taskbar, HWND::default(), w!("TrayNotifyWnd"), PCWSTR::null()).ok()?;
        FindWindowExW(notify, HWND::default(), w!("Button"), PCWSTR::null()).ok().filter(|h| !h.is_invalid())
    }
}

fn click_win10(button: HWND) {
    unsafe {
        SendMessageW(button, BM_CLICK, WPARAM(0), LPARAM(0));
    }
}

/// A taskbar que passou um tempo recolhida só monta os botões XAML depois de subir: a seta pode
/// levar mais de 400 ms para aparecer na árvore de UI Automation.
fn wait_tray_chevron() -> Option<IUIAutomationElement> {
    poll(Duration::from_millis(1500), || find_tray_chevron().ok())
}

fn click_tray_chevron() -> Result<(), String> {
    if let Some(button) = tray_chevron_win10() {
        click_win10(button);
        return Ok(());
    }
    let chevron = wait_tray_chevron().ok_or("seta da bandeja não encontrada")?;
    invoke(&chevron).err_str()
}

/// Clica na seta e espera a gaveta aparecer.
fn show_tray_overflow() -> Option<HWND> {
    if let Some(button) = tray_chevron_win10() {
        click_win10(button);
        return poll(Duration::from_secs(1), tray_overflow);
    }
    let chevron = wait_tray_chevron()?;
    invoke(&chevron).ok()?;
    let overflow = poll(Duration::from_secs(1), tray_overflow);
    if overflow.is_none() {
        // Sem ícones ocultos o Windows tira a seta, e o botão encontrado é o indicador de
        // idioma: o segundo clique fecha o que ele abriu.
        let _ = invoke(&chevron);
    }
    overflow
}

/// Escondida pelo Pier (`SW_HIDE`), a taskbar não expõe nenhum elemento à UI Automation: ela
/// reaparece só durante `f`. A gaveta é uma janela própria e continua aberta depois.
fn with_taskbar<T>(f: impl FnOnce() -> T) -> T {
    crate::taskbar::show_for_tray(true);
    let result = f();
    crate::taskbar::show_for_tray(false);
    result
}

/// Abre a gaveta de ícones ocultos centrada em `anchor_x`, com a base em `bottom` (pixels
/// físicos de tela). Com ela aberta, fecha: clicar na dock nem sempre a fecha sozinho.
pub fn open_tray(anchor_x: i32, bottom: i32) -> Result<(), String> {
    // Abrir leva até ~1,5 s; um clique nesse meio-tempo esconderia a taskbar no meio da busca.
    if TRAY_BUSY.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let result = toggle_tray(anchor_x, bottom);
    TRAY_BUSY.store(false, Ordering::SeqCst);
    result
}

fn toggle_tray(anchor_x: i32, bottom: i32) -> Result<(), String> {
    // A gaveta fecha no mousedown fora dela, antes deste clique chegar: o clique que a fechou não
    // deve reabri-la.
    let just_closed = TRAY_CLOSED_AT.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(400));
    if just_closed {
        return Ok(());
    }
    if tray_overflow().is_some() {
        TRAY_CLOSING.store(true, Ordering::SeqCst);
        let _ = with_taskbar(click_tray_chevron);
        if poll(Duration::from_millis(500), || tray_overflow().is_none().then_some(())).is_none() {
            TRAY_CLOSING.store(false, Ordering::SeqCst);
        }
        return Ok(());
    }
    let overflow = with_taskbar(show_tray_overflow).ok_or("não foi possível abrir a bandeja")?;
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(overflow, &mut rect).is_ok() {
            let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
            let flags = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
            let _ = SetWindowPos(overflow, HWND::default(), anchor_x - width / 2, bottom - height, 0, 0, flags);
        }
    }
    let handle = overflow.0 as isize;
    std::thread::spawn(move || {
        let overflow = HWND(handle as _);
        while unsafe { IsWindowVisible(overflow) }.as_bool() {
            std::thread::sleep(Duration::from_millis(50));
        }
        if !TRAY_CLOSING.swap(false, Ordering::SeqCst) {
            *TRAY_CLOSED_AT.lock().unwrap() = Some(Instant::now());
        }
    });
    Ok(())
}
