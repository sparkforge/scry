use crate::config::{AccessPointConfig, NetworkConfig, SwitchConfig, VlanConfig};
use crate::status::{HealthStatus, StatusResult, CHECK_TIMEOUT};
use anyhow::Result;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn check_switch(switch: &SwitchConfig) -> StatusResult {
    let status = match &switch.health_url {
        Some(url) => check_http_health(url).await,
        None => check_host_connectivity(&switch.host).await,
    };

    StatusResult {
        category: "network".to_string(),
        label: switch.label.clone(),
        status,
        details: None,
    }
}

pub async fn check_access_points(ap_config: &AccessPointConfig) -> StatusResult {
    let mut online_count = 0;
    let total_to_check = ap_config.hosts.as_ref().map_or(0, |h| h.len());

    if let Some(hosts) = &ap_config.hosts {
        for host in hosts {
            if check_host_connectivity(host).await == HealthStatus::Online {
                online_count += 1;
            }
        }
    }

    let status = if total_to_check == 0 {
        // No hosts to check, assume online based on config presence
        HealthStatus::Online
    } else if online_count == total_to_check {
        HealthStatus::Online
    } else if online_count > 0 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Offline
    };

    StatusResult {
        category: "network".to_string(),
        label: format!("{}x {}", ap_config.count, ap_config.label),
        status,
        details: None,
    }
}

pub async fn check_vlans(vlan_config: &VlanConfig) -> StatusResult {
    // VLANs are configuration-based, we report them as configured
    let vlan_names = vlan_config.names.join("/");

    StatusResult {
        category: "network".to_string(),
        label: format!("VLANs: {}", vlan_names),
        status: HealthStatus::Online,
        details: Some("SEGMENTED".to_string()),
    }
}

pub async fn check_network(config: &NetworkConfig) -> Vec<StatusResult> {
    let mut results = Vec::new();

    // Check switches
    if let Some(switches) = &config.switches {
        for switch in switches {
            results.push(check_switch(switch).await);
        }
    }

    // Check access points
    if let Some(aps) = &config.access_points {
        for ap in aps {
            results.push(check_access_points(ap).await);
        }
    }

    // Check VLANs
    if let Some(vlans) = &config.vlans {
        for vlan in vlans {
            results.push(check_vlans(vlan).await);
        }
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
    // Try TCP connect to common ports (80, 443)
    for port in [443, 80] {
        let addr_str = format!("{}:{}", host, port);
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            for addr in addrs {
                if try_tcp_connect(addr).await {
                    return HealthStatus::Online;
                }
            }
        }
    }

    // Fall back to checking if we can resolve the host at all
    if format!("{}:80", host).to_socket_addrs().is_ok() {
        HealthStatus::Degraded
    } else {
        HealthStatus::Offline
    }
}

async fn try_tcp_connect(addr: SocketAddr) -> bool {
    timeout(CHECK_TIMEOUT, TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

pub async fn ping_host(host: &str) -> Result<Option<std::time::Duration>> {
    use std::net::IpAddr;

    // Parse the host as an IP address, or resolve it
    let ip: IpAddr = if let Ok(ip) = host.parse() {
        ip
    } else {
        // Try to resolve the hostname
        let addr_str = format!("{}:80", host);
        match addr_str.to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    addr.ip()
                } else {
                    return Ok(None);
                }
            }
            Err(_) => return Ok(None),
        }
    };

    // Use surge-ping for ICMP
    let client = surge_ping::Client::new(&surge_ping::Config::default())?;
    let mut pinger = client.pinger(ip, surge_ping::PingIdentifier(rand_id())).await;
    pinger.timeout(CHECK_TIMEOUT);

    let payload = [0u8; 56];
    match pinger.ping(surge_ping::PingSequence(0), &payload).await {
        Ok((_, duration)) => Ok(Some(duration)),
        Err(_) => Ok(None),
    }
}

fn rand_id() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_nanos() % 65535) as u16
}
