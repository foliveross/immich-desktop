use crate::config;
use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Hide console windows for child processes on Windows.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CliLockInfo {
    pub pid: u32,
    pub operation: String,
    pub started_at: String,
}

pub struct CliLockGuard<'a> {
    manager: &'a ProcessManager,
    released: bool,
}

impl Drop for CliLockGuard<'_> {
    fn drop(&mut self) {
        if !self.released {
            self.manager.release_cli_lock();
        }
    }
}

impl<'a> CliLockGuard<'a> {
    pub fn release(mut self) {
        self.released = true;
        self.manager.release_cli_lock();
    }
}

pub struct ProcessManager {
    child_pids: Mutex<Vec<u32>>,
    cli_busy: AtomicBool,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            child_pids: Mutex::new(Vec::new()),
            cli_busy: AtomicBool::new(false),
        }
    }

    pub fn lock_path() -> Result<std::path::PathBuf> {
        Ok(config::app_data_dir()?.join("immich-desktop.lock"))
    }

    pub fn app_instance_lock_path() -> Result<std::path::PathBuf> {
        Ok(config::app_data_dir()?.join("immich-desktop-app.lock"))
    }

    pub fn acquire_app_instance() -> Result<(), String> {
        config::ensure_app_dirs().map_err(|e| e.to_string())?;
        let path = Self::app_instance_lock_path().map_err(|e| e.to_string())?;

        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if Self::is_pid_alive(pid) {
                        return Err(format!(
                            "Immich Desktop is already running (PID {pid}). \
                             Check the system tray or end that process before relaunching."
                        ));
                    }
                }
            }
            let _ = fs::remove_file(&path);
        }

        fs::write(&path, std::process::id().to_string()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn release_app_instance() {
        if let Ok(path) = Self::app_instance_lock_path() {
            let _ = fs::remove_file(path);
        }
    }

    fn read_lock_file() -> Result<Option<CliLockInfo>> {
        let path = Self::lock_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path).context("Failed to read CLI lock file")?;
        let info: CliLockInfo = serde_json::from_str(&contents)
            .context("Failed to parse CLI lock file")?;
        Ok(Some(info))
    }

    fn is_pid_alive(pid: u32) -> bool {
        #[cfg(windows)]
        {
            let output = hidden_command("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                .output();
            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    text.contains(&pid.to_string())
                }
                Err(_) => false,
            }
        }
        #[cfg(not(windows))]
        {
            Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
    }

    pub fn acquire_cli_lock(&self, operation: &str) -> Result<CliLockGuard<'_>, String> {
        if self.cli_busy.swap(true, Ordering::SeqCst) {
            return Err("A CLI operation is already running in this session.".to_string());
        }

        if let Ok(Some(existing)) = Self::read_lock_file() {
            if Self::is_pid_alive(existing.pid) {
                self.cli_busy.store(false, Ordering::SeqCst);
                return Err(format!(
                    "CLI backend is locked by PID {} ({}) since {}. \
                     Delete {:?} if the app crashed.",
                    existing.pid, existing.operation, existing.started_at,
                    Self::lock_path().map_err(|e| e.to_string())?
                ));
            }
            let _ = fs::remove_file(Self::lock_path().map_err(|e| e.to_string())?);
        }

        config::ensure_app_dirs().map_err(|e| e.to_string())?;
        let info = CliLockInfo {
            pid: std::process::id(),
            operation: operation.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        fs::write(
            Self::lock_path().map_err(|e| e.to_string())?,
            serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        Ok(CliLockGuard {
            manager: self,
            released: false,
        })
    }

    pub fn release_cli_lock(&self) {
        self.cli_busy.store(false, Ordering::SeqCst);
        if let Ok(path) = Self::lock_path() {
            let _ = fs::remove_file(path);
        }
    }

    pub fn register_child_pid(&self, pid: u32) {
        self.child_pids.lock().push(pid);
    }

    pub fn unregister_child_pid(&self, pid: u32) {
        self.child_pids.lock().retain(|p| *p != pid);
    }

    pub fn kill_all_children(&self) {
        let pids: Vec<u32> = self.child_pids.lock().drain(..).collect();
        for pid in pids {
            kill_process_tree(pid);
        }
        self.release_cli_lock();
    }

    pub fn shutdown(&self) {
        log::info!("Shutting down CLI child processes");
        self.kill_all_children();
        Self::release_app_instance();
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn hidden_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

pub fn apply_hidden(cmd: &mut Command) {
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(windows)]
pub fn kill_process_tree(pid: u32) {
    let _ = hidden_command("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
pub fn kill_process_tree(pid: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

pub fn stale_lock_message() -> Result<String> {
    let path = ProcessManager::lock_path()?;
    Ok(format!(
        "If the app crashed, delete the lock file manually:\n{}",
        path.display()
    ))
}
