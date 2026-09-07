use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ImportListCommand {
    /// List all import lists
    List,
    /// Get an import list by ID
    Get {
        /// Import list ID
        id: i64,
    },
    /// Create an import list
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update an import list
    Update {
        /// Import list ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete an import list
    Delete {
        /// Import list ID
        id: i64,
    },
    /// Test an import list configuration
    Test {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Test all import lists
    TestAll,
    /// Get import list schema
    Schema,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportListResource {
    id: Option<i64>,
    name: Option<String>,
    implementation_name: Option<String>,
    enable_auto: Option<bool>,
    enabled: Option<bool>,
    config_contract: Option<String>,
    list_type: Option<String>,
}

pub async fn run(cmd: ImportListCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ImportListCommand::List => {
            let items: Vec<ImportListResource> = api.get("/api/v3/importlist").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|i| {
                            vec![
                                i.id.map(|v| v.to_string()).unwrap_or_default(),
                                i.name.clone().unwrap_or_default(),
                                i.implementation_name.clone().unwrap_or_default(),
                                if i.enabled.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                if i.enable_auto.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Implementation", "Enabled", "Auto"], rows);
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
        ImportListCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/importlist/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        ImportListCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: ImportListResource = api.post("/api/v3/importlist", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created import list: {} (ID: {})",
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
        ImportListCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: ImportListResource =
                api.put(&format!("/api/v3/importlist/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated import list: {} (ID: {})",
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
        ImportListCommand::Delete { id } => {
            api.delete(&format!("/api/v3/importlist/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted import list {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        ImportListCommand::Test { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let _: serde_json::Value = api.post("/api/v3/importlist/test", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Test passed"),
                OutputFormat::Json => println!(r#"{{"success": true}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        ImportListCommand::TestAll => {
            let results: serde_json::Value = api.post_empty("/api/v3/importlist/testall").await?;
            match fmt {
                OutputFormat::Table => println!("Test all completed"),
                OutputFormat::Json => output::print_json(&results)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        ImportListCommand::Schema => {
            let schema: serde_json::Value = api.get("/api/v3/importlist/schema").await?;
            output::print_json(&schema)?;
            Ok(())
        }
    }
}
