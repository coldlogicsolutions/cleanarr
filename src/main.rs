mod commands;
mod config;
mod output;
mod plex;
mod sonarr;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use radarr_api::RadarrClient;

#[derive(Parser)]
#[command(
    name = "cleanarr",
    version,
    about = "Media library cleanup for Radarr, Sonarr and Plex"
)]
pub struct Cli {
    /// Radarr instance URL (e.g. http://localhost:7878)
    #[arg(long, global = true, env = "RADARR_URL")]
    radarr_url: Option<String>,

    /// Radarr API key
    #[arg(long, global = true, env = "RADARR_API_KEY")]
    radarr_api_key: Option<String>,

    /// Output format: table, json, quiet
    #[arg(long, short, global = true)]
    output: Option<String>,

    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,

    /// Plex server URL (e.g. http://localhost:32400)
    #[arg(long, global = true, env = "PLEX_URL")]
    plex_url: Option<String>,

    /// Plex authentication token
    #[arg(long, global = true, env = "PLEX_TOKEN")]
    plex_token: Option<String>,

    /// Sonarr instance URL (e.g. http://localhost:8989)
    #[arg(long, global = true, env = "SONARR_URL")]
    sonarr_url: Option<String>,

    /// Sonarr API key
    #[arg(long, global = true, env = "SONARR_API_KEY")]
    sonarr_api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Plex media server integration
    Plex {
        #[command(subcommand)]
        command: commands::plex::PlexCommand,
    },
    /// Disk cleanup — delete movies and TV episodes to free space
    Cleanup {
        #[command(flatten)]
        args: commands::cleanup::CleanupArgs,
    },
    /// Save Radarr, Sonarr, Plex and cleanup settings to the config file
    Init {
        /// Comma-separated tag names cleanup must never delete (replaces the stored list)
        #[arg(long, value_delimiter = ',')]
        protected_tags: Option<Vec<String>>,
        /// Default cleanup size threshold (e.g. "4TB")
        #[arg(long)]
        default_max_size: Option<String>,
        /// Default cleanup strategy
        #[arg(long)]
        default_strategy: Option<String>,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = <Cli as clap::CommandFactory>::command();
        clap_complete::generate(*shell, &mut cmd, "cleanarr", &mut std::io::stdout());
        return Ok(());
    }

    // Handle commands that don't require a valid Radarr config
    if let Commands::Init {
        protected_tags,
        default_max_size,
        default_strategy,
    } = &cli.command
    {
        let updates = config::ConfigUpdates {
            default_max_size: default_max_size.clone(),
            default_strategy: default_strategy.clone(),
            protected_tags: protected_tags.as_ref().map(|t| {
                t.iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }),
            radarr_url: cli.radarr_url.clone(),
            radarr_api_key: cli.radarr_api_key.clone(),
            plex_url: cli.plex_url.clone(),
            plex_token: cli.plex_token.clone(),
            plex_client_id: None,
            sonarr_url: cli.sonarr_url.clone(),
            sonarr_api_key: cli.sonarr_api_key.clone(),
        };
        if updates.is_empty() {
            anyhow::bail!(
                "Provide settings to save: --radarr-url, --radarr-api-key, --plex-url, --plex-token, --sonarr-url, --sonarr-api-key, --protected-tags, --default-max-size, --default-strategy\n\
                 Example: cleanarr init --radarr-url http://localhost:7878 --radarr-api-key YOUR_KEY"
            );
        }
        let path = config::save_to_config(cli.config.as_deref(), updates)?;
        eprintln!("Configuration saved to {}", path.display());
        return Ok(());
    }

    if matches!(
        &cli.command,
        Commands::Plex {
            command: commands::plex::PlexCommand::Login
        }
    ) {
        commands::plex::login(cli.plex_url.as_deref(), cli.config.as_deref()).await?;
        return Ok(());
    }

    let cfg = config::AppConfig::load(
        cli.radarr_url.as_deref(),
        cli.radarr_api_key.as_deref(),
        cli.output.as_deref(),
        cli.config.as_deref(),
        cli.plex_url.as_deref(),
        cli.plex_token.as_deref(),
        cli.sonarr_url.as_deref(),
        cli.sonarr_api_key.as_deref(),
    )?;

    let api = RadarrClient::new(&cfg.url, &cfg.api_key)?;
    let fmt = cfg.output_format;

    match cli.command {
        Commands::Plex { command } => commands::plex::run(command, &api, &cfg.plex, fmt).await,
        Commands::Cleanup { args } => {
            commands::cleanup::run(
                args,
                &api,
                &cfg.plex,
                &cfg.sonarr,
                &cfg.protected_tags,
                &cfg.cleanup_defaults,
                fmt,
            )
            .await
        }
        Commands::Init { .. } => unreachable!(),
        Commands::Completions { .. } => unreachable!(),
    }
}
