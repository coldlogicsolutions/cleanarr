use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ReleaseProfileCommand {
    /// List all release profiles
    List,
    /// Get a release profile by ID
    Get {
        /// Release profile ID
        id: i64,
    },
    /// Create a release profile
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Update a release profile
    Update {
        /// Release profile ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
    /// Delete a release profile
    Delete {
        /// Release profile ID
        id: i64,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseProfileResource {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub indexer_id: Option<i64>,
}

pub async fn run(cmd: ReleaseProfileCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ReleaseProfileCommand::List => {
            let profiles: Vec<ReleaseProfileResource> = api.get("/api/v3/releaseprofile").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = profiles
                        .iter()
                        .map(|p| {
                            vec![
                                p.id.map(|i| i.to_string()).unwrap_or_default(),
                                p.name.clone().unwrap_or_default(),
                                if p.enabled.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                p.indexer_id.map(|i| i.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Name", "Enabled", "Indexer ID"], rows);
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
        ReleaseProfileCommand::Get { id } => {
            let profile: ReleaseProfileResource =
                api.get(&format!("/api/v3/releaseprofile/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        profile.id.map(|i| i.to_string()).unwrap_or_default(),
                        profile.name.clone().unwrap_or_default(),
                        if profile.enabled.unwrap_or(false) {
                            "Yes"
                        } else {
                            "No"
                        }
                        .to_string(),
                        profile
                            .indexer_id
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                    ]];
                    output::print_table(&["ID", "Name", "Enabled", "Indexer ID"], rows);
                }
                OutputFormat::Json => output::print_json(&profile)?,
                OutputFormat::Quiet => {
                    println!("{}", profile.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        ReleaseProfileCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let profile: ReleaseProfileResource = api.post("/api/v3/releaseprofile", &body).await?;
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
        ReleaseProfileCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let profile: ReleaseProfileResource = api
                .put(&format!("/api/v3/releaseprofile/{id}"), &body)
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
        ReleaseProfileCommand::Delete { id } => {
            api.delete(&format!("/api/v3/releaseprofile/{id}")).await?;
            match fmt {
                OutputFormat::Table => println!("Deleted release profile {id}"),
                OutputFormat::Json => println!(r#"{{"deleted": {id}}}"#),
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
