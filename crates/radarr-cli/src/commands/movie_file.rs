use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum MovieFileCommand {
    /// List movie files for a movie, or by movie file ID
    List {
        /// List all files belonging to this movie
        #[arg(long)]
        movie_id: Option<i64>,
        /// Comma-separated movie file IDs
        #[arg(long, value_delimiter = ',')]
        ids: Vec<i64>,
    },
    /// Get a movie file by ID
    Get {
        /// Movie file ID
        id: i64,
    },
    /// Update a movie file
    Update {
        /// Movie file ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a movie file
    Delete {
        /// Movie file ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovieFileResource {
    id: Option<i64>,
    movie_id: Option<i64>,
    relative_path: Option<String>,
    size: Option<i64>,
    date_added: Option<String>,
}

pub async fn run(cmd: MovieFileCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        MovieFileCommand::List { movie_id, ids } => {
            // Radarr's GET /moviefile requires a filter: either movieId or movieFileIds.
            let owned_query: Vec<(&str, String)> = match (movie_id, ids.is_empty()) {
                (Some(_), false) => anyhow::bail!(
                    "Provide either --movie-id or --ids, not both, when listing movie files"
                ),
                (None, true) => anyhow::bail!(
                    "Provide --movie-id <id> or --ids <id,id,...> when listing movie files"
                ),
                (Some(movie_id), true) => vec![("movieId", movie_id.to_string())],
                (None, false) => ids
                    .iter()
                    .map(|id| ("movieFileIds", id.to_string()))
                    .collect(),
            };
            let query: Vec<(&str, &str)> =
                owned_query.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let items: Vec<MovieFileResource> =
                api.get_with_query("/api/v3/moviefile", &query).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|f| {
                            vec![
                                f.id.map(|v| v.to_string()).unwrap_or_default(),
                                f.movie_id.map(|v| v.to_string()).unwrap_or_default(),
                                f.relative_path.clone().unwrap_or_default(),
                                f.size.map(|v| v.to_string()).unwrap_or_default(),
                                f.date_added.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Movie ID", "Path", "Size", "Date Added"], rows);
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
        MovieFileCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/moviefile/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        MovieFileCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: MovieFileResource =
                api.put(&format!("/api/v3/moviefile/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated movie file {}",
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        MovieFileCommand::Delete { id } => {
            api.delete(&format!("/api/v3/moviefile/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted movie file {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
