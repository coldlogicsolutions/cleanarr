use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum NotificationCommand {
    /// List all notifications
    List,
    /// Get a notification by ID
    Get {
        /// Notification ID
        id: i64,
    },
    /// Create a notification
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a notification
    Update {
        /// Notification ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a notification
    Delete {
        /// Notification ID
        id: i64,
    },
    /// Test a notification configuration
    Test {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Test all notifications
    TestAll,
    /// Get notification schema
    Schema,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationResource {
    id: Option<i64>,
    name: Option<String>,
    implementation_name: Option<String>,
    on_grab: Option<bool>,
    on_download: Option<bool>,
    config_contract: Option<String>,
}

pub async fn run(cmd: NotificationCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        NotificationCommand::List => {
            let items: Vec<NotificationResource> = api.get("/api/v3/notification").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|i| {
                            vec![
                                i.id.map(|v| v.to_string()).unwrap_or_default(),
                                i.name.clone().unwrap_or_default(),
                                i.implementation_name.clone().unwrap_or_default(),
                                if i.on_grab.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                if i.on_download.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["ID", "Name", "Implementation", "On Grab", "On Download"],
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
        NotificationCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/notification/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        NotificationCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: NotificationResource = api.post("/api/v3/notification", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created notification: {} (ID: {})",
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
        NotificationCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: NotificationResource = api
                .put(&format!("/api/v3/notification/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated notification: {} (ID: {})",
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
        NotificationCommand::Delete { id } => {
            api.delete(&format!("/api/v3/notification/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted notification {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        NotificationCommand::Test { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let _: serde_json::Value = api.post("/api/v3/notification/test", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Test passed"),
                OutputFormat::Json => println!(r#"{{"success": true}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        NotificationCommand::TestAll => {
            let results: serde_json::Value = api.post_empty("/api/v3/notification/testall").await?;
            match fmt {
                OutputFormat::Table => println!("Test all completed"),
                OutputFormat::Json => output::print_json(&results)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        NotificationCommand::Schema => {
            let schema: serde_json::Value = api.get("/api/v3/notification/schema").await?;
            output::print_json(&schema)?;
            Ok(())
        }
    }
}
