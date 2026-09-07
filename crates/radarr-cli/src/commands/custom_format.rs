use anyhow::{Context, Result};
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
            let raw: serde_json::Value = api.get("/api/v3/customformat").await?;
            if fmt == OutputFormat::Json {
                return output::print_json(&raw);
            }
            let formats: Vec<CustomFormatResource> = serde_json::from_value(raw)
                .context("Failed to decode custom format list from the Radarr API")?;
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
                OutputFormat::Json => unreachable!("raw JSON handled above"),
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
            let raw: serde_json::Value = api.get(&format!("/api/v3/customformat/{id}")).await?;
            if fmt == OutputFormat::Json {
                return output::print_json(&raw);
            }
            let format: CustomFormatResource = serde_json::from_value(raw).with_context(|| {
                format!("Failed to decode custom format {id} from the Radarr API")
            })?;
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
                OutputFormat::Json => unreachable!("raw JSON handled above"),
                OutputFormat::Quiet => {
                    println!("{}", format.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        CustomFormatCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let raw: serde_json::Value = api.post("/api/v3/customformat", &body).await?;
            if fmt == OutputFormat::Json {
                return output::print_json(&raw);
            }
            let format: CustomFormatResource = serde_json::from_value(raw)
                .context("Failed to decode the created custom format from the Radarr API")?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Created: {} (ID: {})",
                        format.name.unwrap_or_default(),
                        format.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => unreachable!("raw JSON handled above"),
                OutputFormat::Quiet => {
                    println!("{}", format.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        CustomFormatCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let raw: serde_json::Value = api
                .put(&format!("/api/v3/customformat/{id}"), &body)
                .await?;
            if fmt == OutputFormat::Json {
                return output::print_json(&raw);
            }
            let cf: CustomFormatResource = serde_json::from_value(raw).with_context(|| {
                format!("Failed to decode the updated custom format {id} from the Radarr API")
            })?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated: {} (ID: {})",
                        cf.name.unwrap_or_default(),
                        cf.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => unreachable!("raw JSON handled above"),
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
