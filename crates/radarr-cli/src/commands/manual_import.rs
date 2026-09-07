use anyhow::Result;
use clap::Subcommand;

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum ManualImportCommand {
    /// List manual imports
    List,
    /// Create a manual import
    Create {
        /// JSON body
        #[arg(long)]
        json: String,
    },
}

pub async fn run(cmd: ManualImportCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        ManualImportCommand::List => {
            let items: serde_json::Value = api.get("/api/v3/manualimport").await?;
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
