use crate::cli::CliManager;
use crate::config;
use crate::sync_triggers;
use anyhow::Result;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex as SyncMutex;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

pub struct WatchService {
    watcher: SyncMutex<Option<RecommendedWatcher>>,
    debounce_paths: Arc<SyncMutex<Vec<String>>>,
}

impl WatchService {
    pub fn new() -> Self {
        Self {
            watcher: SyncMutex::new(None),
            debounce_paths: Arc::new(SyncMutex::new(Vec::new())),
        }
    }

    pub fn stop(&self) {
        *self.watcher.lock() = None;
    }

    pub async fn start(
        &self,
        app: AppHandle,
        cli_manager: Arc<CliManager>,
    ) -> Result<()> {
        self.stop();

        let config = config::load_config()?;
        if !config.watch_mode.enabled || config.watch_folders.is_empty() {
            return Ok(());
        }

        let debounce_ms = config.watch_mode.debounce_ms;
        let folders = config.watch_folders.clone();
        let _debounce_paths = self.debounce_paths.clone();

        let (tx, mut rx) = mpsc::channel::<String>(256);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_)
                    ) {
                        for path in event.paths {
                            if path.is_file() {
                                let _ = tx.blocking_send(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            },
            Config::default(),
        )?;

        for folder in &folders {
            if Path::new(folder).exists() {
                watcher.watch(Path::new(folder), RecursiveMode::Recursive)?;
            }
        }

        *self.watcher.lock() = Some(watcher);

        let app_clone = app.clone();
        let manager = cli_manager.clone();
        tokio::spawn(async move {
            let mut pending: Vec<String> = Vec::new();
            loop {
                tokio::select! {
                    Some(path) = rx.recv() => {
                        if !pending.contains(&path) {
                            pending.push(path);
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(debounce_ms)) => {
                        if pending.is_empty() {
                            continue;
                        }

                        let cfg = config::load_config().unwrap_or_default();
                        let triggers = sync_triggers::evaluate_triggers(&cfg.sync_triggers);
                        if !triggers.can_sync {
                            let _ = app_clone.emit("watch-blocked", &triggers);
                            pending.clear();
                            continue;
                        }

                        let paths = std::mem::take(&mut pending);
                        let _ = app_clone.emit("watch-triggered", &paths);

                        if let Err(e) = crate::cli::run_upload(&app_clone, &manager, paths).await {
                            log::error!("Watch upload failed: {e}");
                            let _ = app_clone.emit("watch-error", e.to_string());
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

pub async fn restart_watch(app: AppHandle, watch: Arc<WatchService>, cli: Arc<CliManager>) {
    watch.stop();
    if let Err(e) = watch.start(app, cli).await {
        log::error!("Failed to restart watch service: {e}");
    }
}
