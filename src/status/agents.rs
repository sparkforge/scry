use crate::config::AgentConfig;
use crate::status::{HealthStatus, StatusResult, CHECK_TIMEOUT};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AgentHealthResponse {
    pub status: Option<String>,
    pub last_run: Option<String>,
    pub error_count: Option<u32>,
    pub uptime: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentDetailedStatus {
    pub name: String,
    pub status: HealthStatus,
    pub last_run: Option<String>,
    pub error_count: Option<u32>,
    pub health_url: String,
}

pub async fn check_agent(agent: &AgentConfig) -> StatusResult {
    let status = check_agent_health(&agent.health_url).await;

    StatusResult {
        category: "agents".to_string(),
        label: agent.name.clone(),
        status: if status == HealthStatus::Online {
            HealthStatus::Online // Will be displayed as "RUNNING"
        } else {
            status
        },
        details: None,
    }
}

pub async fn check_agents(agents: &[AgentConfig]) -> Vec<StatusResult> {
    let mut results = Vec::new();
    for agent in agents {
        results.push(check_agent(agent).await);
    }
    results
}

pub async fn check_agent_detailed(agent: &AgentConfig) -> AgentDetailedStatus {
    let client = reqwest::Client::builder()
        .timeout(CHECK_TIMEOUT)
        .danger_accept_invalid_certs(true)
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => {
            return AgentDetailedStatus {
                name: agent.name.clone(),
                status: HealthStatus::Unknown,
                last_run: None,
                error_count: None,
                health_url: agent.health_url.clone(),
            };
        }
    };

    match client.get(&agent.health_url).send().await {
        Ok(response) if response.status().is_success() => {
            // Try to parse detailed health info
            if let Ok(health) = response.json::<AgentHealthResponse>().await {
                AgentDetailedStatus {
                    name: agent.name.clone(),
                    status: HealthStatus::Online,
                    last_run: health.last_run,
                    error_count: health.error_count,
                    health_url: agent.health_url.clone(),
                }
            } else {
                AgentDetailedStatus {
                    name: agent.name.clone(),
                    status: HealthStatus::Online,
                    last_run: None,
                    error_count: None,
                    health_url: agent.health_url.clone(),
                }
            }
        }
        Ok(_) => AgentDetailedStatus {
            name: agent.name.clone(),
            status: HealthStatus::Degraded,
            last_run: None,
            error_count: None,
            health_url: agent.health_url.clone(),
        },
        Err(_) => AgentDetailedStatus {
            name: agent.name.clone(),
            status: HealthStatus::Offline,
            last_run: None,
            error_count: None,
            health_url: agent.health_url.clone(),
        },
    }
}

pub async fn check_agents_detailed(agents: &[AgentConfig]) -> Vec<AgentDetailedStatus> {
    let mut results = Vec::new();
    for agent in agents {
        results.push(check_agent_detailed(agent).await);
    }
    results
}

async fn check_agent_health(url: &str) -> HealthStatus {
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
