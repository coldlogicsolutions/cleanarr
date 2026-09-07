use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum RemotePathMappingCommand {
    /// List all remote path mappings
    List,
    /// Get a remote path mapping by ID
    Get {
        /// Remote path mapping ID
        id: i64,
    },
    /// Create a remote path mapping
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a remote path mapping
    Update {
        /// Remote path mapping ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a remote path mapping
    Delete {
        /// Remote path mapping ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemotePathMappingResource {
    id: Option<i64>,
    host: Option<String>,
    remote_path: Option<String>,
    local_path: Option<String>,
}

pub async fn run(
    cmd: RemotePathMappingCommand,
    api: &RadarrClient,
    fmt: OutputFormat,
) -> Result<()> {
    match cmd {
        RemotePathMappingCommand::List => {
            let items: Vec<RemotePathMappingResource> =
                api.get("/api/v3/remotepathmapping").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|m| {
                            vec![
                                m.id.map(|v| v.to_string()).unwrap_or_default(),
                                m.host.clone().unwrap_or_default(),
                                m.remote_path.clone().unwrap_or_default(),
                                m.local_path.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Host", "Remote Path", "Local Path"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|m| m.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        RemotePathMappingCommand::Get { id } => {
            let item: serde_json::Value =
                api.get(&format!("/api/v3/remotepathmapping/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
        RemotePathMappingCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: RemotePathMappingResource =
                api.post("/api/v3/remotepathmapping", &body).await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Created remote path mapping (ID: {})",
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        RemotePathMappingCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let item: RemotePathMappingResource = api
                .put(&format!("/api/v3/remotepathmapping/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => println!(
                    "Updated remote path mapping (ID: {})",
                    item.id.map(|v| v.to_string()).unwrap_or_default()
                ),
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => {
                    println!("{}", item.id.map(|v| v.to_string()).unwrap_or_default())
                }
            }
            Ok(())
        }
        RemotePathMappingCommand::Delete { id } => {
            api.delete(&format!("/api/v3/remotepathmapping/{id}"))
                .await?;
            match fmt {
                OutputFormat::Table => println!("Deleted remote path mapping {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
