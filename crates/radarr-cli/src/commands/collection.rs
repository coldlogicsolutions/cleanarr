use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum CollectionCommand {
    /// List all collections
    List,
    /// Get a collection by ID
    Get {
        /// Collection ID
        id: i64,
    },
    /// Update a collection
    Update {
        /// Collection ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionResource {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub tmdb_id: Option<i64>,
    pub monitored: Option<bool>,
    pub missing_movies: Option<i64>,
    pub quality_profile_id: Option<i64>,
    pub root_folder_path: Option<String>,
}

pub async fn run(cmd: CollectionCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        CollectionCommand::List => {
            let collections: Vec<CollectionResource> = api.get("/api/v3/collection").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = collections
                        .iter()
                        .map(|c| {
                            vec![
                                c.id.map(|i| i.to_string()).unwrap_or_default(),
                                c.title.clone().unwrap_or_default(),
                                c.tmdb_id.map(|i| i.to_string()).unwrap_or_default(),
                                if c.monitored.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                c.missing_movies.map(|i| i.to_string()).unwrap_or_default(),
                                c.quality_profile_id
                                    .map(|i| i.to_string())
                                    .unwrap_or_default(),
                                c.root_folder_path.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Title",
                            "TMDB",
                            "Monitored",
                            "Missing",
                            "Quality Profile",
                            "Root Folder",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&collections)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = collections
                        .iter()
                        .map(|c| c.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        CollectionCommand::Get { id } => {
            let collection: CollectionResource =
                api.get(&format!("/api/v3/collection/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        collection.id.map(|i| i.to_string()).unwrap_or_default(),
                        collection.title.clone().unwrap_or_default(),
                        collection
                            .tmdb_id
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        if collection.monitored.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        collection
                            .missing_movies
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        collection
                            .quality_profile_id
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        collection.root_folder_path.clone().unwrap_or_default(),
                    ]];
                    output::print_table(
                        &[
                            "ID",
                            "Title",
                            "TMDB",
                            "Monitored",
                            "Missing",
                            "Quality Profile",
                            "Root Folder",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&collection)?,
                OutputFormat::Quiet => {
                    println!(
                        "{}",
                        collection.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        CollectionCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let collection: CollectionResource =
                api.put(&format!("/api/v3/collection/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated: {} (ID: {})",
                        collection.title.unwrap_or_default(),
                        collection.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&collection)?,
                OutputFormat::Quiet => {
                    println!(
                        "{}",
                        collection.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
    }
}
