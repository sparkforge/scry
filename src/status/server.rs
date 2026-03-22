use crate::config::ServerConfig;
use crate::status::{HealthStatus, StatusResult, CHECK_TIMEOUT};
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn check_server(server: &ServerConfig) -> StatusResult {
    let status = match &server.health_url {
        Some(url) => check_http_health(url).await,
        None => check_host_connectivity(&server.host).await,
    };

    // Build the label with specs
    let mut specs = Vec::new();
    if let Some(ram) = server.ram_gb {
        specs.push(format!("{}GB", ram));
    }
    if let Some(storage) = &server.storage {
        specs.push(storage.clone());
    }

    let label = if specs.is_empty() {
        server.label.clone()
    } else {
        format!("{} {}", server.label, specs.join("/"))
    };

    StatusResult {
        category: "server".to_string(),
        label,
        status,
        details: None,
    }
}

pub async fn check_servers(servers: &[ServerConfig]) -> Vec<StatusResult> {
    let mut results = Vec::new();
    for server in servers {
        results.push(check_server(server).await);
    }
    results
}

async fn check_http_health(url: &str) -> HealthStatus {
    let client = reqwest::Client::builder()
        .timeout(CHECK_TIMEOUT)
        .danger_accept_invalid_certs(true)
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => return HealthStatus::Unknown,
    };

    match client.get(url).send().await {
        Ok(response) if response.status().is_success() => HealthStatus::Online,
        Ok(_) => HealthStatus::Degraded,
        Err(_) => HealthStatus::Offline,
    }
}

async fn check_host_connectivity(host: &str) -> HealthStatus {
    // Try TCP connect to common ports
    for port in [22, 443, 80, 8080] {
        let addr_str = format!("{}:{}", host, port);
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            for addr in addrs {
                if try_tcp_connect(addr).await {
                    return HealthStatus::Online;
                }
            }
        }
    }

    HealthStatus::Offline
}

async fn try_tcp_connect(addr: SocketAddr) -> bool {
    timeout(CHECK_TIMEOUT, TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}
