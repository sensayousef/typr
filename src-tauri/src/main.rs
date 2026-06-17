#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::MacosLauncher;

mod app_state;
mod commands;
mod history;
mod hotkey;
mod single_instance;
mod tts;
mod tts_groq;

use typr_lib::{recording::Recorder, settings::Settings};
use app_state::AppState;

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.typr.app")
}

fn main() {
    // Newest launch wins: terminate any older (possibly tray-only) instance so
    // this process becomes the sole owner of the hotkeys and tray icon.
    single_instance::kill_other_instances();

    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();
    let initial_tts_hotkey = settings.tts_hotkey.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(AppState {
            recorder: Recorder::new(),
            history: Mutex::new(history::History::load(&app_dir)),
            settings: Mutex::new(settings),
            app_dir,
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_microphones,
            commands::get_recording_state,
            commands::check_model_downloaded,
            commands::download_model,
            commands::toggle_recording,
            commands::get_history,
            commands::clear_history,
            commands::get_autostart_enabled,
            commands::set_autostart,
            hotkey::update_hotkey,
            tts::list_voices_cmd,
            tts::stop_speaking,
            tts::pause_speaking,
            tts::resume_speaking,
            tts::speak_text_cmd,
            tts::update_tts_hotkey,
        ])
        .setup(move |app| {
            // ── Speech service (must be managed here — needs AppHandle) ────
            let speech_service = tts::SpeechService::new(app.handle().clone());
            app.manage(speech_service);

            // ── Overlay window ───────────────────────────────────
            let monitor = app.primary_monitor().ok().flatten();
            let (x, y) = if let Some(m) = monitor {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                ((logical_w - 60.0) as i32, 10_i32)
            } else {
                (1380, 10)
            };

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("src/overlay.html".into()),
            )
            .title("")
            .inner_size(50.0, 50.0)
            .position(x as f64, y as f64)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .build();

            match overlay {
                Ok(_) => println!("[Typr] Overlay window created"),
                Err(e) => eprintln!("[Typr] Failed to create overlay: {}", e),
            }

            // ── Global hotkeys (both dictation + TTS) ────────────────────
            println!(
                "[Typr] Registering shortcuts: dictation='{}' tts='{}'",
                initial_hotkey, initial_tts_hotkey
            );
            match hotkey::register_all_hotkeys(
                app.handle(),
                &initial_hotkey,
                &initial_tts_hotkey,
            ) {
                Ok(_) => println!("[Typr] Global shortcuts registered"),
                Err(e) => eprintln!("[Typr] ERROR: Failed to register shortcuts: {}", e),
            }

            // ── Hide to tray on window close ──────────────────────
            if let Some(main_win) = app.get_webview_window("main") {
                let win_clone = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let _ = win_clone.hide();
                        api.prevent_close();
                    }
                });
            }

            // ── System tray ───────────────────────────────────────
            let autostart_enabled = {
                use tauri_plugin_autostart::ManagerExt;
                app.autolaunch().is_enabled().unwrap_or(false)
            };

            let autostart_item = CheckMenuItemBuilder::with_id("autostart", "Launch on Startup")
                .checked(autostart_enabled)
                .build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&MenuItemBuilder::with_id("open", "Open Settings").build(app)?)
                .item(&MenuItemBuilder::with_id("toggle", "Toggle Recording").build(app)?)
                .item(&PredefinedMenuItem::separator(app)?)
                .item(&autostart_item)
                .item(&PredefinedMenuItem::separator(app)?)
                .item(&PredefinedMenuItem::quit(app, Some("Quit"))?)
                .build()?;

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("app icon must be set");

            TrayIconBuilder::with_id("main")
                .icon(icon)
                .menu(&menu)
                .tooltip("Typr")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "toggle" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<AppState>();
                            if let Err(e) =
                                crate::hotkey::do_toggle_recording(&app, state.inner()).await
                            {
                                eprintln!("[Typr] Tray toggle error: {}", e);
                            }
                        });
                    }
                    "autostart" => {
                        use tauri_plugin_autostart::ManagerExt;
                        let currently_enabled = app.autolaunch().is_enabled().unwrap_or(false);
                        if currently_enabled {
                            let _ = app.autolaunch().disable();
                        } else {
                            let _ = app.autolaunch().enable();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    {
                        if button == MouseButton::Left
                            && button_state == MouseButtonState::Up
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
