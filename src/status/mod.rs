pub mod network;
pub mod server;
pub mod agents;
pub mod monitor;

use std::time::Duration;

pub const CHECK_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Online,
    Offline,
    Degraded,
    Unknown,
}


#[derive(Debug, Clone)]
pub struct StatusResult {
    pub category: String,
    pub label: String,
    pub status: HealthStatus,
    pub details: Option<String>,
}
