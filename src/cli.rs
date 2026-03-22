use clap::{Parser, Subcommand};

/// SparkForge CLI - Monitor and manage SparkForge client sites
#[derive(Parser, Debug)]
#[command(name = "forge")]
#[command(author = "SparkForge <hello@sparkforge.io>")]
#[command(version)]
#[command(about = "Monitor and manage SparkForge managed client sites", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Display status of all components at a site
    Status {
        /// Site name to check (reads from FORGE_SITE env var if not provided)
        #[arg(short, long, env = "FORGE_SITE")]
        site: Option<String>,

        /// Watch mode - refresh every 30 seconds
        #[arg(short, long)]
        watch: bool,
    },

    /// List all configured sites
    Sites,

    /// Add a new site configuration
    #[command(subcommand)]
    Site(SiteCommands),

    /// Show detailed agent status
    Agents {
        /// Site name to check
        #[arg(short, long, env = "FORGE_SITE")]
        site: Option<String>,
    },

    /// Quick connectivity check with latency
    Ping {
        /// Host to ping
        host: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SiteCommands {
    /// Add a new site configuration interactively
    Add,
}
