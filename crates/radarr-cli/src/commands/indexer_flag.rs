use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum IndexerFlagCommand {
    /// List all indexer flags
    List,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexerFlagResource {
    id: Option<i64>,
    name: Option<String>,
}

pub async fn run(cmd: IndexerFlagCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        IndexerFlagCommand::List => {
            let items: Vec<IndexerFlagResource> = api.get("/api/v3/indexerflag").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|f| {
                            vec![
                                f.id.map(|v| v.to_string()).unwrap_or_default(),
                                f.name.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name"], rows);
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
    }
}
