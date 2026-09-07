use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum DelayProfileCommand {
    /// List all delay profiles
    List,
    /// Get a delay profile by ID
    Get {
        /// Delay profile ID
        id: i64,
    },
    /// Create a delay profile
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a delay profile
    Update {
        /// Delay profile ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a delay profile
    Delete {
        /// Delay profile ID
        id: i64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DelayProfileResource {
    pub id: Option<i64>,
    pub enable_usenet: Option<bool>,
    pub enable_torrent: Option<bool>,
    pub preferred_protocol: Option<String>,
    pub usenet_delay: Option<i64>,
    pub torrent_delay: Option<i64>,
    pub order: Option<i64>,
}

pub async fn run(cmd: DelayProfileCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        DelayProfileCommand::List => {
            let profiles: Vec<DelayProfileResource> = api.get("/api/v3/delayprofile").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = profiles
                        .iter()
                        .map(|p| {
                            vec![
                                p.id.map(|i| i.to_string()).unwrap_or_default(),
                                if p.enable_usenet.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                if p.enable_torrent.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                p.preferred_protocol.clone().unwrap_or_default(),
                                p.usenet_delay.map(|i| i.to_string()).unwrap_or_default(),
                                p.torrent_delay.map(|i| i.to_string()).unwrap_or_default(),
                                p.order.map(|i| i.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Usenet",
                            "Torrent",
                            "Protocol",
                            "Usenet Delay",
                            "Torrent Delay",
                            "Order",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&profiles)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = profiles
                        .iter()
                        .map(|p| p.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        DelayProfileCommand::Get { id } => {
            let profile: DelayProfileResource =
                api.get(&format!("/api/v3/delayprofile/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        profile.id.map(|i| i.to_string()).unwrap_or_default(),
                        if profile.enable_usenet.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        if profile.enable_torrent.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        profile.preferred_protocol.clone().unwrap_or_default(),
                        profile
                            .usenet_delay
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        profile
                            .torrent_delay
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        profile.order.map(|i| i.to_string()).unwrap_or_default(),
                    ]];
                    output::print_table(
                        &[
                            "ID",
                            "Usenet",
                            "Torrent",
                            "Protocol",
                            "Usenet Delay",
                            "Torrent Delay",
                            "Order",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&profile)?,
                OutputFormat::Quiet => {
                    println!("{}", profile.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        DelayProfileCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let profile: DelayProfileResource = api.post("/api/v3/delayprofile", &body).await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Created delay profile (ID: {})",
                        profile.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&profile)?,
                OutputFormat::Quiet => {
                    println!("{}", profile.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        DelayProfileCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let profile: DelayProfileResource = api
                .put(&format!("/api/v3/delayprofile/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated delay profile (ID: {})",
                        profile.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&profile)?,
                OutputFormat::Quiet => {
                    println!("{}", profile.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        DelayProfileCommand::Delete { id } => {
            api.delete(&format!("/api/v3/delayprofile/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted delay profile {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
