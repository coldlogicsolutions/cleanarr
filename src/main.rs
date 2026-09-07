mod commands;
mod config;
mod output;
mod plex;
mod sonarr;

use anyhow::Result;
use clap::parser::ValueSource;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::Shell;
use radarr_api::RadarrClient;

/// Whether `id` was given as an actual command-line flag (as opposed to being
/// filled in from an `env = ...` fallback or a default).
///
/// Global args are propagated to the parent matches by clap, but the search
/// recurses into subcommand matches so subcommand-local args are found too.
fn from_command_line(matches: &ArgMatches, id: &str) -> bool {
    if matches!(matches.try_contains_id(id), Ok(true))
        && matches.value_source(id) == Some(ValueSource::CommandLine)
    {
        return true;
    }
    match matches.subcommand() {
        Some((_, sub)) => from_command_line(sub, id),
        None => false,
    }
}

/// `Some(value)` only when `id` came from the command line — used by `init` so
/// environment-derived secrets are never persisted to the config file.
fn flag_only(matches: &ArgMatches, id: &str, value: &Option<String>) -> Option<String> {
    value.clone().filter(|_| from_command_line(matches, id))
}

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

    /// Path to config file (default: $XDG_CONFIG_HOME/cleanarr/config.toml,
    /// else ~/.config/cleanarr/config.toml)
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
    // Parse via ArgMatches so `init` can tell explicit flags from env fallbacks.
    let matches = Cli::command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

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
            // Only values typed as flags are persisted; env vars stay runtime-only
            // so `init` never writes secrets the user did not ask it to save.
            radarr_url: flag_only(&matches, "radarr_url", &cli.radarr_url),
            radarr_api_key: flag_only(&matches, "radarr_api_key", &cli.radarr_api_key),
            plex_url: flag_only(&matches, "plex_url", &cli.plex_url),
            plex_token: flag_only(&matches, "plex_token", &cli.plex_token),
            plex_client_id: None,
            sonarr_url: flag_only(&matches, "sonarr_url", &cli.sonarr_url),
            sonarr_api_key: flag_only(&matches, "sonarr_api_key", &cli.sonarr_api_key),
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

    let fmt = cfg.output_format;

    // Radarr is optional (Plex-only setups), so build the client only where needed.
    match cli.command {
        Commands::Plex { command } => {
            let api = match &cfg.radarr {
                Some(r) => Some(RadarrClient::new(&r.url, &r.api_key)?),
                None => None,
            };
            commands::plex::run(command, api.as_ref(), &cfg.plex, fmt).await
        }
        Commands::Cleanup { args } => {
            let radarr = cfg.require_radarr()?;
            let api = RadarrClient::new(&radarr.url, &radarr.api_key)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_only_keeps_command_line_values_and_drops_env_ones() {
        // Global flag given after the subcommand.
        let m = Cli::command()
            .try_get_matches_from(["cleanarr", "init", "--radarr-url", "http://x:7878"])
            .unwrap();
        assert!(from_command_line(&m, "radarr_url"));
        assert_eq!(
            flag_only(&m, "radarr_url", &Some("http://x:7878".into())),
            Some("http://x:7878".into())
        );

        // Not given as a flag: whatever clap filled in came from an env var,
        // so `init` must not persist it.
        assert!(!from_command_line(&m, "radarr_api_key"));
        assert_eq!(
            flag_only(&m, "radarr_api_key", &Some("SECRET".into())),
            None
        );

        // A global flag given before the subcommand is found too.
        let m = Cli::command()
            .try_get_matches_from(["cleanarr", "--plex-token", "T", "init"])
            .unwrap();
        assert!(from_command_line(&m, "plex_token"));
    }
}
