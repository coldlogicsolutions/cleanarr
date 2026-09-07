use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum CustomFilterCommand {
    /// List all custom filters
    List,
    /// Get a custom filter by ID
    Get {
        /// Custom filter ID
        id: i64,
    },
    /// Create a custom filter
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a custom filter
    Update {
        /// Custom filter ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a custom filter
    Delete {
        /// Custom filter ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomFilterResource {
    id: Option<i64>,
    #[serde(rename = "type")]
    filter_type: Option<String>,
    label: Option<String>,
}

pub async fn run(cmd: CustomFilterCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        CustomFilterCommand::List => {
            let items: Vec<CustomFilterResource> = api.get("/api/v3/customfilter").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|f| {
                            vec![
                                f.id.map(|v| v.to_string()).unwrap_or_default(),
                                f.label.clone().unwrap_or_default(),
                                f.filter_type.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Label", "Type"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|f| f.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        CustomFilterCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/customfilter/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        CustomFilterCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: CustomFilterResource = api.post("/api/v3/customfilter", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created custom filter: {} (ID: {})",
                    item.label.unwrap_or_default(),
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        CustomFilterCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: CustomFilterResource = api
                .put(&format!("/api/v3/customfilter/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated custom filter: {} (ID: {})",
                    item.label.unwrap_or_default(),
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        CustomFilterCommand::Delete { id } => {
            api.delete(&format!("/api/v3/customfilter/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted custom filter {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
