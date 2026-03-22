mod cli;
mod config;
mod output;
mod status;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, SiteCommands};
use config::{
    AgentConfig, MonitorConfig, NetworkConfig, SiteConfig, SiteInfo, AccessPointConfig,
    ServerConfig, SwitchConfig, VlanConfig,
};
use dialoguer::{Confirm, Input};
use futures::future::join_all;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        output::render_error(&e.to_string());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status { site, watch } => {
            cmd_status(site, watch).await?;
        }
        Commands::Sites => {
            cmd_sites().await?;
        }
        Commands::Site(SiteCommands::Add) => {
            cmd_site_add().await?;
        }
        Commands::Agents { site } => {
            cmd_agents(site).await?;
        }
        Commands::Ping { host } => {
            cmd_ping(&host).await?;
        }
    }

    Ok(())
}

async fn cmd_status(site: Option<String>, watch: bool) -> Result<()> {
    let site_name = site
        .or_else(config::get_default_site)
        .context("No site specified. Use --site or set SCRY_SITE environment variable.")?;

    loop {
        if watch {
            output::render_watch_header();
        }

        let config = config::load_site_config(&site_name)?;

        // If agent_url is configured, fetch from scryd instead of doing local checks
        if let Some(agent_url) = &config.site.agent_url {
            match status::remote::fetch_remote_status(agent_url, config.site.api_key.as_deref()).await {
                Ok(remote_status) => {
                    output::render_site_header(
                        &remote_status.site,
                        &remote_status.display_name,
                        Some(&remote_status.location),
                    );

                    let results = status::remote::convert_to_status_results(&remote_status);
                    output::render_status_results(&results);
                }
                Err(e) => {
                    output::render_site_header(
                        &config.site.name,
                        &config.site.display_name,
                        config.site.location.as_deref(),
                    );
                    output::render_error(&format!("Failed to fetch from agent: {}", e));
                }
            }
        } else {
            output::render_site_header(
                &config.site.name,
                &config.site.display_name,
                config.site.location.as_deref(),
            );

            let results = gather_all_status(&config).await;
            output::render_status_results(&results);
        }

        println!();

        if !watch {
            break;
        }

        sleep(Duration::from_secs(30)).await;
    }

    Ok(())
}

async fn gather_all_status(config: &SiteConfig) -> Vec<status::StatusResult> {
    let mut all_results = Vec::new();

    // Run all checks concurrently
    let mut futures = Vec::new();

    // Network checks
    if let Some(network) = &config.network {
        let network = network.clone();
        futures.push(tokio::spawn(async move {
            status::network::check_network(&network).await
        }));
    }

    // Server checks
    if let Some(servers) = &config.servers {
        let servers = servers.clone();
        futures.push(tokio::spawn(async move {
            status::server::check_servers(&servers).await
        }));
    }

    // Agent checks
    if let Some(agents) = &config.agents {
        let agents = agents.clone();
        futures.push(tokio::spawn(async move {
            status::agents::check_agents(&agents).await
        }));
    }

    // Monitor checks
    if let Some(monitor) = &config.monitor {
        let monitor = monitor.clone();
        futures.push(tokio::spawn(async move {
            vec![status::monitor::check_monitor(&monitor).await]
        }));
    }

    // Collect results
    let results = join_all(futures).await;
    for result in results {
        if let Ok(items) = result {
            all_results.extend(items);
        }
    }

    all_results
}

async fn cmd_sites() -> Result<()> {
    let sites = config::list_sites()?;

    // Quick health check for each site
    let mut site_health = Vec::new();
    for site_name in sites {
        let is_healthy = if let Ok(config) = config::load_site_config(&site_name) {
            // Just check if we can load the config, don't do full health checks for listing
            config.site.name == site_name
        } else {
            false
        };
        site_health.push((site_name, is_healthy));
    }

    output::render_sites_list(&site_health);
    Ok(())
}

async fn cmd_site_add() -> Result<()> {
    println!();
    println!("{}", "Add New Site Configuration");
    println!("{}", "─".repeat(40));
    println!();

    let name: String = Input::new()
        .with_prompt("Site identifier (e.g., client-04)")
        .interact_text()?;

    let display_name: String = Input::new()
        .with_prompt("Display name (e.g., Acme Corp)")
        .interact_text()?;

    let location: String = Input::new()
        .with_prompt("Location (e.g., Milwaukee, WI)")
        .allow_empty(true)
        .interact_text()?;

    let agent_url: String = Input::new()
        .with_prompt("Agent URL (e.g., http://192.168.0.94:7734/status, leave empty for local checks)")
        .allow_empty(true)
        .interact_text()?;

    let api_key: String = if !agent_url.is_empty() {
        Input::new()
            .with_prompt("Agent API key (optional)")
            .allow_empty(true)
            .interact_text()?
    } else {
        String::new()
    };

    let mut site_config = SiteConfig {
        site: SiteInfo {
            name: name.clone(),
            display_name,
            location: if location.is_empty() {
                None
            } else {
                Some(location)
            },
            agent_url: if agent_url.is_empty() {
                None
            } else {
                Some(agent_url)
            },
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            },
        },
        network: None,
        servers: None,
        agents: None,
        monitor: None,
    };

    // Network configuration
    if Confirm::new()
        .with_prompt("Add network devices?")
        .default(true)
        .interact()?
    {
        let mut network_config = NetworkConfig {
            switches: None,
            access_points: None,
            vlans: None,
        };

        // Switches
        if Confirm::new()
            .with_prompt("Add a switch?")
            .default(true)
            .interact()?
        {
            let switch_host: String = Input::new()
                .with_prompt("Switch IP/hostname")
                .interact_text()?;

            let switch_label: String = Input::new()
                .with_prompt("Switch label (e.g., 48-port managed switch)")
                .interact_text()?;

            network_config.switches = Some(vec![SwitchConfig {
                host: switch_host,
                label: switch_label,
                check_type: "ping".to_string(),
                health_url: None,
            }]);
        }

        // Access Points
        if Confirm::new()
            .with_prompt("Add access points?")
            .default(true)
            .interact()?
        {
            let ap_count: u32 = Input::new()
                .with_prompt("Number of access points")
                .default(1)
                .interact_text()?;

            let ap_label: String = Input::new()
                .with_prompt("AP label (e.g., enterprise APs)")
                .default("access points".to_string())
                .interact_text()?;

            network_config.access_points = Some(vec![AccessPointConfig {
                count: ap_count,
                label: ap_label,
                hosts: None,
                health_url: None,
            }]);
        }

        // VLANs
        if Confirm::new()
            .with_prompt("Add VLANs?")
            .default(false)
            .interact()?
        {
            let vlan_names: String = Input::new()
                .with_prompt("VLAN names (comma-separated, e.g., ops,guest,iot)")
                .interact_text()?;

            let names: Vec<String> = vlan_names
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !names.is_empty() {
                network_config.vlans = Some(vec![VlanConfig { names }]);
            }
        }

        site_config.network = Some(network_config);
    }

    // Server configuration
    if Confirm::new()
        .with_prompt("Add a server?")
        .default(true)
        .interact()?
    {
        let server_host: String = Input::new()
            .with_prompt("Server IP/hostname")
            .interact_text()?;

        let server_label: String = Input::new()
            .with_prompt("Server label (e.g., rack-01)")
            .interact_text()?;

        let ram_gb: u32 = Input::new()
            .with_prompt("RAM (GB)")
            .default(16)
            .interact_text()?;

        let storage: String = Input::new()
            .with_prompt("Storage (e.g., 1TB SSD)")
            .default("500GB SSD".to_string())
            .interact_text()?;

        site_config.servers = Some(vec![ServerConfig {
            host: server_host,
            label: server_label,
            ram_gb: Some(ram_gb),
            storage: Some(storage),
            health_url: None,
        }]);
    }

    // Agent configuration
    if Confirm::new()
        .with_prompt("Add AI agents?")
        .default(false)
        .interact()?
    {
        let mut agents = Vec::new();

        loop {
            let agent_name: String = Input::new()
                .with_prompt("Agent name")
                .interact_text()?;

            let health_url: String = Input::new()
                .with_prompt("Health endpoint URL")
                .interact_text()?;

            agents.push(AgentConfig {
                name: agent_name,
                health_url,
            });

            if !Confirm::new()
                .with_prompt("Add another agent?")
                .default(false)
                .interact()?
            {
                break;
            }
        }

        if !agents.is_empty() {
            site_config.agents = Some(agents);
        }
    }

    // Monitor configuration
    let uptime_url: String = Input::new()
        .with_prompt("Uptime monitor URL (optional)")
        .allow_empty(true)
        .interact_text()?;

    if !uptime_url.is_empty() {
        site_config.monitor = Some(MonitorConfig {
            uptime_url: Some(uptime_url),
        });
    }

    // Save the config
    config::save_site_config(&site_config)?;

    println!();
    output::render_success(&format!(
        "Site '{}' saved to {}",
        name,
        config::get_config_dir()?.join(format!("{}.toml", name)).display()
    ));
    println!();
    println!("Check status with: scry status --site {}", name);

    Ok(())
}

async fn cmd_agents(site: Option<String>) -> Result<()> {
    let site_name = site
        .or_else(config::get_default_site)
        .context("No site specified. Use --site or set SCRY_SITE environment variable.")?;

    let config = config::load_site_config(&site_name)?;

    let agents = config.agents.as_ref().map_or(vec![], |a| a.clone());

    let detailed_status = status::agents::check_agents_detailed(&agents).await;

    output::render_agents_detailed(&detailed_status, &site_name);

    Ok(())
}

async fn cmd_ping(host: &str) -> Result<()> {
    // First try ICMP ping
    match status::network::ping_host(host).await {
        Ok(Some(duration)) => {
            output::render_ping_result(host, Some(duration));
        }
        Ok(None) | Err(_) => {
            // Fall back to TCP connectivity check
            use std::net::ToSocketAddrs;
            use std::time::Instant;
            use tokio::net::TcpStream;
            use tokio::time::timeout;

            let start = Instant::now();
            let mut connected = false;

            for port in [443, 80, 22] {
                let addr_str = format!("{}:{}", host, port);
                if let Ok(addrs) = addr_str.to_socket_addrs() {
                    for addr in addrs {
                        if timeout(Duration::from_secs(3), TcpStream::connect(addr))
                            .await
                            .map(|r| r.is_ok())
                            .unwrap_or(false)
                        {
                            connected = true;
                            break;
                        }
                    }
                }
                if connected {
                    break;
                }
            }

            if connected {
                output::render_ping_result(host, Some(start.elapsed()));
            } else {
                output::render_ping_result(host, None);
            }
        }
    }

    Ok(())
}
