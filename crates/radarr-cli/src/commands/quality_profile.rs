use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum QualityProfileCommand {
    /// List all quality profiles
    List,
    /// Get a quality profile by ID
    Get {
        /// Quality profile ID
        id: i64,
    },
    /// Create a quality profile
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a quality profile
    Update {
        /// Quality profile ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a quality profile
    Delete {
        /// Quality profile ID
        id: i64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QualityProfileResource {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub upgrade_allowed: Option<bool>,
    pub cutoff: Option<i64>,
    pub min_format_score: Option<i64>,
    pub cutoff_format_score: Option<i64>,
}

pub async fn run(cmd: QualityProfileCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        QualityProfileCommand::List => {
            let profiles: Vec<QualityProfileResource> = api.get("/api/v3/qualityprofile").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = profiles
                        .iter()
                        .map(|p| {
                            vec![
                                p.id.map(|i| i.to_string()).unwrap_or_default(),
                                p.name.clone().unwrap_or_default(),
                                if p.upgrade_allowed.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                p.cutoff.map(|i| i.to_string()).unwrap_or_default(),
                                p.min_format_score
                                    .map(|i| i.to_string())
                                    .unwrap_or_default(),
                                p.cutoff_format_score
                                    .map(|i| i.to_string())
                                    .unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Name",
                            "Upgrade",
                            "Cutoff",
                            "Min Score",
                            "Cutoff Score",
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
        QualityProfileCommand::Get { id } => {
            let profile: QualityProfileResource =
                api.get(&format!("/api/v3/qualityprofile/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        profile.id.map(|i| i.to_string()).unwrap_or_default(),
                        profile.name.clone().unwrap_or_default(),
                        if profile.upgrade_allowed.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        profile.cutoff.map(|i| i.to_string()).unwrap_or_default(),
                        profile
                            .min_format_score
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        profile
                            .cutoff_format_score
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                    ]];
                    output::print_table(
                        &[
                            "ID",
                            "Name",
                            "Upgrade",
                            "Cutoff",
                            "Min Score",
                            "Cutoff Score",
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
        QualityProfileCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let profile: QualityProfileResource = api.post("/api/v3/qualityprofile", &body).await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Created: {} (ID: {})",
                        profile.name.unwrap_or_default(),
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
        QualityProfileCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let profile: QualityProfileResource = api
                .put(&format!("/api/v3/qualityprofile/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated: {} (ID: {})",
                        profile.name.unwrap_or_default(),
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
        QualityProfileCommand::Delete { id } => {
            api.delete(&format!("/api/v3/qualityprofile/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted quality profile {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
