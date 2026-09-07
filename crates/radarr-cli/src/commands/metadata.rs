use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum MetadataCommand {
    /// List all metadata consumers
    List,
    /// Get a metadata consumer by ID
    Get {
        /// Metadata ID
        id: i64,
    },
    /// Create a metadata consumer
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a metadata consumer
    Update {
        /// Metadata ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a metadata consumer
    Delete {
        /// Metadata ID
        id: i64,
    },
    /// Test a metadata consumer configuration
    Test {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Test all metadata consumers
    TestAll,
    /// Get metadata consumer schema
    Schema,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataResource {
    id: Option<i64>,
    name: Option<String>,
    implementation_name: Option<String>,
    enable: Option<bool>,
    config_contract: Option<String>,
}

pub async fn run(cmd: MetadataCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        MetadataCommand::List => {
            let items: Vec<MetadataResource> = api.get("/api/v3/metadata").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|i| {
                            vec![
                                i.id.map(|v| v.to_string()).unwrap_or_default(),
                                i.name.clone().unwrap_or_default(),
                                i.implementation_name.clone().unwrap_or_default(),
                                if i.enable.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Implementation", "Enabled"], rows);
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
        MetadataCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/metadata/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        MetadataCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: MetadataResource = api.post("/api/v3/metadata", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created metadata consumer: {} (ID: {})",
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
        MetadataCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: MetadataResource = api.put(&format!("/api/v3/metadata/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated metadata consumer: {} (ID: {})",
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
        MetadataCommand::Delete { id } => {
            api.delete(&format!("/api/v3/metadata/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted metadata consumer {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        MetadataCommand::Test { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let _: serde_json::Value = api.post("/api/v3/metadata/test", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Test passed"),
                OutputFormat::Json => println!(r#"{{"success": true}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        MetadataCommand::TestAll => {
            let results: serde_json::Value = api.post_empty("/api/v3/metadata/testall").await?;
            match fmt {
                OutputFormat::Table => println!("Test all completed"),
                OutputFormat::Json => output::print_json(&results)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        MetadataCommand::Schema => {
            let schema: serde_json::Value = api.get("/api/v3/metadata/schema").await?;
            output::print_json(&schema)?;
            Ok(())
        }
    }
}
