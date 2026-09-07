use anyhow::Result;
use clap::Args;

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Args)]
pub struct ParseArgs {
    /// Title string to parse
    pub title: String,
}

pub async fn run(args: ParseArgs, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    let result: serde_json::Value = api
        .get_with_query("/api/v3/parse", &[("title", args.title.as_str())])
        .await?;
    match fmt {
        OutputFormat::Table | OutputFormat::Json => output::print_json(&result)?,
        OutputFormat::Quiet => {}
    }
    Ok(())
}
