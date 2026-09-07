use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum HistoryCommand {
    /// List history records
    List {
        /// Page number
        #[arg(long, default_value = "1")]
        page: i64,
        /// Page size
        #[arg(long, default_value = "20")]
        page_size: i64,
        /// Sort key (e.g. date, title)
        #[arg(long)]
        sort_key: Option<String>,
        /// Sort direction (ascending or descending)
        #[arg(long)]
        sort_dir: Option<String>,
        /// Filter by event type (e.g. grabbed, downloadFolderImported, downloadFailed, deleted)
        #[arg(long)]
        event_type: Option<String>,
    },
    /// Get history for a movie
    Movie {
        /// Movie ID
        id: i64,
    },
    /// Get history since a date
    Since {
        /// Date string (e.g. 2024-01-01)
        date: String,
    },
    /// Mark a history item as failed
    Failed {
        /// History record ID
        id: i64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryPagingResource {
    page: Option<i64>,
    page_size: Option<i64>,
    total_records: Option<i64>,
    records: Vec<HistoryResource>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryResource {
    id: Option<i64>,
    movie_id: Option<i64>,
    source_title: Option<String>,
    date: Option<String>,
    download_id: Option<String>,
    event_type: Option<String>,
    quality_cutoff_not_met: Option<bool>,
}

pub async fn run(cmd: HistoryCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        HistoryCommand::List {
            page,
            page_size,
            sort_key,
            sort_dir,
            event_type,
        } => {
            let page_str = page.to_string();
            let page_size_str = page_size.to_string();
            let mut query: Vec<(&str, &str)> =
                vec![("page", &page_str), ("pageSize", &page_size_str)];
            if let Some(ref key) = sort_key {
                query.push(("sortKey", key));
            }
            if let Some(ref dir) = sort_dir {
                query.push(("sortDirection", dir));
            }
            if let Some(ref et) = event_type {
                query.push(("eventType", et));
            }
            let result: HistoryPagingResource =
                api.get_with_query("/api/v3/history", &query).await?;
            print_history_list(&result.records, fmt)?;
            if fmt == OutputFormat::Table
                && let Some(total) = result.total_records
            {
                println!(
                    "Page {}/{} ({} total records)",
                    result.page.unwrap_or(1),
                    (total + result.page_size.unwrap_or(20) - 1)
                        / result.page_size.unwrap_or(20).max(1),
                    total
                );
            }
            Ok(())
        }
        HistoryCommand::Movie { id } => {
            let id_str = id.to_string();
            let records: Vec<HistoryResource> = api
                .get_with_query("/api/v3/history/movie", &[("movieId", id_str.as_str())])
                .await?;
            print_history_list(&records, fmt)?;
            Ok(())
        }
        HistoryCommand::Since { date } => {
            let records: Vec<HistoryResource> = api
                .get_with_query("/api/v3/history/since", &[("date", date.as_str())])
                .await?;
            print_history_list(&records, fmt)?;
            Ok(())
        }
        HistoryCommand::Failed { id } => {
            let result: serde_json::Value = api
                .post_empty(&format!("/api/v3/history/failed/{id}"))
                .await?;
            match fmt {
                OutputFormat::Table => println!("Marked history item {id} as failed"),
                OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
    }
}

fn print_history_list(records: &[HistoryResource], fmt: OutputFormat) -> Result<()> {
    match fmt {
        OutputFormat::Table => {
            let rows: Vec<Vec<String>> = records
                .iter()
                .map(|h| {
                    vec![
                        h.id.map(|i| i.to_string()).unwrap_or_default(),
                        h.movie_id.map(|i| i.to_string()).unwrap_or_default(),
                        h.event_type.clone().unwrap_or_default(),
                        h.source_title.clone().unwrap_or_default(),
                        h.date.clone().unwrap_or_default(),
                        h.download_id.clone().unwrap_or_default(),
                    ]
                })
                .collect();
            output::print_table(
                &[
                    "ID",
                    "Movie ID",
                    "Event",
                    "Source Title",
                    "Date",
                    "Download ID",
                ],
                rows,
            );
        }
        OutputFormat::Json => output::print_json(&serde_json::to_value(records)?)?,
        OutputFormat::Quiet => {
            let ids: Vec<String> = records
                .iter()
                .map(|h| h.id.map(|i| i.to_string()).unwrap_or_default())
                .collect();
            output::print_quiet(&ids);
        }
    }
    Ok(())
}
