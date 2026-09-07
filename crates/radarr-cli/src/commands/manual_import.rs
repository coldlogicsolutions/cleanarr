use anyhow::Result;
use clap::Subcommand;

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ManualImportCommand {
    /// List manual import candidates for a folder, download or movie
    List {
        /// Folder to scan for importable files
        #[arg(long)]
        folder: Option<String>,
        /// Download client download ID to scan
        #[arg(long)]
        download_id: Option<String>,
        /// Movie ID to scan
        #[arg(long)]
        movie_id: Option<i64>,
        /// Exclude files that have already been imported (true or false)
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        filter_existing_files: bool,
    },
    /// Create a manual import
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

pub async fn run(cmd: ManualImportCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ManualImportCommand::List {
            folder,
            download_id,
            movie_id,
            filter_existing_files,
        } => {
            let movie_id_str = movie_id.map(|id| id.to_string());
            let mut query: Vec<(&str, &str)> = Vec::new();
            if let Some(ref folder) = folder {
                query.push(("folder", folder));
            }
            if let Some(ref download_id) = download_id {
                query.push(("downloadId", download_id));
            }
            if let Some(ref movie_id) = movie_id_str {
                query.push(("movieId", movie_id));
            }
            if query.is_empty() {
                anyhow::bail!(
                    "Provide at least one of --folder, --download-id or --movie-id when listing manual imports"
                );
            }
            query.push((
                "filterExistingFiles",
                if filter_existing_files {
                    "true"
                } else {
                    "false"
                },
            ));
            let items: serde_json::Value =
                api.get_with_query("/api/v3/manualimport", &query).await?;
            match fmt {
                OutputFormat::Table | OutputFormat::Json => output::print_json(&items)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        ManualImportCommand::Create { json } => {
            let body: serde_json::Value = serde_json::from_str(&json)?;
            let result: serde_json::Value = api.post("/api/v3/manualimport", &body).await?;
            match fmt {
                OutputFormat::Table => println!("Manual import created"),
                OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
