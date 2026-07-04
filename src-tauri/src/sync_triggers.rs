use crate::config::SyncTriggersConfig;
use anyhow::Result;
use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTriggerStatus {
    pub can_sync: bool,
    pub wifi_connected: bool,
    pub on_allowed_network: bool,
    pub plugged_in: bool,
    pub within_schedule: bool,
    pub reasons: Vec<String>,
}

pub fn evaluate_triggers(config: &SyncTriggersConfig) -> SyncTriggerStatus {
    let mut reasons = Vec::new();
    let wifi_connected = is_wifi_connected();
    let on_allowed_network = is_on_allowed_network(config);
    let plugged_in = is_plugged_in();
    let within_schedule = is_within_schedule(&config.schedule);

    let mut can_sync = true;

    if config.wifi_only && !wifi_connected {
        can_sync = false;
        reasons.push("Wi-Fi only mode enabled but not on Wi-Fi".to_string());
    }

    if !config.allowed_networks.is_empty() && !on_allowed_network {
        can_sync = false;
        reasons.push("Not connected to an allowed network".to_string());
    }

    if config.require_plugged_in && !plugged_in {
        can_sync = false;
        reasons.push("Device must be plugged in to sync".to_string());
    }

    if config.schedule.enabled && !within_schedule {
        can_sync = false;
        reasons.push("Outside configured sync time window".to_string());
    }

    SyncTriggerStatus {
        can_sync,
        wifi_connected,
        on_allowed_network,
        plugged_in,
        within_schedule,
        reasons,
    }
}

fn is_wifi_connected() -> bool {
    let output = Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.contains("connected") && text.contains("SSID")
        }
        Err(_) => true,
    }
}

fn is_on_allowed_network(config: &SyncTriggersConfig) -> bool {
    if config.allowed_networks.is_empty() {
        return true;
    }

    let output = Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            config
                .allowed_networks
                .iter()
                .any(|network| text.contains(network))
        }
        Err(_) => false,
    }
}

fn is_plugged_in() -> bool {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance -ClassName Win32_Battery -ErrorAction SilentlyContinue) -eq $null -or (Get-CimInstance -ClassName Win32_Battery).BatteryStatus -in 2,3,6,7,8,9",
        ])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            text == "true"
        }
        Err(_) => true,
    }
}

fn is_within_schedule(schedule: &crate::config::ScheduleConfig) -> bool {
    if !schedule.enabled {
        return true;
    }

    let now = Local::now();
    let hour = now.hour() as u8;
    let start = schedule.start_hour;
    let end = schedule.end_hour;

    if start <= end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

pub fn get_current_network_name() -> Result<Option<String>> {
    let output = Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.trim().starts_with("SSID") && !line.contains("BSSID") {
            if let Some(name) = line.split(':').nth(1) {
                return Ok(Some(name.trim().to_string()));
            }
        }
    }
    Ok(None)
}
