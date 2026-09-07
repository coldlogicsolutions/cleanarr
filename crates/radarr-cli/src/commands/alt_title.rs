use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum AltTitleCommand {
    /// List alternative titles
    List {
        /// Filter by movie ID
        #[arg(long)]
        movie_id: Option<i64>,
    },
    /// Get an alternative title by ID
    Get {
        /// Alternative title ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AltTitleResource {
    id: Option<i64>,
    title: Option<String>,
    source_type: Option<String>,
    movie_metadata_id: Option<i64>,
}

pub async fn run(cmd: AltTitleCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        AltTitleCommand::List { movie_id } => {
            let items: Vec<AltTitleResource> = if let Some(mid) = movie_id {
                let mid_str = mid.to_string();
                api.get_with_query("/api/v3/alttitle", &[("movieId", mid_str.as_str())])
                    .await?
            } else {
                api.get("/api/v3/alttitle").await?
            };
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|t| {
                            vec![
                                t.id.map(|v| v.to_string()).unwrap_or_default(),
                                t.title.clone().unwrap_or_default(),
                                t.source_type.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Title", "Source Type"], rows);
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
        AltTitleCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/alttitle/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
    }
}
