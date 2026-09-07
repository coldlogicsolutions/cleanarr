use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;
pub use radarr_api::types::MovieResource;

#[derive(Subcommand)]
pub enum MovieCommand {
    /// List all movies
    List,
    /// Get a movie by ID
    Get {
        /// Movie ID
        id: i64,
    },
    /// Add a new movie
    Add {
        /// TMDB ID of the movie
        #[arg(long)]
        tmdb_id: i64,
        /// Quality profile ID
        #[arg(long)]
        quality_profile_id: i64,
        /// Root folder path
        #[arg(long)]
        root_folder_path: String,
        /// Whether to monitor the movie
        #[arg(long, default_value = "true")]
        monitored: bool,
        /// Minimum availability (announced, inCinemas, released, tba)
        #[arg(long, default_value = "released")]
        minimum_availability: String,
        /// Search for movie after adding
        #[arg(long)]
        search: bool,
    },
    /// Update a movie
    Update {
        /// Movie ID
        id: i64,
        /// JSON body (pass full movie resource)
        #[arg(long)]
        json: String,
    },
    /// Delete a movie
    Delete {
        /// Movie ID
        id: i64,
        /// Also delete downloaded files
        #[arg(long)]
        delete_files: bool,
        /// Add to import exclusion list
        #[arg(long)]
        add_exclusion: bool,
    },
    /// Search for movies
    Lookup {
        /// Search term
        #[arg(long, group = "search")]
        term: Option<String>,
        /// Search by IMDB ID
        #[arg(long, group = "search")]
        imdb: Option<String>,
        /// Search by TMDB ID
        #[arg(long, group = "search")]
        tmdb: Option<i64>,
    },
    /// Bulk edit movies
    Edit {
        /// Comma-separated movie IDs
        #[arg(long, value_delimiter = ',')]
        ids: Vec<i64>,
        /// Set monitored status
        #[arg(long)]
        monitored: Option<bool>,
        /// Set quality profile ID
        #[arg(long)]
        quality_profile_id: Option<i64>,
        /// Set minimum availability
        #[arg(long)]
        minimum_availability: Option<String>,
        /// Set root folder path
        #[arg(long)]
        root_folder_path: Option<String>,
        /// Move files when changing root folder
        #[arg(long)]
        move_files: bool,
    },
    /// Import movies from a list
    Import {
        /// JSON body with import data
        #[arg(long)]
        json: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MovieEditorResource {
    movie_ids: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    monitored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_profile_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_availability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_folder_path: Option<String>,
    move_files: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AddMovieRequest {
    tmdb_id: i64,
    quality_profile_id: i64,
    root_folder_path: String,
    monitored: bool,
    minimum_availability: String,
    add_options: AddMovieOptions,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AddMovieOptions {
    search_for_movie: bool,
}

pub async fn run(cmd: MovieCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        MovieCommand::List => {
            let movies: Vec<MovieResource> = api.get("/api/v3/movie").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = movies
                        .iter()
                        .map(|m| {
                            vec![
                                m.id.map(|i| i.to_string()).unwrap_or_default(),
                                m.title.clone().unwrap_or_default(),
                                m.year.map(|y| y.to_string()).unwrap_or_default(),
                                m.status.clone().unwrap_or_default(),
                                if m.monitored.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                if m.has_file.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                format_size(m.size_on_disk.unwrap_or(0)),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Title",
                            "Year",
                            "Status",
                            "Monitored",
                            "Has File",
                            "Size",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&movies)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = movies
                        .iter()
                        .map(|m| m.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        MovieCommand::Get { id } => {
            let movie: MovieResource = api.get(&format!("/api/v3/movie/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        movie.id.map(|i| i.to_string()).unwrap_or_default(),
                        movie.title.clone().unwrap_or_default(),
                        movie.year.map(|y| y.to_string()).unwrap_or_default(),
                        movie.status.clone().unwrap_or_default(),
                        if movie.monitored.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        movie.path.clone().unwrap_or_default(),
                        format_size(movie.size_on_disk.unwrap_or(0)),
                    ]];
                    output::print_table(
                        &["ID", "Title", "Year", "Status", "Monitored", "Path", "Size"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&movie)?,
                OutputFormat::Quiet => {
                    println!("{}", movie.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        MovieCommand::Add {
            tmdb_id,
            quality_profile_id,
            root_folder_path,
            monitored,
            minimum_availability,
            search,
        } => {
            let body = AddMovieRequest {
                tmdb_id,
                quality_profile_id,
                root_folder_path,
                monitored,
                minimum_availability,
                add_options: AddMovieOptions {
                    search_for_movie: search,
                },
            };
            let movie: MovieResource = api.post("/api/v3/movie", &body).await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Added: {} (ID: {})",
                        movie.title.unwrap_or_default(),
                        movie.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&movie)?,
                OutputFormat::Quiet => {
                    println!("{}", movie.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        MovieCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let movie: MovieResource = api.put(&format!("/api/v3/movie/{id}"), &body).await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated: {} (ID: {})",
                        movie.title.unwrap_or_default(),
                        movie.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&movie)?,
                OutputFormat::Quiet => {
                    println!("{}", movie.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        MovieCommand::Delete {
            id,
            delete_files,
            add_exclusion,
        } => {
            let mut query = vec![];
            if delete_files {
                query.push(("deleteFiles", "true"));
            }
            if add_exclusion {
                query.push(("addImportExclusion", "true"));
            }
            api.delete_with_query(&format!("/api/v3/movie/{id}"), &query)
                .await?;
            match fmt {
                OutputFormat::Table => println!("Deleted movie {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        MovieCommand::Lookup { term, imdb, tmdb } => {
            let movies: Vec<MovieResource> = if let Some(term) = term {
                api.get_with_query("/api/v3/movie/lookup", &[("term", term.as_str())])
                    .await?
            } else if let Some(imdb) = imdb {
                api.get_with_query("/api/v3/movie/lookup/imdb", &[("imdbId", imdb.as_str())])
                    .await?
            } else if let Some(tmdb) = tmdb {
                let tmdb_str = tmdb.to_string();
                api.get_with_query(
                    "/api/v3/movie/lookup/tmdb",
                    &[("tmdbId", tmdb_str.as_str())],
                )
                .await?
            } else {
                anyhow::bail!("Provide --term, --imdb, or --tmdb for lookup");
            };
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = movies
                        .iter()
                        .map(|m| {
                            vec![
                                m.tmdb_id.map(|i| i.to_string()).unwrap_or_default(),
                                m.title.clone().unwrap_or_default(),
                                m.year.map(|y| y.to_string()).unwrap_or_default(),
                                m.imdb_id.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["TMDB", "Title", "Year", "IMDB"], rows);
                }
                OutputFormat::Json => output::print_json(&movies)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = movies
                        .iter()
                        .map(|m| m.tmdb_id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        MovieCommand::Edit {
            ids,
            monitored,
            quality_profile_id,
            minimum_availability,
            root_folder_path,
            move_files,
        } => {
            let body = MovieEditorResource {
                movie_ids: ids,
                monitored,
                quality_profile_id,
                minimum_availability,
                root_folder_path,
                move_files,
            };
            let movies: Vec<MovieResource> = api.put("/api/v3/movie/editor", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Updated {} movies", movies.len()),
                OutputFormat::Json => output::print_json(&movies)?,
                OutputFormat::Quiet => println!("{}", movies.len()),
            }
            Ok(())
        }
        MovieCommand::Import { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let movies: Vec<MovieResource> = api.post("/api/v3/movie/import", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Imported {} movies", movies.len()),
                OutputFormat::Json => output::print_json(&movies)?,
                OutputFormat::Quiet => println!("{}", movies.len()),
            }
            Ok(())
        }
    }
}

fn format_size(bytes: i64) -> String {
    const KB: i64 = 1_000;
    const MB: i64 = 1_000_000;
    const GB: i64 = 1_000_000_000;
    const TB: i64 = 1_000_000_000_000;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
