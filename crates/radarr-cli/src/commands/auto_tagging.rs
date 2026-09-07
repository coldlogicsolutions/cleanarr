use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum AutoTaggingCommand {
    /// List all auto-tagging rules
    List,
    /// Get an auto-tagging rule by ID
    Get {
        /// Auto-tagging rule ID
        id: i64,
    },
    /// Create an auto-tagging rule
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update an auto-tagging rule
    Update {
        /// Auto-tagging rule ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete an auto-tagging rule
    Delete {
        /// Auto-tagging rule ID
        id: i64,
    },
    /// Get auto-tagging schema
    Schema,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoTaggingResource {
    id: Option<i64>,
    name: Option<String>,
    remove_tags_automatically: Option<bool>,
    #[serde(default)]
    tags: Vec<i64>,
}

pub async fn run(cmd: AutoTaggingCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        AutoTaggingCommand::List => {
            let items: Vec<AutoTaggingResource> = api.get("/api/v3/autotagging").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|a| {
                            vec![
                                a.id.map(|v| v.to_string()).unwrap_or_default(),
                                a.name.clone().unwrap_or_default(),
                                a.tags.len().to_string(),
                                if a.remove_tags_automatically.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Tags", "Auto Remove"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|a| a.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        AutoTaggingCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/autotagging/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        AutoTaggingCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: AutoTaggingResource = api.post("/api/v3/autotagging", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created auto-tagging rule: {} (ID: {})",
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
        AutoTaggingCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: AutoTaggingResource =
                api.put(&format!("/api/v3/autotagging/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated auto-tagging rule: {} (ID: {})",
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
        AutoTaggingCommand::Delete { id } => {
            api.delete(&format!("/api/v3/autotagging/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted auto-tagging rule {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        AutoTaggingCommand::Schema => {
            let schema: serde_json::Value = api.get("/api/v3/autotagging/schema").await?;
            output::print_json(&schema)?;
            Ok(())
        }
    }
}
