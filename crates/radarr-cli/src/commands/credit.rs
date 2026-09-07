use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum CreditCommand {
    /// List credits
    List {
        /// Filter by movie ID
        #[arg(long)]
        movie_id: Option<i64>,
    },
    /// Get a credit by ID
    Get {
        /// Credit ID
        id: i64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreditResource {
    id: Option<i64>,
    person_name: Option<String>,
    credit_tmdb_id: Option<String>,
    character: Option<String>,
    department: Option<String>,
    job: Option<String>,
    #[serde(rename = "type")]
    credit_type: Option<String>,
}

pub async fn run(cmd: CreditCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        CreditCommand::List { movie_id } => {
            let items: Vec<CreditResource> = if let Some(mid) = movie_id {
                let mid_str = mid.to_string();
                api.get_with_query("/api/v3/credit", &[("movieId", mid_str.as_str())])
                    .await?
            } else {
                api.get("/api/v3/credit").await?
            };
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|c| {
                            vec![
                                c.id.map(|v| v.to_string()).unwrap_or_default(),
                                c.person_name.clone().unwrap_or_default(),
                                c.credit_type.clone().unwrap_or_default(),
                                c.character.clone().unwrap_or_default(),
                                c.department.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Person", "Type", "Character", "Department"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|c| c.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
        CreditCommand::Get { id } => {
            let item: serde_json::Value = api.get(&format!("/api/v3/credit/{id}")).await?;
            match fmt {
                OutputFormat::Table => output::print_json(&item)?,
                OutputFormat::Json => output::print_json(&item)?,
                OutputFormat::Quiet => println!("{id}"),
            }
            Ok(())
        }
    }
}
