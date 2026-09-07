use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::output::OutputFormat;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

pub struct Config {
    pub url: String,
    pub api_key: String,
    pub output_format: OutputFormat,
}

pub fn config_path(cli_path: Option<&str>) -> Option<PathBuf> {
    match cli_path {
        Some(p) => Some(PathBuf::from(p)),
        None => directories::ProjectDirs::from("", "", "radarr-cli")
            .map(|d| d.config_dir().join("config.toml")),
    }
}

fn read_file(cli_path: Option<&str>) -> Result<ConfigFile> {
    let Some(path) = config_path(cli_path) else {
        return Ok(ConfigFile::default());
    };
    if !path.exists() {
        if cli_path.is_some() {
            bail!("Config file not found: {}", path.display());
        }
        return Ok(ConfigFile::default());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("Invalid config file: {}", path.display()))
}

/// Resolve settings with precedence CLI > env (RADARR_URL, RADARR_API_KEY, RADARR_OUTPUT) > file.
pub fn load(
    cli_url: Option<&str>,
    cli_api_key: Option<&str>,
    cli_output: Option<&str>,
    cli_config_path: Option<&str>,
) -> Result<Config> {
    // clap already applied the env vars for url/api_key/output, so CLI values include env.
    let file = read_file(cli_config_path)?;
    let url = cli_url
        .map(String::from)
        .or(file.url)
        .context("Radarr URL is required. Set via --url, RADARR_URL env var, or config file.")?;
    let api_key = cli_api_key.map(String::from).or(file.api_key).context(
        "Radarr API key is required. Set via --api-key, RADARR_API_KEY env var, or config file.",
    )?;
    let output_format = cli_output
        .map(String::from)
        .or(file.output)
        .unwrap_or_else(|| "table".to_string())
        .parse()?;
    Ok(Config {
        url,
        api_key,
        output_format,
    })
}

pub fn save(
    cli_config_path: Option<&str>,
    url: Option<String>,
    api_key: Option<String>,
) -> Result<PathBuf> {
    let path = config_path(cli_config_path).context("Could not determine config directory")?;
    let mut file = if path.exists() {
        read_file(cli_config_path)?
    } else {
        ConfigFile::default()
    };
    if url.is_some() {
        file.url = url;
    }
    if api_key.is_some() {
        file.api_key = api_key;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }
    std::fs::write(&path, toml::to_string_pretty(&file)?)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;
    Ok(path)
}
