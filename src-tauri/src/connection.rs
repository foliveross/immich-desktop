use anyhow::{anyhow, Context, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub success: bool,
    pub status_code: u16,
    pub server_version: Option<String>,
    pub message: String,
}

pub fn normalize_api_url(input: &str) -> String {
    let mut url = input.trim().to_string();
    if url.is_empty() {
        return url;
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        url = format!("http://{url}");
    }
    url = url.trim_end_matches('/').to_string();
    if !url.ends_with("/api") {
        url.push_str("/api");
    }
    url
}

pub fn web_url_from_api(api_url: &str) -> String {
    api_url.trim_end_matches("/api").trim_end_matches('/').to_string()
}

pub fn probe_server_reachable(api_url: &str) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .context("Failed to build HTTP client")?;

    for path in ping_paths(api_url) {
        if let Ok(resp) = client.get(&path).send() {
            if resp.status().is_success() || resp.status() == StatusCode::UNAUTHORIZED {
                return Ok(());
            }
        }
    }

    Err(anyhow!("No Immich server responded at {api_url}"))
}

pub async fn handshake(server_url: &str, api_key: &str) -> Result<HandshakeResult> {
    let api_url = normalize_api_url(server_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .context("Failed to build HTTP client")?;

    let mut last_status = 0u16;
    let mut last_message = String::from("Could not reach Immich server");

    for path in ping_paths(&api_url) {
        let resp = client
            .get(&path)
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .send()
            .await;

        match resp {
            Ok(response) => {
                last_status = response.status().as_u16();
                if response.status().is_success() || response.status().as_u16() == 204 {
                    let version = fetch_version(&client, &api_url, api_key).await;
                    return Ok(HandshakeResult {
                        success: true,
                        status_code: last_status,
                        server_version: version,
                        message: "Handshake successful".to_string(),
                    });
                }
                last_message = format!(
                    "Server responded with HTTP {last_status} at {path}"
                );
            }
            Err(e) => {
                last_message = format!("Connection failed at {path}: {e}");
            }
        }
    }

    Ok(HandshakeResult {
        success: false,
        status_code: last_status,
        server_version: None,
        message: last_message,
    })
}

async fn fetch_version(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
) -> Option<String> {
    let paths = [
        format!("{api_url}/server/version"),
        format!("{api_url}/server-info/version"),
    ];

    for path in paths {
        if let Ok(resp) = client
            .get(&path)
            .header("x-api-key", api_key)
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(v) = json.get("major").and_then(|m| m.as_i64()) {
                            let minor = json.get("minor").and_then(|m| m.as_i64()).unwrap_or(0);
                            return Some(format!("{v}.{minor}"));
                        }
                        return Some(body.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

fn ping_paths(api_url: &str) -> Vec<String> {
    let base = api_url.trim_end_matches('/');
    vec![
        format!("{base}/server/ping"),
        format!("{base}/server-info/ping"),
    ]
}
