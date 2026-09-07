use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum QualityDefinitionCommand {
    /// List all quality definitions
    List,
    /// Get a quality definition by ID
    Get {
        /// Quality definition ID
        id: i64,
    },
    /// Update a quality definition
    Update {
        /// Quality definition ID
        id: i64,
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QualityDefinitionResource {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub weight: Option<i64>,
    pub min_size: Option<f64>,
    pub max_size: Option<f64>,
    pub preferred_size: Option<f64>,
}

fn format_opt_f64(v: Option<f64>) -> String {
    v.map(|f| format!("{f:.1}")).unwrap_or_default()
}

pub async fn run(
    cmd: QualityDefinitionCommand,
    api: &RadarrClient,
    fmt: OutputFormat,
) -> Result<()> {
    match cmd {
        QualityDefinitionCommand::List => {
            let defs: Vec<QualityDefinitionResource> = api.get("/api/v3/qualitydefinition").await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = defs
                        .iter()
                        .map(|d| {
                            vec![
                                d.id.map(|i| i.to_string()).unwrap_or_default(),
                                d.title.clone().unwrap_or_default(),
                                d.weight.map(|i| i.to_string()).unwrap_or_default(),
                                format_opt_f64(d.min_size),
                                format_opt_f64(d.max_size),
                                format_opt_f64(d.preferred_size),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["ID", "Title", "Weight", "Min Size", "Max Size", "Preferred"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&defs)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = defs
                        .iter()
                        .map(|d| d.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        QualityDefinitionCommand::Get { id } => {
            let def: QualityDefinitionResource =
                api.get(&format!("/api/v3/qualitydefinition/{id}")).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows = vec![vec![
                        def.id.map(|i| i.to_string()).unwrap_or_default(),
                        def.title.clone().unwrap_or_default(),
                        def.weight.map(|i| i.to_string()).unwrap_or_default(),
                        format_opt_f64(def.min_size),
                        format_opt_f64(def.max_size),
                        format_opt_f64(def.preferred_size),
                    ]];
                    output::print_table(
                        &["ID", "Title", "Weight", "Min Size", "Max Size", "Preferred"],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&def)?,
                OutputFormat::Quiet => {
                    println!("{}", def.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
        QualityDefinitionCommand::Update { id, json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let def: QualityDefinitionResource = api
                .put(&format!("/api/v3/qualitydefinition/{id}"), &body)
                .await?;
            match fmt {
                OutputFormat::Table => {
                    println!(
                        "Updated: {} (ID: {})",
                        def.title.unwrap_or_default(),
                        def.id.map(|i| i.to_string()).unwrap_or_default()
                    );
                }
                OutputFormat::Json => output::print_json(&def)?,
                OutputFormat::Quiet => {
                    println!("{}", def.id.map(|i| i.to_string()).unwrap_or_default());
                }
            }
            Ok(())
        }
    }
}
