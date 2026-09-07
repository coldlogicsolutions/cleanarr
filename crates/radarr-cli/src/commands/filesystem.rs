use anyhow::Result;
use clap::Subcommand;

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum FilesystemCommand {
    /// Browse a file system path
    Browse {
        /// Path to browse
        #[arg(long)]
        path: Option<String>,
    },
    /// Get the type of a path
    Type {
        /// Path to check
        #[arg(long)]
        path: String,
    },
    /// List media files in a path
    MediaFiles {
        /// Path to scan
        #[arg(long)]
        path: String,
    },
}

pub async fn run(cmd: FilesystemCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        FilesystemCommand::Browse { path } => {
            let result: serde_json::Value = if let Some(p) = path {
                api.get_with_query("/api/v3/filesystem", &[("path", p.as_str())])
                    .await?
            } else {
                api.get("/api/v3/filesystem").await?
            };
            match fmt {
                OutputFormat::Table | OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        FilesystemCommand::Type { path } => {
            let result: serde_json::Value = api
                .get_with_query("/api/v3/filesystem/type", &[("path", path.as_str())])
                .await?;
            match fmt {
                OutputFormat::Table | OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        FilesystemCommand::MediaFiles { path } => {
            let result: serde_json::Value = api
                .get_with_query("/api/v3/filesystem/mediafiles", &[("path", path.as_str())])
                .await?;
            match fmt {
                OutputFormat::Table | OutputFormat::Json => output::print_json(&result)?,
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}
