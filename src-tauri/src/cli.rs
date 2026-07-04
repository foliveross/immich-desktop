use crate::config::{self, AppConfig, UploadOptions};
use crate::credentials;
use crate::process_manager::{self, ProcessManager};
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
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
    pub process_manager: Arc<ProcessManager>,
    paused: Arc<Mutex<bool>>,
    cancel_flag: Arc<Mutex<bool>>,
    active_child: Arc<Mutex<Option<u32>>>,
}

impl CliManager {
    pub fn new(process_manager: Arc<ProcessManager>) -> Self {
        Self {
            progress: Arc::new(Mutex::new(UploadProgress::default())),
            activities: Arc::new(Mutex::new(Vec::new())),
            process_manager,
            paused: Arc::new(Mutex::new(false)),
            cancel_flag: Arc::new(Mutex::new(false)),
            active_child: Arc::new(Mutex::new(None)),
        }
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
        if let Some(pid) = *self.active_child.lock().await {
            process_manager::kill_process_tree(pid);
        }
    }

    pub async fn reset_cancel(&self) {
        *self.cancel_flag.lock().await = false;
    }

    pub async fn should_cancel(&self) -> bool {
        *self.cancel_flag.lock().await
    }

    pub async fn is_upload_running(&self) -> bool {
        self.progress.lock().await.is_running
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
        return Ok((
            "npx".to_string(),
            vec!["--yes".to_string(), "@immich/cli".to_string()],
        ));
    }

    Err(anyhow!(
        "Immich CLI not found. Install with: npm install -g @immich/cli"
    ))
}

fn which_cli(name: &str) -> bool {
    process_manager::hidden_command("where")
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
            .add_activity(
                line,
                FileStatus::Skipped,
                Some("Duplicate or skipped".to_string()),
            )
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

fn build_hidden_command(cmd: &str, args: &[String], env: &[(String, String)]) -> Command {
    let mut command = Command::new(cmd);
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.stdin(Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.as_std_mut().creation_flags(process_manager::CREATE_NO_WINDOW);
    }
    command
}

pub async fn run_login(
    manager: &CliManager,
    url: &str,
    api_key: &str,
) -> Result<String> {
    let _lock = manager
        .process_manager
        .acquire_cli_lock("login")
        .map_err(|e| anyhow!(e))?;

    let config = config::load_config()?;
    let (cmd, mut args) = resolve_cli_command(&config)?;
    args.push("login".to_string());
    args.push(url.to_string());
    args.push(api_key.to_string());

    let mut env_pairs = build_env(&config)?;
    env_pairs.push(("IMMICH_INSTANCE_URL".to_string(), url.to_string()));
    env_pairs.push(("IMMICH_API_KEY".to_string(), api_key.to_string()));

    run_command_once(manager, &cmd, &args, &env_pairs).await
}

pub async fn run_server_info(manager: &CliManager) -> Result<ServerInfo> {
    let _lock = manager
        .process_manager
        .acquire_cli_lock("server-info")
        .map_err(|e| anyhow!(e))?;

    let config = config::load_config()?;
    let (cmd, mut prefix) = resolve_cli_command(&config)?;
    prefix.push("server-info".to_string());
    let env = build_env(&config)?;
    let output = run_command_once(manager, &cmd, &prefix, &env).await?;
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
    if manager.is_upload_running().await {
        return Err(anyhow!("An upload is already in progress"));
    }

    let _lock = manager
        .process_manager
        .acquire_cli_lock("upload")
        .map_err(|e| anyhow!(e))?;

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
        manager.add_activity(path, FileStatus::Queued, None).await;
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

async fn run_command_once(
    manager: &CliManager,
    cmd: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<String> {
    let mut command = build_hidden_command(cmd, args, env);
    let child = command
        .spawn()
        .context("Failed to spawn CLI process")?;

    let pid = child.id();
    if let Some(pid) = pid {
        manager.process_manager.register_child_pid(pid);
        *manager.active_child.lock().await = Some(pid);
    }

    let output = child.wait_with_output().await?;
    if let Some(pid) = pid {
        manager.process_manager.unregister_child_pid(pid);
        *manager.active_child.lock().await = None;
    }

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
    let mut command = build_hidden_command(cmd, args, env);
    let mut child = command
        .spawn()
        .context("Failed to spawn CLI process")?;

    if let Some(pid) = child.id() {
        manager.process_manager.register_child_pid(pid);
        *manager.active_child.lock().await = Some(pid);
    }

    let stdout = child
        .stdout
        .take()
        .context("Failed to capture CLI stdout")?;
    let stderr = child.stderr.take();
    let mut reader = BufReader::new(stdout).lines();
    let mut combined = String::new();

    loop {
        if manager.should_cancel().await {
            if let Some(pid) = child.id() {
                process_manager::kill_process_tree(pid);
            }
            break;
        }

        while manager.is_paused().await {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if manager.should_cancel().await {
                break;
            }
        }

        match reader.next_line().await {
            Ok(Some(line)) => {
                combined.push_str(&line);
                combined.push('\n');
                parse_line(&line, manager).await;
                let _ = app.emit("upload-log", &line);
            }
            Ok(None) => break,
            Err(e) => return Err(e.into()),
        }
    }

    if let Some(stderr) = stderr {
        let mut err_reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = err_reader.next_line().await {
            combined.push_str(&line);
            combined.push('\n');
            parse_line(&line, manager).await;
            let _ = app.emit("upload-log", &line);
        }
    }

    let status = child.wait().await?;
    if let Some(pid) = child.id() {
        manager.process_manager.unregister_child_pid(pid);
        *manager.active_child.lock().await = None;
    }

    if manager.should_cancel().await {
        return Err(anyhow!("Upload cancelled"));
    }

    if !status.success() {
        return Err(anyhow!("Upload process exited with code {:?}", status.code()));
    }

    Ok(combined)
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
