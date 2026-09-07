use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ConfigFile {
    #[serde(default)]
    pub radarr: RadarrFileConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub cleanup: CleanupConfig,
    #[serde(default)]
    pub plex: PlexFileConfig,
    #[serde(default)]
    pub sonarr: SonarrFileConfig,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct RadarrFileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct OutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CleanupConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_max_size: Option<String>,
    /// Tag names (Radarr movie tags / Sonarr series tags) that cleanup must never delete.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PlexFileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Per-install identifier sent as X-Plex-Client-Identifier during login.
    /// Generated on first `plex login` so each install shows up as its own device on plex.tv.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlexConfig {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SonarrFileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SonarrConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Debug)]
pub struct AppConfig {
    pub url: String,
    pub api_key: String,
    pub output_format: crate::output::OutputFormat,
    pub plex: Option<PlexConfig>,
    pub sonarr: Option<SonarrConfig>,
    /// Protected tag names from config file and CLEANARR_PROTECTED_TAGS (deduplicated).
    pub protected_tags: Vec<String>,
    /// Cleanup defaults from env (CLEANARR_CLEANUP_*) over config file.
    pub cleanup_defaults: CleanupDefaults,
}

#[derive(Debug, Clone, Default)]
pub struct CleanupDefaults {
    pub max_size: Option<String>,
    pub strategy: Option<String>,
}

/// Split a comma-separated tag list, trimming whitespace and dropping empties.
pub fn parse_tag_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

/// Union tag lists, deduplicating case-insensitively while preserving first-seen order.
pub fn merge_tags<'a>(lists: impl IntoIterator<Item = &'a [String]>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for list in lists {
        for t in list {
            if seen.insert(t.to_lowercase()) {
                out.push(t.clone());
            }
        }
    }
    out
}

impl AppConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        cli_url: Option<&str>,
        cli_api_key: Option<&str>,
        cli_output: Option<&str>,
        cli_config_path: Option<&str>,
        cli_plex_url: Option<&str>,
        cli_plex_token: Option<&str>,
        cli_sonarr_url: Option<&str>,
        cli_sonarr_api_key: Option<&str>,
    ) -> Result<Self> {
        let config_file = load_config_file(cli_config_path)?;

        let url = cli_url
            .map(String::from)
            .or_else(|| std::env::var("RADARR_URL").ok())
            .or(config_file.radarr.url)
            .context("Radarr URL is required. Set via --radarr-url, RADARR_URL env var, or [radarr] url in the config file.")?;

        let api_key = cli_api_key
            .map(String::from)
            .or_else(|| std::env::var("RADARR_API_KEY").ok())
            .or(config_file.radarr.api_key)
            .context(
                "Radarr API key is required. Set via --radarr-api-key, RADARR_API_KEY env var, or [radarr] api_key in the config file.",
            )?;

        let format_str = cli_output
            .map(String::from)
            .or_else(|| std::env::var("CLEANARR_OUTPUT").ok())
            .or(config_file.output.format)
            .unwrap_or_else(|| "table".to_string());

        let output_format = format_str.parse()?;

        let plex_url = cli_plex_url
            .map(String::from)
            .or_else(|| std::env::var("PLEX_URL").ok())
            .or(config_file.plex.url);

        let plex_token = cli_plex_token
            .map(String::from)
            .or_else(|| std::env::var("PLEX_TOKEN").ok())
            .or(config_file.plex.token);

        let plex = match (plex_url, plex_token) {
            (Some(url), Some(token)) => Some(PlexConfig { url, token }),
            _ => None,
        };

        let sonarr_url = cli_sonarr_url
            .map(String::from)
            .or_else(|| std::env::var("SONARR_URL").ok())
            .or(config_file.sonarr.url);

        let sonarr_api_key = cli_sonarr_api_key
            .map(String::from)
            .or_else(|| std::env::var("SONARR_API_KEY").ok())
            .or(config_file.sonarr.api_key);

        let sonarr = match (sonarr_url, sonarr_api_key) {
            (Some(url), Some(api_key)) => Some(SonarrConfig { url, api_key }),
            _ => None,
        };

        let env_tags = std::env::var("CLEANARR_PROTECTED_TAGS")
            .ok()
            .map(|v| parse_tag_list(&v))
            .unwrap_or_default();
        let protected_tags = merge_tags([
            config_file.cleanup.protected_tags.as_slice(),
            env_tags.as_slice(),
        ]);

        let cleanup_defaults = CleanupDefaults {
            max_size: std::env::var("CLEANARR_CLEANUP_MAX_SIZE")
                .ok()
                .or(config_file.cleanup.default_max_size),
            strategy: std::env::var("CLEANARR_CLEANUP_STRATEGY")
                .ok()
                .or(config_file.cleanup.default_strategy),
        };

        Ok(Self {
            url,
            api_key,
            output_format,
            plex,
            sonarr,
            protected_tags,
            cleanup_defaults,
        })
    }
}

pub fn config_path(cli_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = cli_path {
        Some(PathBuf::from(p))
    } else {
        directories::ProjectDirs::from("", "", "cleanarr")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }
}

#[derive(Default)]
pub struct ConfigUpdates {
    pub radarr_url: Option<String>,
    pub radarr_api_key: Option<String>,
    pub plex_url: Option<String>,
    pub plex_token: Option<String>,
    pub plex_client_id: Option<String>,
    pub sonarr_url: Option<String>,
    pub sonarr_api_key: Option<String>,
    /// Replaces the stored list when Some.
    pub protected_tags: Option<Vec<String>>,
    pub default_max_size: Option<String>,
    pub default_strategy: Option<String>,
}

impl ConfigUpdates {
    pub fn is_empty(&self) -> bool {
        self.radarr_url.is_none()
            && self.radarr_api_key.is_none()
            && self.plex_url.is_none()
            && self.plex_token.is_none()
            && self.plex_client_id.is_none()
            && self.sonarr_url.is_none()
            && self.sonarr_api_key.is_none()
            && self.protected_tags.is_none()
            && self.default_max_size.is_none()
            && self.default_strategy.is_none()
    }
}

pub fn save_to_config(cli_path: Option<&str>, updates: ConfigUpdates) -> Result<PathBuf> {
    let path = config_path(cli_path).context("Could not determine config directory")?;

    let mut config = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Invalid config file: {}", path.display()))?
    } else {
        ConfigFile::default()
    };

    if let Some(url) = updates.radarr_url {
        config.radarr.url = Some(url);
    }
    if let Some(api_key) = updates.radarr_api_key {
        config.radarr.api_key = Some(api_key);
    }
    if let Some(url) = updates.plex_url {
        config.plex.url = Some(url);
    }
    if let Some(token) = updates.plex_token {
        config.plex.token = Some(token);
    }
    if let Some(id) = updates.plex_client_id {
        config.plex.client_id = Some(id);
    }
    if let Some(url) = updates.sonarr_url {
        config.sonarr.url = Some(url);
    }
    if let Some(key) = updates.sonarr_api_key {
        config.sonarr.api_key = Some(key);
    }
    if let Some(tags) = updates.protected_tags {
        config.cleanup.protected_tags = tags;
    }
    if let Some(v) = updates.default_max_size {
        config.cleanup.default_max_size = Some(v);
    }
    if let Some(v) = updates.default_strategy {
        config.cleanup.default_strategy = Some(v);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(&config).context("Failed to serialize config")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(path)
}

pub fn load_config_file(cli_path: Option<&str>) -> Result<ConfigFile> {
    let path = if let Some(p) = cli_path {
        PathBuf::from(p)
    } else if let Some(dirs) = directories::ProjectDirs::from("", "", "cleanarr") {
        dirs.config_dir().join("config.toml")
    } else {
        return Ok(ConfigFile::default());
    };

    if !path.exists() {
        if cli_path.is_some() {
            bail!("Config file not found: {}", path.display());
        }
        return Ok(ConfigFile::default());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: ConfigFile = toml::from_str(&content)
        .with_context(|| format!("Invalid config file: {}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_list_trims_and_drops_empty() {
        assert_eq!(parse_tag_list("keep, kids ,,"), vec!["keep", "kids"]);
        assert!(parse_tag_list("").is_empty());
    }

    #[test]
    fn merge_tags_dedupes_case_insensitively() {
        let a = vec!["Keep".to_string(), "kids".to_string()];
        let b = vec!["keep".to_string(), "4k".to_string()];
        assert_eq!(
            merge_tags([a.as_slice(), b.as_slice()]),
            vec!["Keep", "kids", "4k"]
        );
    }

    #[test]
    fn protected_tags_round_trip_toml() {
        let mut cfg = ConfigFile::default();
        cfg.cleanup.protected_tags = vec!["keep".into()];
        let text = toml::to_string_pretty(&cfg).unwrap();
        assert!(text.contains("protected_tags = [\"keep\"]"), "{text}");
        let back: ConfigFile = toml::from_str(&text).unwrap();
        assert_eq!(back.cleanup.protected_tags, vec!["keep"]);
    }
}
