use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum TagCommand {
    /// List all tags
    List,
    /// Get a tag by ID
    Get {
        /// Tag ID
        id: i64,
    },
    /// Create a tag
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a tag
    Update {
        /// Tag ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a tag
    Delete {
        /// Tag ID
        id: i64,
    },
    /// Get detailed tag information (includes usage)
    Detail,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagResource {
    id: Option<i64>,
    label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagDetailResource {
    id: Option<i64>,
    label: Option<String>,
    #[serde(default)]
    movie_ids: Vec<i64>,
    #[serde(default)]
    indexer_ids: Vec<i64>,
    #[serde(default)]
    notification_ids: Vec<i64>,
}

pub async fn run(cmd: TagCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        TagCommand::List => {
            let items: Vec<TagResource> = api.get("/api/v3/tag").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|t| {
                            vec![
                                t.id.map(|v| v.to_string()).unwrap_or_default(),
                                t.label.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Label"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|t| t.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        TagCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/tag/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        TagCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: TagResource = api.post("/api/v3/tag", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created tag: {} (ID: {})",
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
        TagCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: TagResource = api.put(&format!("/api/v3/tag/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated tag: {} (ID: {})",
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
        TagCommand::Delete { id } => {
            api.delete(&format!("/api/v3/tag/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted tag {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        TagCommand::Detail => {
            let items: Vec<TagDetailResource> = api.get("/api/v3/tag/detail").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|t| {
                            vec![
                                t.id.map(|v| v.to_string()).unwrap_or_default(),
                                t.label.clone().unwrap_or_default(),
                                t.movie_ids.len().to_string(),
                                t.indexer_ids.len().to_string(),
                                t.notification_ids.len().to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["ID", "Label", "Movies", "Indexers", "Notifications"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|t| t.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
    }
}
