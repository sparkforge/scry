use super::{HealthStatus, StatusResult};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
pub struct RemoteSiteStatus {
    pub site: String,
    pub display_name: String,
    pub location: String,
    pub system: RemoteSystemInfo,
    pub network: Vec<RemoteNetworkStatus>,
    pub agents: Vec<RemoteAgentStatus>,
    pub services: Vec<RemoteServiceStatus>,
    #[serde(default)]
    pub crons: Vec<CronStatus>,
    pub uptime_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct RemoteSystemInfo {
    pub hostname: String,
    pub uptime_secs: u64,
    pub cpu_count: usize,
    pub ram_total_gb: u64,
    pub ram_used_gb: u64,
    pub disk_total_gb: u64,
    pub disk_used_gb: u64,
}

#[derive(Debug, Deserialize)]
pub struct RemoteNetworkStatus {
    pub label: String,
    pub host: String,
    pub status: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteAgentStatus {
    pub name: String,
    pub health_url: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoteServiceStatus {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CronStatus {
    pub name: String,
    pub schedule: String,
    pub last_run: String,
    pub next_run: String,
    pub status: String,
}

pub async fn fetch_remote_status(agent_url: &str, api_key: Option<&str>) -> Result<RemoteSiteStatus> {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .build()?;

    let mut request = client.get(agent_url);

    if let Some(key) = api_key {
        if !key.is_empty() {
            request = request.header("X-Api-Key", key);
        }
    }

    let response = request
        .send()
        .await
        .context("Failed to connect to scryd agent")?;

    if !response.status().is_success() {
        anyhow::bail!("Agent returned status: {}", response.status());
    }

    let status: RemoteSiteStatus = response
        .json()
        .await
        .context("Failed to parse agent response")?;

    Ok(status)
}

pub fn convert_to_status_results(remote: &RemoteSiteStatus) -> Vec<StatusResult> {
    let mut results = Vec::new();

    // System info as a server entry
    let ram_pct = (remote.system.ram_used_gb as f64 / remote.system.ram_total_gb as f64 * 100.0) as u32;
    let disk_pct = (remote.system.disk_used_gb as f64 / remote.system.disk_total_gb as f64 * 100.0) as u32;
    let uptime_days = remote.system.uptime_secs / 86400;

    results.push(StatusResult {
        category: "server".to_string(),
        label: format!("{} ({}c, {}GB RAM)", remote.system.hostname, remote.system.cpu_count, remote.system.ram_total_gb),
        status: HealthStatus::Online,
        details: Some(format!("RAM {}%, Disk {}%, Up {}d", ram_pct, disk_pct, uptime_days)),
    });

    // Network hosts
    for net in &remote.network {
        let status = if net.status == "online" {
            HealthStatus::Online
        } else {
            HealthStatus::Offline
        };

        let details = net.latency_ms.map(|ms| format!("{}ms", ms));

        results.push(StatusResult {
            category: "network".to_string(),
            label: net.label.clone(),
            status,
            details,
        });
    }

    // Agents
    for agent in &remote.agents {
        let status = if agent.status == "running" {
            HealthStatus::Online
        } else {
            HealthStatus::Offline
        };

        results.push(StatusResult {
            category: "agents".to_string(),
            label: agent.name.clone(),
            status,
            details: None,
        });
    }

    // Services
    for service in &remote.services {
        let status = if service.status == "active" {
            HealthStatus::Online
        } else {
            HealthStatus::Offline
        };

        results.push(StatusResult {
            category: "services".to_string(),
            label: service.name.clone(),
            status,
            details: None,
        });
    }

    // Crons
    for cron in &remote.crons {
        let status = match cron.status.as_str() {
            "ok" => HealthStatus::Online,
            "error" => HealthStatus::Offline,
            _ => HealthStatus::Unknown, // idle
        };
        let label = if cron.name.len() > 38 {
            format!("{}...", &cron.name[..35])
        } else {
            cron.name.clone()
        };
        results.push(StatusResult {
            category: "crons".to_string(),
            label,
            status,
            details: Some(format!("last: {} | next: {}", cron.last_run, cron.next_run)),
        });
    }

    results
}
