use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_QUALIFIER: &str = "com";
const APP_ORG: &str = "ImmichDesktop";
const APP_NAME: &str = "ImmichDesktop";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadOptions {
    #[serde(default = "default_true")]
    pub recursive: bool,
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
    #[serde(default)]
    pub album: bool,
    #[serde(default)]
    pub album_name: Option<String>,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub skip_hash: bool,
}

fn default_true() -> bool {
    true
}

fn default_concurrency() -> u32 {
    4
}

impl Default for UploadOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            concurrency: 4,
            album: false,
            album_name: None,
            ignore_patterns: Vec::new(),
            include_hidden: false,
            dry_run: false,
            skip_hash: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchModeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_debounce")]
    pub debounce_ms: u64,
}

fn default_debounce() -> u64 {
    5000
}

impl Default for WatchModeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_start_hour")]
    pub start_hour: u8,
    #[serde(default = "default_end_hour")]
    pub end_hour: u8,
}

fn default_start_hour() -> u8 {
    22
}

fn default_end_hour() -> u8 {
    6
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start_hour: 22,
            end_hour: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTriggersConfig {
    #[serde(default)]
    pub wifi_only: bool,
    #[serde(default)]
    pub allowed_networks: Vec<String>,
    #[serde(default)]
    pub require_plugged_in: bool,
    #[serde(default)]
    pub schedule: ScheduleConfig,
}

impl Default for SyncTriggersConfig {
    fn default() -> Self {
        Self {
            wifi_only: false,
            allowed_networks: Vec::new(),
            require_plugged_in: false,
            schedule: ScheduleConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server_url: Option<String>,
    #[serde(default)]
    pub watch_folders: Vec<String>,
    #[serde(default)]
    pub upload_options: UploadOptions,
    #[serde(default)]
    pub watch_mode: WatchModeConfig,
    #[serde(default)]
    pub sync_triggers: SyncTriggersConfig,
    #[serde(default)]
    pub cli_path: Option<String>,
    #[serde(default = "default_true")]
    pub use_credential_manager: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub setup_complete: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: None,
            watch_folders: Vec::new(),
            upload_options: UploadOptions::default(),
            watch_mode: WatchModeConfig::default(),
            sync_triggers: SyncTriggersConfig::default(),
            cli_path: None,
            use_credential_manager: true,
            start_minimized: false,
            setup_complete: false,
        }
    }
}

pub fn app_data_dir() -> Result<PathBuf> {
    ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME)
        .map(|dirs| dirs.data_dir().to_path_buf())
        .context("Could not resolve application data directory")
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("config.json"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("logs"))
}

pub fn immich_config_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join(".immich"))
}

pub fn retry_queue_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("retry_queue.json"))
}

pub fn ensure_app_dirs() -> Result<()> {
    let data = app_data_dir()?;
    fs::create_dir_all(&data)?;
    fs::create_dir_all(logs_dir()?)?;
    fs::create_dir_all(immich_config_dir()?)?;
    Ok(())
}

pub fn load_config() -> Result<AppConfig> {
    ensure_app_dirs()?;
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = fs::read_to_string(&path).context("Failed to read config.json")?;
    serde_json::from_str(&contents).context("Failed to parse config.json")
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    ensure_app_dirs()?;
    let path = config_path()?;
    let contents =
        serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(path, contents).context("Failed to write config.json")
}
