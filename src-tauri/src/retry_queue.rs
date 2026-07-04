use crate::config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryItem {
    pub id: String,
    pub path: String,
    pub error: String,
    pub attempts: u32,
    pub last_attempt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryQueue {
    pub items: Vec<RetryItem>,
    pub max_attempts: u32,
}

impl Default for RetryQueue {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            max_attempts: 5,
        }
    }
}

impl RetryQueue {
    pub fn load() -> Result<Self> {
        let path = config::retry_queue_path()?;
        if !path.exists() {
            return Ok(Self {
                items: Vec::new(),
                max_attempts: 5,
            });
        }
        let contents = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&contents).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        config::ensure_app_dirs()?;
        let path = config::retry_queue_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents).context("Failed to save retry queue")
    }

    pub fn add(&mut self, path: String, error: String) {
        if self.items.iter().any(|i| i.path == path) {
            return;
        }
        self.items.push(RetryItem {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            error,
            attempts: 0,
            last_attempt: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn remove(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
    }

    pub fn increment_attempt(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.attempts += 1;
            item.last_attempt = chrono::Utc::now().to_rfc3339();
        }
    }

    pub fn paths_ready_for_retry(&self) -> Vec<String> {
        self.items
            .iter()
            .filter(|i| i.attempts < self.max_attempts)
            .map(|i| i.path.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictItem {
    pub id: String,
    pub local_path: String,
    pub remote_info: Option<String>,
    pub local_modified: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConflictStore {
    pub conflicts: Vec<ConflictItem>,
}

impl ConflictStore {
    pub fn path() -> Result<PathBuf> {
        Ok(config::app_data_dir()?.join("conflicts.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&contents).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        config::ensure_app_dirs()?;
        let path = Self::path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents).context("Failed to save conflicts")
    }

    pub fn resolve(&mut self, id: &str, resolution: &str) {
        if let Some(conflict) = self.conflicts.iter_mut().find(|c| c.id == id) {
            conflict.resolution = Some(resolution.to_string());
        }
        self.conflicts.retain(|c| c.resolution.is_none());
    }
}
