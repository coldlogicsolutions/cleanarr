use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ExtraFileCommand {
    /// List extra files
    List {
        /// Filter by movie ID
        #[arg(long)]
        movie_id: Option<i64>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtraFileResource {
    id: Option<i64>,
    movie_id: Option<i64>,
    relative_path: Option<String>,
    extension: Option<String>,
    #[serde(rename = "type")]
    file_type: Option<String>,
}

pub async fn run(cmd: ExtraFileCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ExtraFileCommand::List { movie_id } => {
            let items: Vec<ExtraFileResource> = if let Some(mid) = movie_id {
                let mid_str = mid.to_string();
                api.get_with_query("/api/v3/extrafile", &[("movieId", mid_str.as_str())])
                    .await?
            } else {
                api.get("/api/v3/extrafile").await?
            };
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = items
                        .iter()
                        .map(|f| {
                            vec![
                                f.id.map(|v| v.to_string()).unwrap_or_default(),
                                f.movie_id.map(|v| v.to_string()).unwrap_or_default(),
                                f.relative_path.clone().unwrap_or_default(),
                                f.file_type.clone().unwrap_or_default(),
                                f.extension.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(&["ID", "Movie ID", "Path", "Type", "Extension"], rows);
                }
                OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = items
                        .iter()
                        .map(|f| f.id.map(|v| v.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
    }
}
