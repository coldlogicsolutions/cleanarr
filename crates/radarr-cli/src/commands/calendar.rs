use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputFormat};
use radarr_api::RadarrClient;

#[derive(Subcommand)]
pub enum CalendarCommand {
    /// List calendar events
    List {
        /// Start date (e.g. 2024-01-01)
        #[arg(long)]
        start: Option<String>,
        /// End date (e.g. 2024-12-31)
        #[arg(long)]
        end: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarMovie {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub year: Option<i64>,
    pub status: Option<String>,
    pub monitored: Option<bool>,
    pub has_file: Option<bool>,
    pub physical_release: Option<String>,
    pub digital_release: Option<String>,
    pub in_cinemas: Option<String>,
}

pub async fn run(cmd: CalendarCommand, api: &RadarrClient, fmt: OutputFormat) -> Result<()> {
    match cmd {
        CalendarCommand::List { start, end } => {
            let mut query: Vec<(&str, &str)> = vec![];
            if let Some(ref s) = start {
                query.push(("start", s.as_str()));
            }
            if let Some(ref e) = end {
                query.push(("end", e.as_str()));
            }
            let movies: Vec<CalendarMovie> = if query.is_empty() {
                api.get("/api/v3/calendar").await?
            } else {
                api.get_with_query("/api/v3/calendar", &query).await?
            };
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = movies
                        .iter()
                        .map(|m| {
                            vec![
                                m.id.map(|i| i.to_string()).unwrap_or_default(),
                                m.title.clone().unwrap_or_default(),
                                m.year.map(|y| y.to_string()).unwrap_or_default(),
                                m.status.clone().unwrap_or_default(),
                                if m.monitored.unwrap_or(false) {
                                    "Yes"
                                } else {
                                    "No"
                                }
                                .to_string(),
                                m.in_cinemas.clone().unwrap_or_default(),
                                m.physical_release.clone().unwrap_or_default(),
                                m.digital_release.clone().unwrap_or_default(),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &[
                            "ID",
                            "Title",
                            "Year",
                            "Status",
                            "Monitored",
                            "In Cinemas",
                            "Physical",
                            "Digital",
                        ],
                        rows,
                    );
                }
                OutputFormat::Json => output::print_json(&movies)?,
                OutputFormat::Quiet => {
                    let ids: Vec<String> = movies
                        .iter()
                        .map(|m| m.id.map(|i| i.to_string()).unwrap_or_default())
                        .collect();
                    output::print_quiet(&ids);
                }
            }
            Ok(())
        }
    }
}
