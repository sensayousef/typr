// Always build as a GUI-subsystem binary so no terminal window pops up on
// launch (in dev or release). A debug console can be summoned at runtime from
// the settings UI — see `console.rs`.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

mod app_state;
mod commands;
mod console;
mod history;
mod hotkey;
mod markitdown;
mod single_instance;
mod tts;
mod tts_groq;

use robin_lib::{recording::Recorder, settings::Settings};
use app_state::AppState;

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.robin.app")
}

fn autostart_enabled(app: &tauri::AppHandle) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }

    app.autolaunch().is_enabled().unwrap_or(false)
}

fn set_autostart_enabled(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) && enabled {
        return Err(
            "Launch on startup is only available from a built Robin app. Run `npm run app:build`, then launch Robin with `npm run app`.".into(),
        );
    }

    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

fn main() {
    // Newest launch wins: terminate any older (possibly tray-only) instance so
    // this process becomes the sole owner of the hotkeys and tray icon.
    single_instance::kill_other_instances();
    single_instance::disable_legacy_typr_autostart();

    let app_dir = get_app_dir();
    let settings = Settings::load(&app_dir);
    let initial_hotkey = settings.hotkey.clone();
    let initial_tts_hotkey = settings.tts_hotkey.clone();

    // Attach the debug console early (before the Tauri builder) if the user
    // left it enabled, so startup diagnostics are captured too.
    console::set_visible(settings.show_console);

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_dialog::init())
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
            commands::set_console_visible,
            markitdown::convert_markitdown,
            markitdown::save_markdown,
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
                Ok(_) => println!("[Robin] Overlay window created"),
                Err(e) => eprintln!("[Robin] Failed to create overlay: {}", e),
            }

            // ── Global hotkeys (both dictation + TTS) ────────────────────
            println!(
                "[Robin] Registering shortcuts: dictation='{}' tts='{}'",
                initial_hotkey, initial_tts_hotkey
            );
            match hotkey::register_all_hotkeys(
                app.handle(),
                &initial_hotkey,
                &initial_tts_hotkey,
            ) {
                Ok(_) => println!("[Robin] Global shortcuts registered"),
                Err(e) => eprintln!("[Robin] ERROR: Failed to register shortcuts: {}", e),
            }

            // ── Close behavior ────────────────────────────────────
            if let Some(main_win) = app.get_webview_window("main") {
                let win_clone = main_win.clone();
                let app_handle = app.handle().clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let state = app_handle.state::<AppState>();
                        let run_in_background = state
                            .settings
                            .lock()
                            .map(|settings| settings.run_in_background)
                            .unwrap_or(false);

                        if run_in_background {
                            let _ = win_clone.hide();
                            api.prevent_close();
                        } else {
                            app_handle.exit(0);
                        }
                    }
                });
            }

            // ── System tray ───────────────────────────────────────
            let autostart_is_enabled = autostart_enabled(app.handle());

            let run_in_background = app
                .state::<AppState>()
                .settings
                .lock()
                .map(|settings| settings.run_in_background)
                .unwrap_or(false);

            let autostart_item = CheckMenuItemBuilder::with_id("autostart", "Launch on Startup")
                .checked(autostart_is_enabled)
                .build(app)?;
            let background_item =
                CheckMenuItemBuilder::with_id("background", "Keep Running on Close")
                    .checked(run_in_background)
                    .build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&MenuItemBuilder::with_id("open", "Open Settings").build(app)?)
                .item(&MenuItemBuilder::with_id("toggle", "Toggle Recording").build(app)?)
                .item(&PredefinedMenuItem::separator(app)?)
                .item(&autostart_item)
                .item(&background_item)
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
                .tooltip("Robin")
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
                                eprintln!("[Robin] Tray toggle error: {}", e);
                            }
                        });
                    }
                    "autostart" => {
                        let currently_enabled = autostart_enabled(app);
                        let _ = set_autostart_enabled(app, !currently_enabled);
                    }
                    "background" => {
                        let state = app.state::<AppState>();
                        if let Ok(mut settings) = state.settings.lock() {
                            settings.run_in_background = !settings.run_in_background;
                            let _ = settings.save(&state.app_dir);
                        };
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
