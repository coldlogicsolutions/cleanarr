use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Args)]
pub struct RenameArgs {
    /// Movie ID to preview renames for
    pub movie_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameResource {
    movie_id: Option<i64>,
    movie_file_id: Option<i64>,
    existing_path: Option<String>,
    new_path: Option<String>,
}

pub async fn run(args: RenameArgs, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    let mid_str = args.movie_id.to_string();
    let items: Vec<RenameResource> = api
        .get_with_query("/api/v3/rename", &[("movieId", mid_str.as_str())])
        .await?;
    match fmt {
        OutputFormat::Table => {
            let rows: Vec<Vec<String>> = items
                .iter()
                .map(|r| {
                    vec![
                        r.movie_file_id.map(|v| v.to_string()).unwrap_or_default(),
                        r.existing_path.clone().unwrap_or_default(),
                        r.new_path.clone().unwrap_or_default(),
                    ]
                })
                .collect();
            output::print_table(&["File ID", "Existing Path", "New Path"], rows);
        }
        OutputFormat::Json => output::print_json(&items)?,
        OutputFormat::Quiet => {
            let ids: Vec<String> = items
                .iter()
                .map(|r| r.movie_file_id.map(|v| v.to_string()).unwrap_or_default())
                .collect();
            output::print_quiet(&ids);
        }
    }
    Ok(())
}
