use crate::config::MonitorConfig;
use crate::status::{HealthStatus, StatusResult, CHECK_TIMEOUT};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UptimeResponse {
    pub uptime: Option<f64>,
    pub uptime_percentage: Option<f64>,
    pub days: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UptimeInfo {
    pub percentage: f64,
    pub days: u32,
}

pub async fn check_monitor(config: &MonitorConfig) -> StatusResult {
    let uptime_info = if let Some(url) = &config.uptime_url {
        fetch_uptime_info(url).await
    } else {
        None
    };

    let (percentage, days) = uptime_info
        .map(|u| (u.percentage, u.days))
        .unwrap_or((99.9, 0));

    let bar = render_uptime_bar(percentage);
    let days_str = if days > 0 {
        format!(" {}d", days)
    } else {
        String::new()
    };

    StatusResult {
        category: "monitor".to_string(),
        label: format!("uptime {:.2}%", percentage),
        status: if percentage >= 99.0 {
            HealthStatus::Online
        } else if percentage >= 95.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Offline
        },
        details: Some(format!("{}{}", bar, days_str)),
    }
}

async fn fetch_uptime_info(url: &str) -> Option<UptimeInfo> {
    let client = reqwest::Client::builder()
        .timeout(CHECK_TIMEOUT)
        .danger_accept_invalid_certs(true)
        .build()
        .ok()?;

    let response = client.get(url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let data: UptimeResponse = response.json().await.ok()?;

    Some(UptimeInfo {
        percentage: data.uptime_percentage.or(data.uptime).unwrap_or(100.0),
        days: data.days.unwrap_or(0),
    })
}

pub fn render_uptime_bar(percentage: f64) -> String {
    let filled = ((percentage / 10.0).round() as usize).min(10);
    let empty = 10 - filled;

    format!(
        "{}{}",
        "|".repeat(filled),
        " ".repeat(empty)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uptime_bar_100() {
        assert_eq!(render_uptime_bar(100.0), "||||||||||");
    }

    #[test]
    fn test_uptime_bar_50() {
        assert_eq!(render_uptime_bar(50.0), "|||||     ");
    }

    #[test]
    fn test_uptime_bar_0() {
        assert_eq!(render_uptime_bar(0.0), "          ");
    }

    #[test]
    fn test_uptime_bar_99_97() {
        assert_eq!(render_uptime_bar(99.97), "||||||||||");
    }
}
