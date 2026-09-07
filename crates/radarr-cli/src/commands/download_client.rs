use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum DownloadClientCommand {
    /// List all download clients
    List,
    /// Get a download client by ID
    Get {
        /// Download client ID
        id: i64,
    },
    /// Create a download client
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a download client
    Update {
        /// Download client ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a download client
    Delete {
        /// Download client ID
        id: i64,
    },
    /// Test a download client configuration
    Test {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Test all download clients
    TestAll,
    /// Get download client schema
    Schema,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadClientResource {
    id: Option<i64>,
    name: Option<String>,
    implementation_name: Option<String>,
    enable: Option<bool>,
    protocol: Option<String>,
    priority: Option<i64>,
}

pub async fn run(cmd: DownloadClientCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        DownloadClientCommand::List => {
            let items: Vec<DownloadClientResource> = api.get("/api/v3/downloadclient").await?;
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
                                if i.enable.unwrap_or(false) {
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
                            "Enabled",
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
        DownloadClientCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/downloadclient/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        DownloadClientCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: DownloadClientResource = api.post("/api/v3/downloadclient", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created download client: {} (ID: {})",
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
        DownloadClientCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: DownloadClientResource = api
                .put(&format!("/api/v3/downloadclient/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated download client: {} (ID: {})",
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
        DownloadClientCommand::Delete { id } => {
            api.delete(&format!("/api/v3/downloadclient/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted download client {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        DownloadClientCommand::Test { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let _: serde_json::Value = api.post("/api/v3/downloadclient/test", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Test passed"),
                OutputFormat::Json => println!(r#"{{"success": true}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        DownloadClientCommand::TestAll => {
            let results: serde_json::Value =
                api.post_empty("/api/v3/downloadclient/testall").await?;
            match fmt {
                OutputFormat::Table => println!("Test all completed"),
                OutputFormat::Json => output::print_json(&results)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        DownloadClientCommand::Schema => {
            let schema: serde_json::Value = api.get("/api/v3/downloadclient/schema").await?;
            output::print_json(&schema)?;
            Ok(())
        }
    }
}
