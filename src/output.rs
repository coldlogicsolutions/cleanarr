use anyhow::{Result, bail};
use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Quiet,
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "quiet" => Ok(Self::Quiet),
            _ => bail!("Invalid output format '{}'. Use: table, json, quiet", s),
        }
    }
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let mut table = Table::new();
    table.load_style(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers.iter().map(|h| h.to_string()));
    for row in rows {
        table.add_row(row);
    }
    println!("{table}");
}

pub fn print_quiet(values: &[String]) {
    for v in values {
        println!("{v}");
    }
}
