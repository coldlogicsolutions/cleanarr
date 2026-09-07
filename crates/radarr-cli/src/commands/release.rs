use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ReleaseCommand {
    /// Search for releases
    Search {
        /// Movie ID to search for
        #[arg(long)]
        movie_id: Option<i64>,
    },
    /// Push a release
    Push {
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseResource {
    guid: Option<String>,
    title: Option<String>,
    indexer: Option<String>,
    size: Option<i64>,
    protocol: Option<String>,
    approved: Option<bool>,
}

pub async fn run(cmd: ReleaseCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ReleaseCommand::Search { movie_id } => {
            let items: Vec<ReleaseResource> = if let Some(mid) = movie_id {
                let mid_str = mid.to_string();
                api.get_with_query("/api/v3/release", &[("movieId", mid_str.as_str())])
                    .await?
            } else {
                api.get("/api/v3/release").await?
            };
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|r| {
                            vec![
                                r.title.clone().unwrap_or_default(),
                                r.indexer.clone().unwrap_or_default(),
                                r.protocol.clone().unwrap_or_default(),
                                r.size.map(|v| v.to_string()).unwrap_or_default(),
                                if r.approved.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["Title", "Indexer", "Protocol", "Size", "Approved"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let guids: Vec<String> = items
                        .iter()
                        .map(|r| r.guid.clone().unwrap_or_default())
                        .collect();
                    output::print_quiet(&guids);
                }
            }
            Ok(())
        }
        ReleaseCommand::Push { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let result: serde_json::Value = api.post("/api/v3/release/push", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Release pushed"),
                OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
