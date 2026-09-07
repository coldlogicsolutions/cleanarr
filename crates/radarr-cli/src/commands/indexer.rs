use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum IndexerCommand {
    /// List all indexers
    List,
    /// Get an indexer by ID
    Get {
        /// Indexer ID
        id: i64,
    },
    /// Create an indexer
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update an indexer
    Update {
        /// Indexer ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete an indexer
    Delete {
        /// Indexer ID
        id: i64,
    },
    /// Test an indexer configuration
    Test {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Test all indexers
    TestAll,
    /// Get indexer schema
    Schema,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexerResource {
    id: Option<i64>,
    name: Option<String>,
    implementation_name: Option<String>,
    enable_rss: Option<bool>,
    enable_automatic_search: Option<bool>,
    enable_interactive_search: Option<bool>,
    config_contract: Option<String>,
    protocol: Option<String>,
    priority: Option<i64>,
}

pub async fn run(cmd: IndexerCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        IndexerCommand::List => {
            let items: Vec<IndexerResource> = api.get("/api/v3/indexer").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|i| {
                            vec![
                                i.id.map(|v| v.to_string()).unwrap_or_default(),
                                i.name.clone().unwrap_or_default(),
                                i.implementation_name.clone().unwrap_or_default(),
                                i.protocol.clone().unwrap_or_default(),
                                if i.enable_rss.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                i.priority.map(|v| v.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Name",
                            "Implementation",
                            "Protocol",
                            "RSS",
                            "Priority",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|i| i.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        IndexerCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/indexer/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        IndexerCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: IndexerResource = api.post("/api/v3/indexer", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created indexer: {} (ID: {})",
                    item.name.unwrap_or_default(),
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        IndexerCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: IndexerResource = api.put(&format!("/api/v3/indexer/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated indexer: {} (ID: {})",
                    item.name.unwrap_or_default(),
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        IndexerCommand::Delete { id } => {
            api.delete(&format!("/api/v3/indexer/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted indexer {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        IndexerCommand::Test { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let _: serde_json::Value = api.post("/api/v3/indexer/test", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Test passed"),
                OutputFormat::Json => println!(r#"{{"success": true}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        IndexerCommand::TestAll => {
            let results: serde_json::Value = api.post_empty("/api/v3/indexer/testall").await?;
            match fmt {
                OutputFormat::Table => println!("Test all completed"),
                OutputFormat::Json => output::print_json(&results)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        IndexerCommand::Schema => {
            let schema: serde_json::Value = api.get("/api/v3/indexer/schema").await?;
            output::print_json(&schema)?;
            Ok(())
        }
    }
}
