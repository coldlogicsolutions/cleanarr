mod commands;
mod config;
mod output;

use anyhow::Result;
use clap::parser::ValueSource;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::Shell;
use radarr_api::RadarrClient;

/// Whether `id` was given as an actual command-line flag (as opposed to being
/// filled in from an `env = ...` fallback or a default).
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
    name = "radarr-cli",
    version,
    about = "Command-line client for the Radarr v3 API"
)]
pub struct Cli {
    /// Radarr instance URL (e.g. http://localhost:7878)
    #[arg(long, visible_alias = "radarr-url", global = true, env = "RADARR_URL")]
    url: Option<String>,

    /// Radarr API key
    #[arg(
        long,
        visible_alias = "radarr-api-key",
        global = true,
        env = "RADARR_API_KEY"
    )]
    api_key: Option<String>,

    /// Output format: table, json, quiet
    #[arg(long, short, global = true, env = "RADARR_OUTPUT")]
    output: Option<String>,

    /// Path to config file (default: $XDG_CONFIG_HOME/radarr-cli/config.toml,
    /// else ~/.config/radarr-cli/config.toml)
    #[arg(long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Movie management
    Movie {
        #[command(subcommand)]
        command: commands::movie::MovieCommand,
    },
    /// Download queue management
    Queue {
        #[command(subcommand)]
        command: commands::queue::QueueCommand,
    },
    /// History browsing
    History {
        #[command(subcommand)]
        command: commands::history::HistoryCommand,
    },
    /// System information and management
    System {
        #[command(subcommand)]
        command: commands::system::SystemCommand,
    },
    /// Collection management
    Collection {
        #[command(subcommand)]
        command: commands::collection::CollectionCommand,
    },
    /// Calendar events
    Calendar {
        #[command(subcommand)]
        command: commands::calendar::CalendarCommand,
    },
    /// Radarr configuration
    Config {
        #[command(subcommand)]
        command: commands::config_cmd::ConfigCommand,
    },
    /// Quality profile management
    QualityProfile {
        #[command(subcommand)]
        command: commands::quality_profile::QualityProfileCommand,
    },
    /// Quality definition management
    QualityDefinition {
        #[command(subcommand)]
        command: commands::quality_definition::QualityDefinitionCommand,
    },
    /// Custom format management
    CustomFormat {
        #[command(subcommand)]
        command: commands::custom_format::CustomFormatCommand,
    },
    /// Delay profile management
    DelayProfile {
        #[command(subcommand)]
        command: commands::delay_profile::DelayProfileCommand,
    },
    /// Release profile management
    ReleaseProfile {
        #[command(subcommand)]
        command: commands::release_profile::ReleaseProfileCommand,
    },
    /// Indexer management
    Indexer {
        #[command(subcommand)]
        command: commands::indexer::IndexerCommand,
    },
    /// Download client management
    DownloadClient {
        #[command(subcommand)]
        command: commands::download_client::DownloadClientCommand,
    },
    /// Notification management
    Notification {
        #[command(subcommand)]
        command: commands::notification::NotificationCommand,
    },
    /// Import list management
    ImportList {
        #[command(subcommand)]
        command: commands::import_list::ImportListCommand,
    },
    /// Metadata consumer management
    Metadata {
        #[command(subcommand)]
        command: commands::metadata::MetadataCommand,
    },
    /// Tag management
    Tag {
        #[command(subcommand)]
        command: commands::tag::TagCommand,
    },
    /// Blocklist management
    Blocklist {
        #[command(subcommand)]
        command: commands::blocklist::BlocklistCommand,
    },
    /// Import list exclusion management
    Exclusion {
        #[command(subcommand)]
        command: commands::exclusion::ExclusionCommand,
    },
    /// Movie file management
    MovieFile {
        #[command(subcommand)]
        command: commands::movie_file::MovieFileCommand,
    },
    /// Release search and push
    Release {
        #[command(subcommand)]
        command: commands::release::ReleaseCommand,
    },
    /// Command execution
    Command {
        #[command(subcommand)]
        command: commands::command::CommandCommand,
    },
    /// Parse a title string
    Parse {
        #[command(flatten)]
        args: commands::parse::ParseArgs,
    },
    /// Preview rename operations
    Rename {
        #[command(flatten)]
        args: commands::rename::RenameArgs,
    },
    /// Manual import management
    ManualImport {
        #[command(subcommand)]
        command: commands::manual_import::ManualImportCommand,
    },
    /// Remote path mapping management
    RemotePathMapping {
        #[command(subcommand)]
        command: commands::remote_path_mapping::RemotePathMappingCommand,
    },
    /// Movie credits
    Credit {
        #[command(subcommand)]
        command: commands::credit::CreditCommand,
    },
    /// Alternative titles
    AltTitle {
        #[command(subcommand)]
        command: commands::alt_title::AltTitleCommand,
    },
    /// Extra files
    ExtraFile {
        #[command(subcommand)]
        command: commands::extra_file::ExtraFileCommand,
    },
    /// Auto-tagging rules
    AutoTagging {
        #[command(subcommand)]
        command: commands::auto_tagging::AutoTaggingCommand,
    },
    /// Custom filter management
    CustomFilter {
        #[command(subcommand)]
        command: commands::custom_filter::CustomFilterCommand,
    },
    /// Language information
    Language {
        #[command(subcommand)]
        command: commands::language::LanguageCommand,
    },
    /// Indexer flags
    IndexerFlag {
        #[command(subcommand)]
        command: commands::indexer_flag::IndexerFlagCommand,
    },
    /// File system browsing
    Filesystem {
        #[command(subcommand)]
        command: commands::filesystem::FilesystemCommand,
    },
    /// Save connection settings to the config file
    Init,
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
        clap_complete::generate(*shell, &mut cmd, "radarr-cli", &mut std::io::stdout());
        return Ok(());
    }

    if let Commands::Init = &cli.command {
        // Only values typed as flags are persisted; env vars stay runtime-only
        // so `init` never writes secrets the user did not ask it to save.
        let url = flag_only(&matches, "url", &cli.url);
        let api_key = flag_only(&matches, "api_key", &cli.api_key);
        if url.is_none() && api_key.is_none() {
            anyhow::bail!(
                "Provide settings to save: --url, --api-key (or --radarr-url, --radarr-api-key)\n\
                 Example: radarr-cli init --url http://localhost:7878 --api-key YOUR_KEY"
            );
        }
        let path = config::save(cli.config.as_deref(), url, api_key)?;
        eprintln!("Configuration saved to {}", path.display());
        return Ok(());
    }

    let cfg = config::load(
        cli.url.as_deref(),
        cli.api_key.as_deref(),
        cli.output.as_deref(),
        cli.config.as_deref(),
    )?;
    let api = RadarrClient::new(&cfg.url, &cfg.api_key)?;
    let fmt = cfg.output_format;

    match cli.command {
        Commands::Movie { command } => commands::movie::run(command, &api, fmt).await,
        Commands::Queue { command } => commands::queue::run(command, &api, fmt).await,
        Commands::History { command } => commands::history::run(command, &api, fmt).await,
        Commands::System { command } => commands::system::run(command, &api, fmt).await,
        Commands::Collection { command } => commands::collection::run(command, &api, fmt).await,
        Commands::Calendar { command } => commands::calendar::run(command, &api, fmt).await,
        Commands::Config { command } => commands::config_cmd::run(command, &api, fmt).await,
        Commands::QualityProfile { command } => {
            commands::quality_profile::run(command, &api, fmt).await
        }
        Commands::QualityDefinition { command } => {
            commands::quality_definition::run(command, &api, fmt).await
        }
        Commands::CustomFormat { command } => {
            commands::custom_format::run(command, &api, fmt).await
        }
        Commands::DelayProfile { command } => {
            commands::delay_profile::run(command, &api, fmt).await
        }
        Commands::ReleaseProfile { command } => {
            commands::release_profile::run(command, &api, fmt).await
        }
        Commands::Indexer { command } => commands::indexer::run(command, &api, fmt).await,
        Commands::DownloadClient { command } => {
            commands::download_client::run(command, &api, fmt).await
        }
        Commands::Notification { command } => commands::notification::run(command, &api, fmt).await,
        Commands::ImportList { command } => commands::import_list::run(command, &api, fmt).await,
        Commands::Metadata { command } => commands::metadata::run(command, &api, fmt).await,
        Commands::Tag { command } => commands::tag::run(command, &api, fmt).await,
        Commands::Blocklist { command } => commands::blocklist::run(command, &api, fmt).await,
        Commands::Exclusion { command } => commands::exclusion::run(command, &api, fmt).await,
        Commands::MovieFile { command } => commands::movie_file::run(command, &api, fmt).await,
        Commands::Release { command } => commands::release::run(command, &api, fmt).await,
        Commands::Command { command } => commands::command::run(command, &api, fmt).await,
        Commands::Parse { args } => commands::parse::run(args, &api, fmt).await,
        Commands::Rename { args } => commands::rename::run(args, &api, fmt).await,
        Commands::ManualImport { command } => {
            commands::manual_import::run(command, &api, fmt).await
        }
        Commands::RemotePathMapping { command } => {
            commands::remote_path_mapping::run(command, &api, fmt).await
        }
        Commands::Credit { command } => commands::credit::run(command, &api, fmt).await,
        Commands::AltTitle { command } => commands::alt_title::run(command, &api, fmt).await,
        Commands::ExtraFile { command } => commands::extra_file::run(command, &api, fmt).await,
        Commands::AutoTagging { command } => commands::auto_tagging::run(command, &api, fmt).await,
        Commands::CustomFilter { command } => {
            commands::custom_filter::run(command, &api, fmt).await
        }
        Commands::Language { command } => commands::language::run(command, &api, fmt).await,
        Commands::IndexerFlag { command } => commands::indexer_flag::run(command, &api, fmt).await,
        Commands::Filesystem { command } => commands::filesystem::run(command, &api, fmt).await,
        Commands::Init => unreachable!(),
        Commands::Completions { .. } => unreachable!(),
    }
}
