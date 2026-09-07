use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum CommandCommand {
    /// List running commands
    List,
    /// Get a command by ID
    Get {
        /// Command ID
        id: i64,
    },
    /// Run a command
    Run {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Cancel a running command
    Cancel {
        /// Command ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandResource {
    id: Option<i64>,
    name: Option<String>,
    status: Option<String>,
    started: Option<String>,
    ended: Option<String>,
}

pub async fn run(cmd: CommandCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        CommandCommand::List => {
            let items: Vec<CommandResource> = api.get("/api/v3/command").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|c| {
                            vec![
                                c.id.map(|v| v.to_string()).unwrap_or_default(),
                                c.name.clone().unwrap_or_default(),
                                c.status.clone().unwrap_or_default(),
                                c.started.clone().unwrap_or_default(),
                                c.ended.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Status", "Started", "Ended"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|c| c.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        CommandCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/command/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        CommandCommand::Run { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: CommandResource = api.post("/api/v3/command", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Started command: {} (ID: {})",
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
        CommandCommand::Cancel { id } => {
            api.delete(&format!("/api/v3/command/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Cancelled command {id}"),
                OutputFormat::Json => println!(r#"{{"cancelled": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
