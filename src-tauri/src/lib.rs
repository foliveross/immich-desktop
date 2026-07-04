mod cli;
mod commands;
mod config;
mod connection;
mod credentials;
mod discovery;
mod process_manager;
mod retry_queue;
mod sync_triggers;
mod watch;

use commands::AppState;
use process_manager::ProcessManager;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
            Target::new(TargetKind::Folder {
                path: config::logs_dir().unwrap_or_else(|_| std::env::temp_dir()),
                file_name: Some("immich-desktop".to_string()),
            }),
        ]).build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Err(msg) = ProcessManager::acquire_app_instance() {
                log::error!("{msg}");
                eprintln!("{msg}");
                std::process::exit(1);
            }

            let process_manager = Arc::new(ProcessManager::new());
            let cli_manager = Arc::new(cli::CliManager::new(process_manager.clone()));
            let watch_service = Arc::new(watch::WatchService::new());

            app.manage(AppState {
                cli_manager: cli_manager.clone(),
                watch_service: watch_service.clone(),
                process_manager: process_manager.clone(),
            });

            let pause = MenuItem::with_id(app, "pause", "Pause Sync", true, None::<&str>)?;
            let resume = MenuItem::with_id(app, "resume", "Resume Sync", true, None::<&str>)?;
            let open_ui = MenuItem::with_id(app, "open_ui", "Open Web UI", true, None::<&str>)?;
            let view_logs = MenuItem::with_id(app, "view_logs", "View Logs", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &pause,
                    &resume,
                    &PredefinedMenuItem::separator(app)?,
                    &open_ui,
                    &view_logs,
                    &PredefinedMenuItem::separator(app)?,
                    &show,
                    &quit,
                ],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Immich Desktop")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "pause" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            let manager = state.cli_manager.clone();
                            tauri::async_runtime::spawn(async move {
                                manager.set_paused(true).await;
                            });
                        }
                    }
                    "resume" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            let manager = state.cli_manager.clone();
                            tauri::async_runtime::spawn(async move {
                                manager.set_paused(false).await;
                            });
                        }
                    }
                    "open_ui" => {
                        if let Ok(cfg) = config::load_config() {
                            if let Some(url) = cfg.server_url {
                                let web_url = connection::web_url_from_api(&url);
                                let _ = open::that(web_url);
                            }
                        }
                    }
                    "view_logs" => {
                        let _ = commands::open_logs_folder();
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            state.process_manager.shutdown();
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            let watch = watch_service.clone();
            let cli = cli_manager.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(cfg) = config::load_config() {
                    if cfg.watch_mode.enabled {
                        let _ = watch.start(app_handle, cli).await;
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_app_config,
            commands::get_config_path,
            commands::get_logs_dir,
            commands::get_lock_file_path,
            commands::has_stored_credentials,
            commands::store_credentials,
            commands::clear_credentials,
            commands::discover_servers,
            commands::perform_handshake,
            commands::finalize_setup,
            commands::complete_setup,
            commands::test_connection,
            commands::detect_cli,
            commands::start_upload,
            commands::get_upload_progress,
            commands::get_file_activities,
            commands::pause_upload,
            commands::resume_upload,
            commands::cancel_upload,
            commands::get_sync_trigger_status,
            commands::get_current_network,
            commands::get_retry_queue,
            commands::add_to_retry_queue,
            commands::remove_from_retry_queue,
            commands::retry_failed_uploads,
            commands::get_conflicts,
            commands::resolve_conflict,
            commands::toggle_watch_mode,
            commands::pick_folder,
            commands::pick_upload_paths,
            commands::open_logs_folder,
            commands::open_config_folder,
            commands::clear_cli_lock,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                if let Some(state) = app.try_state::<AppState>() {
                    state.process_manager.shutdown();
                }
            }
        });
}
