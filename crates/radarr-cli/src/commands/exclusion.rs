use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ExclusionCommand {
    /// List all exclusions
    List,
    /// Get an exclusion by ID
    Get {
        /// Exclusion ID
        id: i64,
    },
    /// Create an exclusion
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update an exclusion
    Update {
        /// Exclusion ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete an exclusion
    Delete {
        /// Exclusion ID
        id: i64,
    },
    /// List exclusions (paged)
    Paged,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExclusionResource {
    id: Option<i64>,
    tmdb_id: Option<i64>,
    movie_title: Option<String>,
    movie_year: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExclusionPage {
    page: Option<i64>,
    page_size: Option<i64>,
    total_records: Option<i64>,
    #[serde(default)]
    records: Vec<ExclusionResource>,
}

pub async fn run(cmd: ExclusionCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ExclusionCommand::List => {
            let items: Vec<ExclusionResource> = api.get("/api/v3/exclusions").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|e| {
                            vec![
                                e.id.map(|v| v.to_string()).unwrap_or_default(),
                                e.tmdb_id.map(|v| v.to_string()).unwrap_or_default(),
                                e.movie_title.clone().unwrap_or_default(),
                                e.movie_year.map(|v| v.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "TMDB ID", "Title", "Year"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|e| e.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        ExclusionCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/exclusions/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        ExclusionCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: ExclusionResource = api.post("/api/v3/exclusions", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created exclusion: {} (ID: {})",
                    item.movie_title.unwrap_or_default(),
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        ExclusionCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: ExclusionResource =
                api.put(&format!("/api/v3/exclusions/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated exclusion: {} (ID: {})",
                    item.movie_title.unwrap_or_default(),
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        ExclusionCommand::Delete { id } => {
            api.delete(&format!("/api/v3/exclusions/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted exclusion {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        ExclusionCommand::Paged => {
            let page: ExclusionPage = api.get("/api/v3/exclusions/paged").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = page
                        .records
                        .iter()
                        .map(|e| {
                            vec![
                                e.id.map(|v| v.to_string()).unwrap_or_default(),
                                e.tmdb_id.map(|v| v.to_string()).unwrap_or_default(),
                                e.movie_title.clone().unwrap_or_default(),
                                e.movie_year.map(|v| v.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "TMDB ID", "Title", "Year"], rows);
                }
                OutputFormat::Json => output::print_json(&page)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = page
                        .records
                        .iter()
                        .map(|e| e.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
    }
}
