use anyhow::Result;
use clap::Subcommand;

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Host configuration
    Host {
        #[command(subcommand)]
        command: HostConfigCommand,
    },
    /// UI configuration
    Ui {
        #[command(subcommand)]
        command: UiConfigCommand,
    },
    /// Naming configuration
    Naming {
        #[command(subcommand)]
        command: NamingConfigCommand,
    },
    /// Media management configuration
    MediaManagement {
        #[command(subcommand)]
        command: MediaManagementConfigCommand,
    },
    /// Download client configuration
    DownloadClient {
        #[command(subcommand)]
        command: DownloadClientConfigCommand,
    },
    /// Indexer configuration
    Indexer {
        #[command(subcommand)]
        command: IndexerConfigCommand,
    },
    /// Import list configuration
    ImportList {
        #[command(subcommand)]
        command: ImportListConfigCommand,
    },
    /// Metadata configuration
    Metadata {
        #[command(subcommand)]
        command: MetadataConfigCommand,
    },
}

#[derive(Subcommand)]
pub enum HostConfigCommand {
    /// Get host configuration
    Get,
    /// Update host configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum UiConfigCommand {
    /// Get UI configuration
    Get,
    /// Update UI configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum NamingConfigCommand {
    /// Get naming configuration
    Get,
    /// Update naming configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum MediaManagementConfigCommand {
    /// Get media management configuration
    Get,
    /// Update media management configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum DownloadClientConfigCommand {
    /// Get download client configuration
    Get,
    /// Update download client configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum IndexerConfigCommand {
    /// Get indexer configuration
    Get,
    /// Update indexer configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum ImportListConfigCommand {
    /// Get import list configuration
    Get,
    /// Update import list configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Subcommand)]
pub enum MetadataConfigCommand {
    /// Get metadata configuration
    Get,
    /// Update metadata configuration
    Update {
        /// Config ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

async fn get_config(api: &RadarrClient, fmt: OutputFormat, path: &str) -> Result<()> {
    let config: serde_json::Value = api.get(path).await?;
    match fmt {
        OutputFormat::Table | OutputFormat::Json => output::print_json(&config)?,
        OutputFormat::Quiet => {
            if let Some(id) = config.get("id").and_then(|v| v.as_i64()) {
                println!("{id}");
            }
        }
    }
    Ok(())
}

async fn update_config(
    api: &RadarrClient,
    fmt: OutputFormat,
    path: &str,
    id: i64,
    json: &str,
) -> Result<()> {
    let body: serde_json::Value = serde_json::from_str(json)?;
    let config: serde_json::Value = api.put(&format!("{path}/{id}"), &body).await?;
    match fmt {
        OutputFormat::Table => {
            println!("Updated config (ID: {id})");
        }
        OutputFormat::Json => output::print_json(&config)?,
        OutputFormat::Quiet => println!("{id}"),
    }
    Ok(())
}

pub async fn run(cmd: ConfigCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ConfigCommand::Host { command } => match command {
            HostConfigCommand::Get => get_config(api, fmt, "/api/v3/config/host").await,
            HostConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/host", id, &json).await
            }
        },
        ConfigCommand::Ui { command } => match command {
            UiConfigCommand::Get => get_config(api, fmt, "/api/v3/config/ui").await,
            UiConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/ui", id, &json).await
            }
        },
        ConfigCommand::Naming { command } => match command {
            NamingConfigCommand::Get => get_config(api, fmt, "/api/v3/config/naming").await,
            NamingConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/naming", id, &json).await
            }
        },
        ConfigCommand::MediaManagement { command } => match command {
            MediaManagementConfigCommand::Get => {
                get_config(api, fmt, "/api/v3/config/mediamanagement").await
            }
            MediaManagementConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/mediamanagement", id, &json).await
            }
        },
        ConfigCommand::DownloadClient { command } => match command {
            DownloadClientConfigCommand::Get => {
                get_config(api, fmt, "/api/v3/config/downloadclient").await
            }
            DownloadClientConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/downloadclient", id, &json).await
            }
        },
        ConfigCommand::Indexer { command } => match command {
            IndexerConfigCommand::Get => get_config(api, fmt, "/api/v3/config/indexer").await,
            IndexerConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/indexer", id, &json).await
            }
        },
        ConfigCommand::ImportList { command } => match command {
            ImportListConfigCommand::Get => get_config(api, fmt, "/api/v3/config/importlist").await,
            ImportListConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/importlist", id, &json).await
            }
        },
        ConfigCommand::Metadata { command } => match command {
            MetadataConfigCommand::Get => get_config(api, fmt, "/api/v3/config/metadata").await,
            MetadataConfigCommand::Update { id, json } => {
                update_config(api, fmt, "/api/v3/config/metadata", id, &json).await
            }
        },
    }
}
