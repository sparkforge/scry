use crate::status::{HealthStatus, StatusResult};
use crate::status::agents::AgentDetailedStatus;
use colored::Colorize;

const LABEL_WIDTH: usize = 10;

pub fn render_status_results(results: &[StatusResult]) {
    for result in results {
        render_status_line(result);
    }
}

pub fn render_status_line(result: &StatusResult) {
    let category_colored = match result.category.as_str() {
        "network" => format!("[{}]", result.category).cyan(),
        "server" => format!("[{}]", result.category).blue(),
        "agents" => format!("[{}]", result.category).green(),
        "monitor" => format!("[{}]", result.category).yellow(),
        _ => format!("[{}]", result.category).white(),
    };

    // Pad the category to align
    let category_str = format!("{:width$}", category_colored, width = LABEL_WIDTH);

    let status_str = match result.status {
        HealthStatus::Online => {
            if result.category == "agents" {
                "RUNNING".bright_green().bold()
            } else {
                "ONLINE".bright_green().bold()
            }
        }
        HealthStatus::Offline => "OFFLINE".bright_red().bold(),
        HealthStatus::Degraded => "DEGRADED".yellow().bold(),
        HealthStatus::Unknown => "UNKNOWN".white().dimmed(),
    };

    let details_str = result
        .details
        .as_ref()
        .map(|d| format!(" {}", d))
        .unwrap_or_default();

    println!("{} {} {}{}", category_str, result.label, status_str, details_str);
}

pub fn render_site_header(site_name: &str, display_name: &str, location: Option<&str>) {
    println!();
    let header = format!("{} ({})", display_name, site_name);
    println!("{}", header.bold());
    if let Some(loc) = location {
        println!("{}", loc.dimmed());
    }
    println!();
}

pub fn render_sites_list(sites: &[(String, bool)]) {
    println!();
    println!("{}", "Configured Sites".bold());
    println!("{}", "─".repeat(40));

    if sites.is_empty() {
        println!("{}", "No sites configured.".dimmed());
        println!();
        println!("Add a site with: {}", "forge site add".cyan());
    } else {
        for (site, is_healthy) in sites {
            let indicator = if *is_healthy {
                "●".bright_green()
            } else {
                "●".bright_red()
            };
            println!("  {} {}", indicator, site);
        }
    }
    println!();
}

pub fn render_agents_detailed(agents: &[AgentDetailedStatus], site_name: &str) {
    println!();
    println!("{} - {}", "Agent Status".bold(), site_name.cyan());
    println!("{}", "─".repeat(60));
    println!();

    if agents.is_empty() {
        println!("{}", "No agents configured for this site.".dimmed());
        println!();
        return;
    }

    for agent in agents {
        let status_str = match agent.status {
            HealthStatus::Online => "RUNNING".bright_green().bold(),
            HealthStatus::Offline => "STOPPED".bright_red().bold(),
            HealthStatus::Degraded => "WARNING".yellow().bold(),
            HealthStatus::Unknown => "UNKNOWN".white().dimmed(),
        };

        println!("  {} {}", "●".bright_green(), agent.name.bold());
        println!("    Status:    {}", status_str);
        println!("    Endpoint:  {}", agent.health_url.dimmed());

        if let Some(last_run) = &agent.last_run {
            println!("    Last Run:  {}", last_run);
        }

        if let Some(error_count) = agent.error_count {
            let error_str = if error_count == 0 {
                "0".green().to_string()
            } else {
                error_count.to_string().red().to_string()
            };
            println!("    Errors:    {}", error_str);
        }

        println!();
    }
}

pub fn render_ping_result(host: &str, latency: Option<std::time::Duration>) {
    match latency {
        Some(duration) => {
            let ms = duration.as_secs_f64() * 1000.0;
            let latency_colored = if ms < 50.0 {
                format!("{:.2}ms", ms).bright_green()
            } else if ms < 200.0 {
                format!("{:.2}ms", ms).yellow()
            } else {
                format!("{:.2}ms", ms).bright_red()
            };
            println!(
                "{} {} is {} ({})",
                "●".bright_green(),
                host,
                "reachable".bright_green(),
                latency_colored
            );
        }
        None => {
            println!(
                "{} {} is {}",
                "●".bright_red(),
                host,
                "unreachable".bright_red()
            );
        }
    }
}

pub fn render_watch_header() {
    print!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
}

pub fn render_error(message: &str) {
    eprintln!("{} {}", "Error:".bright_red().bold(), message);
}

pub fn render_success(message: &str) {
    println!("{} {}", "✓".bright_green(), message);
}
