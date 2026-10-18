#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apps;
mod audio;
mod backdrop;
mod calendar;
mod claude;
mod config;
mod dock_window;
mod github;
mod i18n;
mod icons;
mod media;
mod panels;
mod platform;
mod pomodoro;
mod report;
mod system;
mod taskbar;
mod updater;
mod usage;
mod util;
mod weather;
mod wifi;
mod windows_list;

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const APP_DIGITS: [Code; 9] = [
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];

fn quit(app: &AppHandle) {
    taskbar::restore();
    app.exit(0);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    quit(&app);
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let [settings_label, reload_label, quit_label] = i18n::tray_labels(i18n::current(app));
    let settings = MenuItem::with_id(app, "settings", settings_label, true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", reload_label, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &reload, &separator, &quit_item])?;

    let tray = TrayIconBuilder::with_id("main")
        .tooltip("Pier")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                let _ = panels::open_settings_window(app);
            }
            "reload" => {
                if let Some(dock) = app.get_webview_window(dock_window::DOCK_LABEL) {
                    let _ = dock.eval("location.reload()");
                }
            }
            "quit" => quit(app),
            updater::MENU_ID => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = updater::install_update(app).await {
                        log::error!("falha ao instalar a atualização: {e}");
                    }
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let _ = panels::open_settings_window(tray.app_handle());
            }
        });
    // Monocromático: no macOS como imagem-modelo (o sistema pinta na cor da barra de menus).
    #[cfg(target_os = "macos")]
    let tray = tray.icon(Image::from_bytes(include_bytes!("../icons/tray-template.png"))?).icon_as_template(true);
    #[cfg(not(target_os = "macos"))]
    let tray = tray.icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?);
    tray.build(app)?;
    app.manage(updater::TrayMenu(menu));
    app.manage(i18n::TrayItems { settings, reload, quit: quit_item });
    Ok(())
}

/// O WinDock registrava o início automático com o nome dele, apontando para o executável antigo:
/// troca esse registro pelo do Pier.
fn migrate_autostart(app: &AppHandle) {
    use tauri_plugin_autostart::ManagerExt;
    let Ok(exe) = std::env::current_exe() else { return };
    let mut builder = auto_launch::AutoLaunchBuilder::new();
    builder.set_app_name("WinDock").set_app_path(&exe.display().to_string());
    #[cfg(target_os = "macos")]
    builder.set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent);
    let Ok(legacy) = builder.build() else { return };
    if legacy.is_enabled().unwrap_or(false) && legacy.disable().is_ok() {
        let _ = app.autolaunch().enable();
    }
}

/// O início automático guarda o caminho do executável de quando foi ligado (o do `tauri dev`, por
/// exemplo): a versão instalada o atualiza para o próprio caminho.
#[cfg(not(debug_assertions))]
fn refresh_autostart(app: &AppHandle) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if autolaunch.is_enabled().unwrap_or(false) {
        let _ = autolaunch.enable();
    }
}

/// Ctrl+Alt+1..9 (⌃⌥ no macOS) abre/foca o app na posição N da dock (a dock decide o que fazer).
fn register_shortcuts(app: &AppHandle) {
    for code in APP_DIGITS {
        let _ = app.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), code));
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    platform::linux::prepare_env();
    // Rodando como AppImage, o Pier se cadastra no menu na primeira execução.
    #[cfg(target_os = "linux")]
    platform::linux::desktop_integration::install_if_needed();
    if std::env::args().any(|a| a == "--restore-taskbar") {
        taskbar::restore();
        return;
    }
    taskbar::install_crash_guards();

    let shortcuts = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                if let Some(index) = APP_DIGITS.iter().position(|c| shortcut.key == *c) {
                    let _ = app.emit_to(dock_window::DOCK_LABEL, "shortcut-app", index);
                }
            }
        })
        .build();

    let app = tauri::Builder::default()
        .plugin(report::log_plugin())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = panels::open_settings_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(shortcuts)
        .manage(config::ConfigState(Mutex::new(config::load())))
        .manage(updater::UpdateState::default())
        .invoke_handler(tauri::generate_handler![
            quit_app,
            config::get_config,
            config::save_config,
            dock_window::set_dock_layout,
            dock_window::set_hit_rect,
            dock_window::dock_visible,
            dock_window::set_dock_hover,
            windows_list::get_windows,
            windows_list::toggle_app_windows,
            windows_list::activate_window,
            windows_list::close_windows,
            apps::list_installed_apps,
            apps::launch_app,
            apps::resolve_app_path,
            icons::get_icon,
            media::media_state,
            media::media_thumbnail,
            media::media_control,
            system::system_status,
            system::get_volume,
            system::set_volume,
            system::open_shell,
            system::open_tray,
            system::bluetooth_status,
            system::set_bluetooth,
            audio::audio_status,
            audio::set_output_device,
            audio::set_app_volume,
            wifi::wifi_status,
            wifi::wifi_scan,
            wifi::wifi_set_enabled,
            wifi::wifi_connect,
            wifi::wifi_disconnect,
            wifi::wifi_request_permission,
            github::github_fetch,
            github::github_cached,
            github::github_auth_status,
            github::github_set_token,
            weather::weather_fetch,
            weather::weather_cached,
            weather::weather_search,
            pomodoro::pomodoro_state,
            pomodoro::pomodoro_control,
            usage::system_usage,
            usage::top_apps_usage,
            usage::quit_app_processes,
            calendar::calendar_fetch,
            calendar::calendar_cached,
            calendar::calendar_set_url,
            claude::claude_usage,
            claude::claude_usage_cached,
            panels::open_panel,
            panels::close_panel,
            panels::focus_panel,
            panels::open_settings,
            platform::platform_info,
            i18n::system_language,
            platform::platform_setup,
            updater::check_update,
            updater::install_update,
            report::report_environment,
            report::recent_logs,
            report::report_issue,
        ])
        .setup(|app| {
            // Sem ícone no Dock nem barra de menus: a dock é o app.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            let (hide_taskbar, auto_hide, material) = {
                let config = app.state::<config::ConfigState>();
                let config = config.0.lock().unwrap();
                (config.hide_taskbar, config.auto_hide.clone(), config.material.clone())
            };
            dock_window::init_auto_hide(&auto_hide);
            backdrop::init_material(&material);
            if hide_taskbar {
                taskbar::hide();
            }
            #[cfg(windows)]
            {
                let restart_handle = handle.clone();
                taskbar::start_watchdog(move || dock_window::reregister(&restart_handle));
            }

            if let Some(dock) = app.get_webview_window(dock_window::DOCK_LABEL) {
                dock_window::init(&dock);
            }
            // O painel não recebe foco no macOS (o app nunca fica ativo), então "perder o foco" não
            // serve para fechá-lo.
            #[cfg(target_os = "macos")]
            {
                let panel_handle = handle.clone();
                platform::macos::events::on_click_outside(move || {
                    panels::close_open_panel(&panel_handle);
                });
            }
            windows_list::start(handle.clone());
            media::start(handle.clone());
            pomodoro::start(handle.clone());
            usage::start();
            register_shortcuts(&handle);
            migrate_autostart(&handle);
            #[cfg(not(debug_assertions))]
            refresh_autostart(&handle);
            #[cfg(not(debug_assertions))]
            updater::start(handle.clone());
            // No GNOME (inclusive Wayland) quem registra os atalhos é a extensão.
            #[cfg(target_os = "linux")]
            platform::linux::gnome_shell::set_shortcuts_enabled(true);
            if let Err(e) = build_tray(&handle) {
                log::error!("falha ao criar o ícone da bandeja: {e}");
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Pier");

    app.run(|app, event| match event {
        // Fechar a janela de configurações ou um painel não deve encerrar a dock.
        RunEvent::ExitRequested { code: None, api, .. } if app.get_webview_window(dock_window::DOCK_LABEL).is_some() => {
            api.prevent_exit();
        }
        RunEvent::Exit => taskbar::restore(),
        _ => {}
    });
}
