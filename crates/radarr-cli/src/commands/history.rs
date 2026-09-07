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
        /// Filter by event type: grabbed, downloadFolderImported, downloadFailed,
        /// movieFileDeleted, movieFolderImported, movieFileRenamed, downloadIgnored
        /// (a raw numeric MovieHistoryEventType is also accepted)
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
            let event_type = event_type.as_deref().map(event_type_value).transpose()?;
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

/// Radarr's `MovieHistoryEventType` enum, keyed by the name the API documents.
const EVENT_TYPES: &[(&str, i32)] = &[
    ("grabbed", 1),
    ("downloadFolderImported", 3),
    ("downloadFailed", 4),
    ("movieFileDeleted", 6),
    ("movieFolderImported", 7),
    ("movieFileRenamed", 8),
    ("downloadIgnored", 9),
];

/// Translate a user-supplied `--event-type` into the numeric value Radarr expects.
///
/// Accepts a documented event name (case-insensitively) or a raw number.
fn event_type_value(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Ok(trimmed.to_string());
    }
    EVENT_TYPES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(trimmed))
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| {
            let names: Vec<&str> = EVENT_TYPES.iter().map(|(name, _)| *name).collect();
            anyhow::anyhow!(
                "Unknown --event-type '{input}'. Expected one of: {} (or a raw numeric event type)",
                names.join(", ")
            )
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_event_names() {
        assert_eq!(event_type_value("grabbed").unwrap(), "1");
        assert_eq!(event_type_value("downloadFolderImported").unwrap(), "3");
        assert_eq!(event_type_value("downloadFailed").unwrap(), "4");
        assert_eq!(event_type_value("movieFileDeleted").unwrap(), "6");
        assert_eq!(event_type_value("movieFolderImported").unwrap(), "7");
        assert_eq!(event_type_value("movieFileRenamed").unwrap(), "8");
        assert_eq!(event_type_value("downloadIgnored").unwrap(), "9");
    }

    #[test]
    fn event_names_are_case_insensitive_and_trimmed() {
        assert_eq!(event_type_value("GRABBED").unwrap(), "1");
        assert_eq!(event_type_value(" downloadfailed ").unwrap(), "4");
    }

    #[test]
    fn raw_numbers_pass_through() {
        assert_eq!(event_type_value("9").unwrap(), "9");
        assert_eq!(event_type_value(" 3 ").unwrap(), "3");
    }

    #[test]
    fn unknown_names_error_with_the_valid_list() {
        let err = event_type_value("deleted").unwrap_err().to_string();
        assert!(err.contains("Unknown --event-type 'deleted'"), "{err}");
        assert!(err.contains("movieFileDeleted"), "{err}");
    }
}
