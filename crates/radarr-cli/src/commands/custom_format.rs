use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum CustomFormatCommand {
    /// List all custom formats
    List,
    /// Get a custom format by ID
    Get {
        /// Custom format ID
        id: i64,
    },
    /// Create a custom format
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a custom format
    Update {
        /// Custom format ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a custom format
    Delete {
        /// Custom format ID
        id: i64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomFormatResource {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub include_custom_format_when_renaming: Option<bool>,
}

pub async fn run(cmd: CustomFormatCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        CustomFormatCommand::List => {
            let formats: Vec<CustomFormatResource> = api.get("/api/v3/customformat").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = formats
                        .iter()
                        .map(|f| {
                            vec![
                                f.id.map(|i| i.to_string()).unwrap_or_default(),
                                f.name.clone().unwrap_or_default(),
                                if f.include_custom_format_when_renaming.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Include When Renaming"], rows);
                }
                OutputFormat::Json => output::print_json(&formats)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = formats
                        .iter()
                        .map(|f| f.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        CustomFormatCommand::Get { id } => {
            let format: CustomFormatResource =
                api.get(&format!("/api/v3/customformat/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        format.id.map(|i| i.to_string()).unwrap_or_default(),
                        format.name.clone().unwrap_or_default(),
                        if format.include_custom_format_when_renaming.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                    ]];
                    output::print_table(&["ID", "Name", "Include When Renaming"], rows);
                }
                OutputFormat::Json => output::print_json(&format)?,
                OutputFormat::Quiet => {
                    println!("{}", format.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        CustomFormatCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let format: CustomFormatResource = api.post("/api/v3/customformat", &body).await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Created: {} (ID: {})",
                        format.name.unwrap_or_default(),
                        format.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&format)?,
                OutputFormat::Quiet => {
                    println!("{}", format.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        CustomFormatCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let cf: CustomFormatResource = api
                .put(&format!("/api/v3/customformat/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated: {} (ID: {})",
                        cf.name.unwrap_or_default(),
                        cf.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&cf)?,
                OutputFormat::Quiet => {
                    println!("{}", cf.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        CustomFormatCommand::Delete { id } => {
            api.delete(&format!("/api/v3/customformat/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted custom format {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
