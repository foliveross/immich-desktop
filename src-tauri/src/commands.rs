use crate::cli::{self, CliManager, FileActivity, ServerInfo, UploadProgress};
use crate::config::{self, AppConfig};
use crate::credentials;
use crate::retry_queue::{ConflictStore, RetryQueue};
use crate::sync_triggers::{self, SyncTriggerStatus};
use crate::watch::{restart_watch, WatchService};
use anyhow::Result;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
pub struct AppState {
    pub cli_manager: Arc<CliManager>,
    pub watch_service: Arc<WatchService>,
}

#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    config::load_config().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_app_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_config_path() -> Result<String, String> {
    config::config_path()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_logs_dir() -> Result<String, String> {
    config::logs_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_stored_credentials() -> bool {
    credentials::has_api_key()
}

#[tauri::command]
pub async fn store_credentials(api_key: String) -> Result<(), String> {
    credentials::store_api_key(&api_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_credentials() -> Result<(), String> {
    credentials::delete_api_key().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn complete_setup(
    app: AppHandle,
    state: State<'_, AppState>,
    server_url: String,
    api_key: String,
) -> Result<ServerInfo, String> {
    credentials::store_api_key(&api_key).map_err(|e| e.to_string())?;

    let mut config = config::load_config().map_err(|e| e.to_string())?;
    config.server_url = Some(server_url.clone());
    config.setup_complete = true;
    config::save_config(&config).map_err(|e| e.to_string())?;

    let login_output = cli::run_login(&app, &server_url, &api_key)
        .await
        .map_err(|e| e.to_string())?;

    if config.watch_mode.enabled {
        restart_watch(
            app.clone(),
            state.watch_service.clone(),
            state.cli_manager.clone(),
        )
        .await;
    }

    match cli::run_server_info(&app).await {
        Ok(info) => Ok(info),
        Err(_) => Ok(ServerInfo {
            version: login_output.lines().next().unwrap_or("Connected").to_string(),
            raw_output: login_output,
        }),
    }
}

#[tauri::command]
pub async fn test_connection(app: AppHandle) -> Result<ServerInfo, String> {
    cli::run_server_info(&app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_cli() -> Result<String, String> {
    cli::detect_cli().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_upload(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<(), String> {
    let triggers = sync_triggers::evaluate_triggers(
        &config::load_config().map_err(|e| e.to_string())?.sync_triggers,
    );
    if !triggers.can_sync {
        return Err(format!("Sync blocked: {}", triggers.reasons.join(", ")));
    }

    let manager = state.cli_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = cli::run_upload(&app, &manager, paths).await {
            log::error!("Upload failed: {e}");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn get_upload_progress(state: State<'_, AppState>) -> Result<UploadProgress, String> {
    Ok(state.cli_manager.progress.lock().await.clone())
}

#[tauri::command]
pub async fn get_file_activities(state: State<'_, AppState>) -> Result<Vec<FileActivity>, String> {
    Ok(state.cli_manager.activities.lock().await.clone())
}

#[tauri::command]
pub async fn pause_upload(state: State<'_, AppState>) -> Result<(), String> {
    state.cli_manager.set_paused(true).await;
    Ok(())
}

#[tauri::command]
pub async fn resume_upload(state: State<'_, AppState>) -> Result<(), String> {
    state.cli_manager.set_paused(false).await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_upload(state: State<'_, AppState>) -> Result<(), String> {
    state.cli_manager.request_cancel().await;
    Ok(())
}

#[tauri::command]
pub fn get_sync_trigger_status() -> Result<SyncTriggerStatus, String> {
    let config = config::load_config().map_err(|e| e.to_string())?;
    Ok(sync_triggers::evaluate_triggers(&config.sync_triggers))
}

#[tauri::command]
pub fn get_current_network() -> Result<Option<String>, String> {
    sync_triggers::get_current_network_name().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_retry_queue() -> Result<RetryQueue, String> {
    RetryQueue::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_to_retry_queue(path: String, error: String) -> Result<RetryQueue, String> {
    let mut queue = RetryQueue::load().map_err(|e| e.to_string())?;
    queue.add(path, error);
    queue.save().map_err(|e| e.to_string())?;
    Ok(queue)
}

#[tauri::command]
pub fn remove_from_retry_queue(id: String) -> Result<RetryQueue, String> {
    let mut queue = RetryQueue::load().map_err(|e| e.to_string())?;
    queue.remove(&id);
    queue.save().map_err(|e| e.to_string())?;
    Ok(queue)
}

#[tauri::command]
pub async fn retry_failed_uploads(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let queue = RetryQueue::load().map_err(|e| e.to_string())?;
    let paths = queue.paths_ready_for_retry();
    if paths.is_empty() {
        return Ok(());
    }
    start_upload(app, state, paths).await
}

#[tauri::command]
pub fn get_conflicts() -> Result<ConflictStore, String> {
    ConflictStore::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_conflict(id: String, resolution: String) -> Result<ConflictStore, String> {
    let mut store = ConflictStore::load().map_err(|e| e.to_string())?;
    store.resolve(&id, &resolution);
    store.save().map_err(|e| e.to_string())?;
    Ok(store)
}

#[tauri::command]
pub async fn toggle_watch_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let mut config = config::load_config().map_err(|e| e.to_string())?;
    config.watch_mode.enabled = enabled;
    config::save_config(&config).map_err(|e| e.to_string())?;

    if enabled {
        restart_watch(app, state.watch_service.clone(), state.cli_manager.clone()).await;
    } else {
        state.watch_service.stop();
    }
    Ok(())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    let folder = app
        .dialog()
        .file()
        .set_title("Select folder to watch")
        .blocking_pick_folder();
    Ok(folder.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn pick_upload_paths(app: AppHandle) -> Result<Vec<String>, String> {
    let files = app
        .dialog()
        .file()
        .set_title("Select files or folders to upload")
        .blocking_pick_files();
    Ok(files
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.to_string())
        .collect())
}

#[tauri::command]
pub fn open_logs_folder() -> Result<(), String> {
    let dir = config::logs_dir().map_err(|e| e.to_string())?;
    open::that(dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_config_folder() -> Result<(), String> {
    let dir = config::app_data_dir().map_err(|e| e.to_string())?;
    open::that(dir).map_err(|e| e.to_string())
}
