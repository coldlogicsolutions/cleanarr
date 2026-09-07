use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum QueueCommand {
    /// List all queue items
    List {
        /// Page number
        #[arg(long, default_value = "1")]
        page: i64,
        /// Page size
        #[arg(long, default_value = "20")]
        page_size: i64,
        /// Sort key (e.g. timeleft, title, status)
        #[arg(long)]
        sort_key: Option<String>,
        /// Sort direction (ascending or descending)
        #[arg(long)]
        sort_dir: Option<String>,
    },
    /// Get queue item details
    Details,
    /// Get queue status
    Status,
    /// Delete a queue item
    Delete {
        /// Queue item ID
        id: i64,
        /// Remove from download client
        #[arg(long)]
        remove_from_client: bool,
        /// Add to blocklist
        #[arg(long)]
        blocklist: bool,
    },
    /// Grab a pending queue item
    Grab {
        /// Queue item ID
        id: i64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueuePagingResource {
    page: Option<i64>,
    page_size: Option<i64>,
    total_records: Option<i64>,
    records: Vec<QueueResource>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueueResource {
    id: Option<i64>,
    movie_id: Option<i64>,
    title: Option<String>,
    status: Option<String>,
    tracked_download_status: Option<String>,
    tracked_download_state: Option<String>,
    size: Option<f64>,
    sizeleft: Option<f64>,
    timeleft: Option<String>,
    estimated_completion_time: Option<String>,
    download_client: Option<String>,
    indexer: Option<String>,
    protocol: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueueStatusResource {
    total_count: Option<i64>,
    count: Option<i64>,
    unknown_count: Option<i64>,
    errors: Option<bool>,
    warnings: Option<bool>,
    unknown_errors: Option<bool>,
    unknown_warnings: Option<bool>,
}

pub async fn run(cmd: QueueCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        QueueCommand::List {
            page,
            page_size,
            sort_key,
            sort_dir,
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
            let result: QueuePagingResource = api.get_with_query("/api/v3/queue", &query).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = result
                        .records
                        .iter()
                        .map(|q| {
                            vec![
                                q.id.map(|i| i.to_string()).unwrap_or_default(),
                                q.title.clone().unwrap_or_default(),
                                q.status.clone().unwrap_or_default(),
                                q.timeleft.clone().unwrap_or_default(),
                                format_size_f64(q.sizeleft.unwrap_or(0.0)),
                                format_size_f64(q.size.unwrap_or(0.0)),
                                q.download_client.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Title",
                            "Status",
                            "Time Left",
                            "Size Left",
                            "Size",
                            "Client",
                        ],
                        rows,
                    );
                    if let Some(total) = result.total_records {
                        println!(
                            "Page {}/{} ({} total records)",
                            result.page.unwrap_or(1),
                            (total + result.page_size.unwrap_or(20) - 1)
                                / result.page_size.unwrap_or(20).max(1),
                            total
                        );
                    }
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&result.records)?)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = result
                        .records
                        .iter()
                        .map(|q| q.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        QueueCommand::Details => {
            let items: Vec<QueueResource> = api.get("/api/v3/queue/details").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|q| {
                            vec![
                                q.id.map(|i| i.to_string()).unwrap_or_default(),
                                q.title.clone().unwrap_or_default(),
                                q.status.clone().unwrap_or_default(),
                                q.tracked_download_status.clone().unwrap_or_default(),
                                q.tracked_download_state.clone().unwrap_or_default(),
                                q.timeleft.clone().unwrap_or_default(),
                                q.indexer.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Title",
                            "Status",
                            "DL Status",
                            "DL State",
                            "Time Left",
                            "Indexer",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&items)?)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|q| q.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        QueueCommand::Status => {
            let status: QueueStatusResource = api.get("/api/v3/queue/status").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        status
                            .total_count
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        status.count.map(|i| i.to_string()).unwrap_or_default(),
                        status
                            .unknown_count
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        if status.errors.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        if status.warnings.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                    ]];
                    output::print_table(&["Total", "Count", "Unknown", "Errors", "Warnings"], rows);
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&status)?)?,
                OutputFormat::Quiet => {
                    println!("{}", status.total_count.unwrap_or(0));
                }
            }
            Ok(())
        }
        QueueCommand::Delete {
            id,
            remove_from_client,
            blocklist,
        } => {
            let mut query = vec![];
            if remove_from_client {
                query.push(("removeFromClient", "true"));
            }
            if blocklist {
                query.push(("blocklist", "true"));
            }
            api.delete_with_query(&format!("/api/v3/queue/{id}"), &query)
                .await?;
            match fmt {
                OutputFormat::Table => println!("Deleted queue item {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        QueueCommand::Grab { id } => {
            let result: serde_json::Value =
                api.post_empty(&format!("/api/v3/queue/grab/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Grabbed queue item {id}"),
                OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
    }
}

fn format_size_f64(bytes: f64) -> String {
    const KB: f64 = 1_000.0;
    const MB: f64 = 1_000_000.0;
    const GB: f64 = 1_000_000_000.0;
    const TB: f64 = 1_000_000_000_000.0;

    if bytes >= TB {
        format!("{:.1} TB", bytes / TB)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{:.0} B", bytes)
    }
}
