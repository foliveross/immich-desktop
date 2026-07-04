use crate::config::{self, AppConfig, UploadOptions};
use crate::credentials;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Queued,
    Uploading,
    Skipped,
    Failed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileActivity {
    pub id: String,
    pub path: String,
    pub status: FileStatus,
    pub message: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadProgress {
    pub total_files: u64,
    pub completed_files: u64,
    pub failed_files: u64,
    pub skipped_files: u64,
    pub bytes_per_second: f64,
    pub eta_seconds: Option<u64>,
    pub is_running: bool,
    pub is_paused: bool,
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub raw_output: String,
}

pub struct CliManager {
    pub progress: Arc<Mutex<UploadProgress>>,
    pub activities: Arc<Mutex<Vec<FileActivity>>>,
    paused: Arc<Mutex<bool>>,
    cancel_flag: Arc<Mutex<bool>>,
}

impl Default for CliManager {
    fn default() -> Self {
        Self {
            progress: Arc::new(Mutex::new(UploadProgress::default())),
            activities: Arc::new(Mutex::new(Vec::new())),
            paused: Arc::new(Mutex::new(false)),
            cancel_flag: Arc::new(Mutex::new(false)),
        }
    }
}

impl CliManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn is_paused(&self) -> bool {
        *self.paused.lock().await
    }

    pub async fn set_paused(&self, paused: bool) {
        *self.paused.lock().await = paused;
        let mut progress = self.progress.lock().await;
        progress.is_paused = paused;
    }

    pub async fn request_cancel(&self) {
        *self.cancel_flag.lock().await = true;
    }

    pub async fn reset_cancel(&self) {
        *self.cancel_flag.lock().await = false;
    }

    pub async fn should_cancel(&self) -> bool {
        *self.cancel_flag.lock().await
    }

    async fn add_activity(&self, path: &str, status: FileStatus, message: Option<String>) {
        let activity = FileActivity {
            id: uuid::Uuid::new_v4().to_string(),
            path: path.to_string(),
            status,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let mut activities = self.activities.lock().await;
        activities.insert(0, activity);
        if activities.len() > 500 {
            activities.truncate(500);
        }
    }
}

fn resolve_cli_command(config: &AppConfig) -> Result<(String, Vec<String>)> {
    if let Some(path) = &config.cli_path {
        if Path::new(path).exists() {
            return Ok((path.clone(), vec![]));
        }
    }

    if which_cli("immich") {
        return Ok(("immich".to_string(), vec![]));
    }

    if which_cli("npx") {
        return Ok(("npx".to_string(), vec!["--yes".to_string(), "@immich/cli".to_string()]));
    }

    Err(anyhow!(
        "Immich CLI not found. Install with: npm install -g @immich/cli"
    ))
}

fn which_cli(name: &str) -> bool {
    std::process::Command::new("where")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn build_env(config: &AppConfig) -> Result<Vec<(String, String)>> {
    let mut env = Vec::new();

    if let Some(url) = &config.server_url {
        env.push(("IMMICH_INSTANCE_URL".to_string(), url.clone()));
    }

    if let Some(key) = credentials::get_api_key()? {
        env.push(("IMMICH_API_KEY".to_string(), key));
    }

    let immich_dir = config::immich_config_dir()?;
    env.push((
        "IMMICH_CONFIG_DIR".to_string(),
        immich_dir.to_string_lossy().to_string(),
    ));

    env.push((
        "IMMICH_UPLOAD_CONCURRENCY".to_string(),
        config.upload_options.concurrency.to_string(),
    ));

    if config.upload_options.recursive {
        env.push(("IMMICH_RECURSIVE".to_string(), "true".to_string()));
    }

    Ok(env)
}

fn append_upload_args(args: &mut Vec<String>, options: &UploadOptions, paths: &[String]) {
    if options.recursive {
        args.push("--recursive".to_string());
    }
    if options.album {
        args.push("--album".to_string());
    }
    if let Some(name) = &options.album_name {
        args.push("--album-name".to_string());
        args.push(name.clone());
    }
    for pattern in &options.ignore_patterns {
        args.push("--ignore".to_string());
        args.push(pattern.clone());
    }
    if options.include_hidden {
        args.push("--include-hidden".to_string());
    }
    if options.dry_run {
        args.push("--dry-run".to_string());
    }
    if options.skip_hash {
        args.push("--skip-hash".to_string());
    }
    args.push("--concurrency".to_string());
    args.push(options.concurrency.to_string());
    args.push("--json-output".to_string());
    args.extend(paths.iter().cloned());
}

async fn parse_line(line: &str, manager: &CliManager) {
    let lower = line.to_lowercase();

    if lower.contains("uploading") || lower.contains("processing") {
        let mut progress = manager.progress.lock().await;
        progress.is_running = true;
        progress.current_file = Some(line.to_string());
        drop(progress);
        manager
            .add_activity(line, FileStatus::Uploading, None)
            .await;
    } else if lower.contains("skip") || lower.contains("duplicate") {
        let mut progress = manager.progress.lock().await;
        progress.skipped_files += 1;
        progress.completed_files += 1;
        drop(progress);
        manager
            .add_activity(line, FileStatus::Skipped, Some("Duplicate or skipped".to_string()))
            .await;
    } else if lower.contains("error") || lower.contains("fail") {
        let mut progress = manager.progress.lock().await;
        progress.failed_files += 1;
        drop(progress);
        manager
            .add_activity(line, FileStatus::Failed, Some(line.to_string()))
            .await;
    } else if lower.contains("uploaded") || lower.contains("success") {
        let mut progress = manager.progress.lock().await;
        progress.completed_files += 1;
        drop(progress);
        manager
            .add_activity(line, FileStatus::Completed, None)
            .await;
    }

    if let Ok(re) = Regex::new(r"(\d+\.?\d*)\s*(?:MB|MiB)/s") {
        if let Some(caps) = re.captures(&lower) {
            if let Some(m) = caps.get(1) {
                if let Ok(speed) = m.as_str().parse::<f64>() {
                    let mut progress = manager.progress.lock().await;
                    progress.bytes_per_second = speed * 1024.0 * 1024.0;
                }
            }
        }
    }
}

pub async fn run_login(app: &AppHandle, url: &str, api_key: &str) -> Result<String> {
    let config = config::load_config()?;
    let (cmd, prefix) = resolve_cli_command(&config)?;

    credentials::store_api_key(api_key)?;

    let mut args = prefix;
    args.push("login".to_string());
    args.push(url.to_string());
    args.push(api_key.to_string());

    let mut env_pairs = build_env(&config)?;
    env_pairs.push(("IMMICH_INSTANCE_URL".to_string(), url.to_string()));
    env_pairs.push(("IMMICH_API_KEY".to_string(), api_key.to_string()));

    let output = run_command(app, &cmd, &args, &env_pairs).await?;
    Ok(output)
}

pub async fn run_server_info(app: &AppHandle) -> Result<ServerInfo> {
    let config = config::load_config()?;
    let (cmd, mut prefix) = resolve_cli_command(&config)?;
    prefix.push("server-info".to_string());
    let env = build_env(&config)?;
    let output = run_command(app, &cmd, &prefix, &env).await?;
    let version = output
        .lines()
        .find(|l| l.contains("version") || l.contains("Version"))
        .unwrap_or(&output)
        .to_string();
    Ok(ServerInfo {
        version,
        raw_output: output,
    })
}

pub async fn run_upload(
    app: &AppHandle,
    manager: &CliManager,
    paths: Vec<String>,
) -> Result<String> {
    manager.reset_cancel().await;
    {
        let mut progress = manager.progress.lock().await;
        *progress = UploadProgress {
            is_running: true,
            ..Default::default()
        };
        progress.total_files = paths.len() as u64;
    }

    let config = config::load_config()?;
    let (cmd, mut args) = resolve_cli_command(&config)?;
    args.push("upload".to_string());
    append_upload_args(&mut args, &config.upload_options, &paths);

    for path in &paths {
        manager
            .add_activity(path, FileStatus::Queued, None)
            .await;
    }

    let env = build_env(&config)?;
    let output = run_command_with_progress(app, manager, &cmd, &args, &env).await?;

    {
        let mut progress = manager.progress.lock().await;
        progress.is_running = false;
        progress.current_file = None;
    }

    Ok(output)
}

async fn run_command(
    app: &AppHandle,
    cmd: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<String> {
    let shell = app.shell();
    let mut command = shell.command(cmd);
    for arg in args {
        command = command.arg(arg);
    }
    for (key, value) in env {
        command = command.env(key, value);
    }

    let output = command.output().await.context("Failed to execute CLI command")?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("CLI command failed: {stderr}"))
    }
}

async fn run_command_with_progress(
    app: &AppHandle,
    manager: &CliManager,
    cmd: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<String> {
    let shell = app.shell();
    let mut command = shell.command(cmd);
    for arg in args {
        command = command.arg(arg);
    }
    for (key, value) in env {
        command = command.env(key, value);
    }

    let (mut rx, _child) = command
        .spawn()
        .context("Failed to spawn CLI process")?;

    let mut stdout = String::new();

    while let Some(event) = rx.recv().await {
        if manager.should_cancel().await {
            break;
        }

        while manager.is_paused().await {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if manager.should_cancel().await {
                break;
            }
        }

        match event {
            CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                let text = String::from_utf8_lossy(&line).to_string();
                stdout.push_str(&text);
                stdout.push('\n');
                for l in text.lines() {
                    parse_line(l, manager).await;
                }
                let _ = app.emit("upload-log", &text);
            }
            CommandEvent::Terminated(payload) => {
                if payload.code != Some(0) {
                    return Err(anyhow!("Upload process exited with code {:?}", payload.code));
                }
            }
            _ => {}
        }
    }

    Ok(stdout)
}

pub async fn detect_cli() -> Result<String> {
    let config = config::load_config()?;
    let (cmd, prefix) = resolve_cli_command(&config)?;
    if prefix.is_empty() {
        Ok(cmd)
    } else {
        Ok(format!("{} {}", cmd, prefix.join(" ")))
    }
}
