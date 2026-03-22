use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SiteConfig {
    pub site: SiteInfo,
    pub network: Option<NetworkConfig>,
    pub servers: Option<Vec<ServerConfig>>,
    pub agents: Option<Vec<AgentConfig>>,
    pub monitor: Option<MonitorConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SiteInfo {
    pub name: String,
    pub display_name: String,
    pub location: Option<String>,
    pub agent_url: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkConfig {
    pub switches: Option<Vec<SwitchConfig>>,
    pub access_points: Option<Vec<AccessPointConfig>>,
    pub vlans: Option<Vec<VlanConfig>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SwitchConfig {
    pub host: String,
    pub label: String,
    #[serde(rename = "type", default = "default_check_type")]
    pub check_type: String,
    pub health_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccessPointConfig {
    pub count: u32,
    pub label: String,
    pub hosts: Option<Vec<String>>,
    pub health_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VlanConfig {
    pub names: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub label: String,
    pub ram_gb: Option<u32>,
    pub storage: Option<String>,
    pub health_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub health_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MonitorConfig {
    pub uptime_url: Option<String>,
}

fn default_check_type() -> String {
    "ping".to_string()
}

pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("scry")
        .join("sites");
    Ok(config_dir)
}

pub fn load_site_config(site_name: &str) -> Result<SiteConfig> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join(format!("{}.toml", site_name));

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Could not read site config: {}", config_path.display()))?;

    let config: SiteConfig = toml::from_str(&content)
        .with_context(|| format!("Could not parse site config: {}", config_path.display()))?;

    Ok(config)
}

pub fn list_sites() -> Result<Vec<String>> {
    let config_dir = get_config_dir()?;

    if !config_dir.exists() {
        return Ok(vec![]);
    }

    let mut sites = Vec::new();
    for entry in std::fs::read_dir(&config_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "toml") {
            if let Some(name) = path.file_stem() {
                sites.push(name.to_string_lossy().to_string());
            }
        }
    }

    sites.sort();
    Ok(sites)
}

pub fn save_site_config(config: &SiteConfig) -> Result<()> {
    let config_dir = get_config_dir()?;
    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join(format!("{}.toml", config.site.name));
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, content)?;

    Ok(())
}

pub fn get_default_site() -> Option<String> {
    std::env::var("SCRY_SITE").ok()
}
