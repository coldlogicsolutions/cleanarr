use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum LanguageCommand {
    /// List all languages
    List,
    /// Get a language by ID
    Get {
        /// Language ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanguageResource {
    id: Option<i64>,
    name: Option<String>,
    name_lower: Option<String>,
}

pub async fn run(cmd: LanguageCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        LanguageCommand::List => {
            let items: Vec<LanguageResource> = api.get("/api/v3/language").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|l| {
                            vec![
                                l.id.map(|v| v.to_string()).unwrap_or_default(),
                                l.name.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|l| l.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        LanguageCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/language/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
    }
}
