use crate::connection;
use crate::process_manager;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{Ipv4Addr, TcpStream};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredServer {
    pub name: String,
    pub url: String,
    pub source: String,
}

const COMMON_PORTS: [u16; 5] = [2283, 3001, 8080, 80, 443];
const MDNS_TIMEOUT: Duration = Duration::from_secs(3);
const PROBE_TIMEOUT: Duration = Duration::from_millis(600);
const SUBNET_SCAN_BUDGET: Duration = Duration::from_secs(8);

pub async fn discover_servers() -> Vec<DiscoveredServer> {
    let mut found: Vec<DiscoveredServer> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    let mdns = tokio::task::spawn_blocking(discover_via_mdns).await;
    if let Ok(servers) = mdns {
        for server in servers {
            if seen_urls.insert(server.url.clone()) {
                found.push(server);
            }
        }
    }

    // Skip expensive subnet scan when mDNS already found servers
    if found.is_empty() {
        let subnet = tokio::time::timeout(
            SUBNET_SCAN_BUDGET,
            tokio::task::spawn_blocking(scan_local_subnet),
        )
        .await;

        if let Ok(Ok(servers)) = subnet {
            for server in servers {
                if seen_urls.insert(server.url.clone()) {
                    found.push(server);
                }
            }
        }
    }

    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn discover_via_mdns() -> Vec<DiscoveredServer> {
    let mut results = Vec::new();
    let Ok(mdns) = mdns_sd::ServiceDaemon::new() else {
        return results;
    };

    let service_types = [
        "_immich._tcp.local.",
        "_http._tcp.local.",
        "_https._tcp.local.",
    ];

    let deadline = Instant::now() + MDNS_TIMEOUT;

    for service_type in service_types {
        let Ok(receiver) = mdns.browse(service_type) else {
            continue;
        };

        while Instant::now() < deadline {
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                    let host = info.get_hostname().trim_end_matches('.').to_string();
                    let port = info.get_port();
                    let scheme = if service_type.starts_with("_https") {
                        "https"
                    } else {
                        "http"
                    };
                    let base = format!("{scheme}://{host}:{port}");
                    let api_url = connection::normalize_api_url(&base);

                    if connection::probe_server_reachable(&api_url).is_ok() {
                        let name = info
                            .get_fullname()
                            .split('.')
                            .next()
                            .unwrap_or("Immich Server")
                            .to_string();
                        results.push(DiscoveredServer {
                            name,
                            url: api_url,
                            source: "mDNS".to_string(),
                        });
                    }
                }
                Ok(mdns_sd::ServiceEvent::SearchStarted(_)) => {}
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let _ = mdns.stop_browse(service_type);
    }

    results
}

fn scan_local_subnet() -> Vec<DiscoveredServer> {
    let results: Arc<StdMutex<Vec<DiscoveredServer>>> = Arc::new(StdMutex::new(Vec::new()));
    let Some(local_ip) = local_ipv4() else {
        return Vec::new();
    };

    let octets = local_ip.octets();
    let prefix = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
    let deadline = Instant::now() + SUBNET_SCAN_BUDGET;

    let handles: Vec<_> = (1u8..=254u8)
        .map(|host| {
            let ip = format!("{prefix}.{host}");
            let results = Arc::clone(&results);
            std::thread::spawn(move || {
                if Instant::now() >= deadline {
                    return;
                }
                for port in COMMON_PORTS {
                    if !is_port_open(&ip, port) {
                        continue;
                    }
                    let scheme = if port == 443 { "https" } else { "http" };
                    let base = format!("{scheme}://{ip}:{port}");
                    let api_url = connection::normalize_api_url(&base);
                    if connection::probe_server_reachable(&api_url).is_ok() {
                        results.lock().unwrap().push(DiscoveredServer {
                            name: format!("Immich @ {ip}:{port}"),
                            url: api_url,
                            source: "Network Scan".to_string(),
                        });
                        break;
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.join();
    }

    Arc::try_unwrap(results)
        .ok()
        .and_then(|m| m.into_inner().ok())
        .unwrap_or_default()
}

fn local_ipv4() -> Option<Ipv4Addr> {
    let output = process_manager::hidden_command("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } | Select-Object -First 1 -ExpandProperty IPAddress)",
        ])
        .output()
        .ok()?;

    let ip_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    ip_str.parse().ok()
}

fn is_port_open(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let target = format!("{host}:{port}");
    if let Ok(mut addrs) = target.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            return TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).is_ok();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_integration() {
        let url = connection::normalize_api_url("http://192.168.1.10:2283");
        assert!(url.ends_with("/api"));
    }
}
