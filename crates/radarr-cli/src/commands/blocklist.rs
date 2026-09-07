use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum BlocklistCommand {
    /// List all blocklist entries
    List {
        /// Page number
        #[arg(long, default_value = "1")]
        page: i64,
        /// Page size
        #[arg(long, default_value = "20")]
        page_size: i64,
        /// Sort key (e.g. date, sourceTitle)
        #[arg(long)]
        sort_key: Option<String>,
        /// Sort direction (ascending or descending)
        #[arg(long)]
        sort_dir: Option<String>,
    },
    /// Get blocklist entries for a movie
    Movie {
        /// Movie ID
        movie_id: i64,
    },
    /// Delete a blocklist entry
    Delete {
        /// Blocklist entry ID
        id: i64,
    },
    /// Bulk delete blocklist entries
    BulkDelete {
        /// Comma-separated blocklist entry IDs
        #[arg(value_delimiter = ',')]
        ids: Vec<i64>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlocklistPage {
    page: Option<i64>,
    page_size: Option<i64>,
    total_records: Option<i64>,
    #[serde(default)]
    records: Vec<BlocklistRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlocklistRecord {
    id: Option<i64>,
    movie_id: Option<i64>,
    source_title: Option<String>,
    date: Option<String>,
    protocol: Option<String>,
    indexer: Option<String>,
}

#[derive(Serialize)]
struct BulkDeleteBody {
    ids: Vec<i64>,
}

pub async fn run(cmd: BlocklistCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        BlocklistCommand::List {
            page: page_num,
            page_size,
            sort_key,
            sort_dir,
        } => {
            let page_str = page_num.to_string();
            let page_size_str = page_size.to_string();
            let mut query: Vec<(&str, &str)> =
                vec![("page", &page_str), ("pageSize", &page_size_str)];
            if let Some(ref key) = sort_key {
                query.push(("sortKey", key));
            }
            if let Some(ref dir) = sort_dir {
                query.push(("sortDirection", dir));
            }
            let page: BlocklistPage = api.get_with_query("/api/v3/blocklist", &query).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = page
                        .records
                        .iter()
                        .map(|r| {
                            vec![
                                r.id.map(|v| v.to_string()).unwrap_or_default(),
                                r.movie_id.map(|v| v.to_string()).unwrap_or_default(),
                                r.source_title.clone().unwrap_or_default(),
                                r.protocol.clone().unwrap_or_default(),
                                r.date.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["ID", "Movie ID", "Source Title", "Protocol", "Date"],
                        rows,
                    );
                    if let Some(total) = page.total_records {
                        let size = page.page_size.unwrap_or(20).max(1);
                        println!(
                            "Page {}/{} ({} total records)",
                            page.page.unwrap_or(1),
                            (total + size - 1) / size,
                            total
                        );
                    }
                }
                OutputFormat::Json => output::print_json(&page)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = page
                        .records
                        .iter()
                        .map(|r| r.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        BlocklistCommand::Movie { movie_id } => {
            let movie_id_str = movie_id.to_string();
            let items: Vec<BlocklistRecord> = api
                .get_with_query(
                    "/api/v3/blocklist/movie",
                    &[("movieId", movie_id_str.as_str())],
                )
                .await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|r| {
                            vec![
                                r.id.map(|v| v.to_string()).unwrap_or_default(),
                                r.source_title.clone().unwrap_or_default(),
                                r.protocol.clone().unwrap_or_default(),
                                r.date.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Source Title", "Protocol", "Date"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|r| r.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        BlocklistCommand::Delete { id } => {
            api.delete(&format!("/api/v3/blocklist/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted blocklist entry {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        BlocklistCommand::BulkDelete { ids } => {
            let body = BulkDeleteBody { ids: ids.clone() };
            api.delete_with_body("/api/v3/blocklist/bulk", &body)
                .await?;
            match fmt {
                OutputFormat::Table => println!("Deleted {} blocklist entries", ids.len()),
                OutputFormat::Json => println!(r#"{{"deleted": {}}}"#, ids.len()),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
