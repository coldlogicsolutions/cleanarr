use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum SystemCommand {
    /// Get system health
    Health,
    /// Get disk space information
    Diskspace,
    /// Get system status
    Status,
    /// Backup management
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// Task management
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// Log management
    Log {
        #[command(subcommand)]
        command: LogCommand,
    },
}

#[derive(Subcommand)]
pub enum BackupCommand {
    /// List backups
    List,
    /// Delete a backup
    Delete {
        /// Backup ID
        id: i64,
    },
    /// Restore a backup
    Restore {
        /// Backup ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum TaskCommand {
    /// List tasks
    List,
    /// Get a task
    Get {
        /// Task ID
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum LogCommand {
    /// List log entries
    List {
        /// Page number
        #[arg(long, default_value = "1")]
        page: i64,
        /// Page size
        #[arg(long, default_value = "20")]
        page_size: i64,
        /// Sort key
        #[arg(long)]
        sort_key: Option<String>,
        /// Sort direction (ascending or descending)
        #[arg(long)]
        sort_dir: Option<String>,
    },
    /// List log files
    Files,
    /// Get a log file
    File {
        /// Log file name
        name: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResource {
    id: Option<i64>,
    source: Option<String>,
    r#type: Option<String>,
    message: Option<String>,
    wiki_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiskSpaceResource {
    id: Option<i64>,
    path: Option<String>,
    label: Option<String>,
    free_space: Option<i64>,
    total_space: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemStatusResource {
    app_name: Option<String>,
    instance_name: Option<String>,
    version: Option<String>,
    build_time: Option<String>,
    is_debug: Option<bool>,
    is_production: Option<bool>,
    is_docker: Option<bool>,
    os_name: Option<String>,
    os_version: Option<String>,
    branch: Option<String>,
    database_type: Option<String>,
    database_version: Option<String>,
    authentication: Option<String>,
    url_base: Option<String>,
    runtime_version: Option<String>,
    runtime_name: Option<String>,
    start_time: Option<String>,
    startup_path: Option<String>,
    app_data: Option<String>,
    package_version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupResource {
    id: Option<i64>,
    name: Option<String>,
    path: Option<String>,
    r#type: Option<String>,
    size: Option<i64>,
    time: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskResource {
    id: Option<i64>,
    name: Option<String>,
    task_name: Option<String>,
    interval: Option<i64>,
    last_execution: Option<String>,
    last_start_time: Option<String>,
    next_execution: Option<String>,
    last_duration: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogPagingResource {
    page: Option<i64>,
    page_size: Option<i64>,
    total_records: Option<i64>,
    records: Vec<LogResource>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogResource {
    id: Option<i64>,
    time: Option<String>,
    level: Option<String>,
    logger: Option<String>,
    message: Option<String>,
    exception: Option<String>,
    exception_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogFileResource {
    id: Option<i64>,
    filename: Option<String>,
    last_write_time: Option<String>,
    contents_url: Option<String>,
    download_url: Option<String>,
}

pub async fn run(cmd: SystemCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        SystemCommand::Health => {
            let items: Vec<HealthResource> = api.get("/api/v3/health").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|h| {
                            vec![
                                h.source.clone().unwrap_or_default(),
                                h.r#type.clone().unwrap_or_default(),
                                h.message.clone().unwrap_or_default(),
                                h.wiki_url.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["Source", "Type", "Message", "Wiki URL"], rows);
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&items)?)?,
                OutputFormat::Quiet => {
                    let sources: Vec<String> = items
                        .iter()
                        .map(|h| h.source.clone().unwrap_or_default())
                        .collect();
                    output::print_quiet(&sources);
                }
            }
            Ok(())
        }
        SystemCommand::Diskspace => {
            let items: Vec<DiskSpaceResource> = api.get("/api/v3/diskspace").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|d| {
                            vec![
                                d.path.clone().unwrap_or_default(),
                                d.label.clone().unwrap_or_default(),
                                format_size(d.free_space.unwrap_or(0)),
                                format_size(d.total_space.unwrap_or(0)),
                                format_percent(
                                    d.free_space.unwrap_or(0),
                                    d.total_space.unwrap_or(0),
                                ),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["Path", "Label", "Free Space", "Total Space", "Free %"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&items)?)?,
                OutputFormat::Quiet => {
                    let paths: Vec<String> = items
                        .iter()
                        .map(|d| d.path.clone().unwrap_or_default())
                        .collect();
                    output::print_quiet(&paths);
                }
            }
            Ok(())
        }
        SystemCommand::Status => {
            let status: SystemStatusResource = api.get("/api/v3/system/status").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        status.app_name.clone().unwrap_or_default(),
                        status.version.clone().unwrap_or_default(),
                        status.os_name.clone().unwrap_or_default(),
                        status.branch.clone().unwrap_or_default(),
                        if status.is_docker.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        status.start_time.clone().unwrap_or_default(),
                        status.runtime_name.clone().unwrap_or_default(),
                    ]];
                    output::print_table(
                        &[
                            "App",
                            "Version",
                            "OS",
                            "Branch",
                            "Docker",
                            "Start Time",
                            "Runtime",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&status)?)?,
                OutputFormat::Quiet => {
                    println!("{}", status.version.unwrap_or_default());
                }
            }
            Ok(())
        }
        SystemCommand::Backup { command } => run_backup(command, api, fmt).await,
        SystemCommand::Task { command } => run_task(command, api, fmt).await,
        SystemCommand::Log { command } => run_log(command, api, fmt).await,
    }
}

async fn run_backup(cmd: BackupCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        BackupCommand::List => {
            let backups: Vec<BackupResource> = api.get("/api/v3/system/backup").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = backups
                        .iter()
                        .map(|b| {
                            vec![
                                b.id.map(|i| i.to_string()).unwrap_or_default(),
                                b.name.clone().unwrap_or_default(),
                                b.r#type.clone().unwrap_or_default(),
                                format_size(b.size.unwrap_or(0)),
                                b.time.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Type", "Size", "Time"], rows);
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&backups)?)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = backups
                        .iter()
                        .map(|b| b.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        BackupCommand::Delete { id } => {
            api.delete(&format!("/api/v3/system/backup/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted backup {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        BackupCommand::Restore { id } => {
            let result: serde_json::Value = api
                .post_empty(&format!("/api/v3/system/backup/restore/{id}"))
                .await?;
            match fmt {
                OutputFormat::Table => println!("Restoring backup {id}"),
                OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
    }
}

async fn run_task(cmd: TaskCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        TaskCommand::List => {
            let tasks: Vec<TaskResource> = api.get("/api/v3/system/task").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = tasks
                        .iter()
                        .map(|t| {
                            vec![
                                t.id.map(|i| i.to_string()).unwrap_or_default(),
                                t.name.clone().unwrap_or_default(),
                                t.interval.map(|i| format!("{i} min")).unwrap_or_default(),
                                t.last_execution.clone().unwrap_or_default(),
                                t.next_execution.clone().unwrap_or_default(),
                                t.last_duration.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["ID", "Name", "Interval", "Last Run", "Next Run", "Duration"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&tasks)?)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = tasks
                        .iter()
                        .map(|t| t.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        TaskCommand::Get { id } => {
            let task: TaskResource = api.get(&format!("/api/v3/system/task/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        task.id.map(|i| i.to_string()).unwrap_or_default(),
                        task.name.clone().unwrap_or_default(),
                        task.task_name.clone().unwrap_or_default(),
                        task.interval
                            .map(|i| format!("{i} min"))
                            .unwrap_or_default(),
                        task.last_execution.clone().unwrap_or_default(),
                        task.next_execution.clone().unwrap_or_default(),
                        task.last_duration.clone().unwrap_or_default(),
                    ]];
                    output::print_table(
                        &[
                            "ID",
                            "Name",
                            "Task Name",
                            "Interval",
                            "Last Run",
                            "Next Run",
                            "Duration",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&task)?)?,
                OutputFormat::Quiet => {
                    println!("{}", task.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
    }
}

async fn run_log(cmd: LogCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        LogCommand::List {
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
            let result: LogPagingResource = api.get_with_query("/api/v3/log", &query).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = result
                        .records
                        .iter()
                        .map(|l| {
                            vec![
                                l.id.map(|i| i.to_string()).unwrap_or_default(),
                                l.time.clone().unwrap_or_default(),
                                l.level.clone().unwrap_or_default(),
                                l.logger.clone().unwrap_or_default(),
                                l.message.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Time", "Level", "Logger", "Message"], rows);
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
                        .map(|l| l.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        LogCommand::Files => {
            let files: Vec<LogFileResource> = api.get("/api/v3/log/file").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = files
                        .iter()
                        .map(|f| {
                            vec![
                                f.id.map(|i| i.to_string()).unwrap_or_default(),
                                f.filename.clone().unwrap_or_default(),
                                f.last_write_time.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Filename", "Last Write Time"], rows);
                }
                OutputFormat::Json => output::print_json(&serde_json::to_value(&files)?)?,
                OutputFormat::Quiet => {
                    let names: Vec<String> = files
                        .iter()
                        .map(|f| f.filename.clone().unwrap_or_default())
                        .collect();
                    output::print_quiet(&names);
                }
            }
            Ok(())
        }
        LogCommand::File { name } => {
            // Log files are served as plain text, not JSON.
            let encoded = radarr_api::encode_path_segment(&name);
            let content = api.get_text(&format!("/api/v3/log/file/{encoded}")).await?;
            match fmt {
                OutputFormat::Table => print!("{content}"),
                OutputFormat::Json => output::print_json(&serde_json::Value::String(content))?,
                OutputFormat::Quiet => {}
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

fn format_percent(free: i64, total: i64) -> String {
    if total == 0 {
        return "N/A".to_string();
    }
    format!("{:.1}%", (free as f64 / total as f64) * 100.0)
}
