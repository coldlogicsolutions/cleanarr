use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Message shown when a command needs Radarr but no Radarr URL is configured.
pub const RADARR_URL_REQUIRED: &str = "Radarr URL is required. Set via --radarr-url, RADARR_URL env var, or [radarr] url in the config file.";
/// Message shown when a command needs Radarr but no Radarr API key is configured.
pub const RADARR_API_KEY_REQUIRED: &str = "Radarr API key is required. Set via --radarr-api-key, RADARR_API_KEY env var, or [radarr] api_key in the config file.";

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

#[derive(Debug, Clone)]
pub struct RadarrConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Debug)]
pub struct AppConfig {
    /// Radarr connection, when both a URL and an API key are configured.
    /// Plex-only commands run without it.
    pub radarr: Option<RadarrConfig>,
    /// Why `radarr` is `None`, so the original message can be reported lazily.
    radarr_error: Option<&'static str>,
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

/// Union tag lists, trimming each entry, dropping empties, and deduplicating
/// case-insensitively while preserving first-seen order.
///
/// This is the single normalisation point for all three tag sources
/// (config file, `CLEANARR_PROTECTED_TAGS`, `--exclude-tag`).
pub fn merge_tags<'a>(lists: impl IntoIterator<Item = &'a [String]>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for list in lists {
        for t in list {
            let t = t.trim();
            if t.is_empty() {
                continue;
            }
            if seen.insert(t.to_lowercase()) {
                out.push(t.to_string());
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
            .or(config_file.radarr.url);

        let api_key = cli_api_key
            .map(String::from)
            .or_else(|| std::env::var("RADARR_API_KEY").ok())
            .or(config_file.radarr.api_key);

        // Radarr is optional: Plex-only commands must run without it. Commands that
        // need it call `require_radarr()` and get the original message.
        let (radarr, radarr_error) = match (url, api_key) {
            (Some(url), Some(api_key)) => (Some(RadarrConfig { url, api_key }), None),
            (None, _) => (None, Some(RADARR_URL_REQUIRED)),
            (Some(_), None) => (None, Some(RADARR_API_KEY_REQUIRED)),
        };

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
            radarr,
            radarr_error,
            output_format,
            plex,
            sonarr,
            protected_tags,
            cleanup_defaults,
        })
    }

    /// Radarr connection for commands that need it, or the original
    /// "Radarr URL/API key is required" error.
    pub fn require_radarr(&self) -> Result<&RadarrConfig> {
        match (&self.radarr, self.radarr_error) {
            (Some(r), _) => Ok(r),
            (None, Some(msg)) => bail!(msg),
            (None, None) => bail!(RADARR_URL_REQUIRED),
        }
    }
}

/// Default config path: `$XDG_CONFIG_HOME/<app>/config.toml`, falling back to
/// `~/.config/<app>/config.toml`. Applied on every platform.
///
/// Per the XDG spec a relative `XDG_CONFIG_HOME` is ignored.
pub fn default_config_path(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
    app: &str,
) -> Option<PathBuf> {
    let base = match xdg_config_home {
        Some(p) if p.is_absolute() => p,
        _ => home?.join(".config"),
    };
    Some(base.join(app).join("config.toml"))
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
}

pub fn config_path(cli_path: Option<&str>) -> Option<PathBuf> {
    match cli_path {
        Some(p) => Some(PathBuf::from(p)),
        None => default_config_path(env_path("XDG_CONFIG_HOME"), home_dir(), "cleanarr"),
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

/// Fetch (creating if needed) a top-level table, so unknown keys and comments
/// elsewhere in the document survive untouched.
fn doc_table<'a>(doc: &'a mut toml_edit::DocumentMut, name: &str) -> &'a mut toml_edit::Table {
    if !doc.contains_table(name) {
        let mut table = toml_edit::Table::new();
        table.set_implicit(false);
        doc.insert(name, toml_edit::Item::Table(table));
    }
    doc[name]
        .as_table_mut()
        .expect("just inserted a table at this key")
}

fn apply_updates(doc: &mut toml_edit::DocumentMut, updates: ConfigUpdates) {
    for (table, key, val) in [
        ("radarr", "url", updates.radarr_url),
        ("radarr", "api_key", updates.radarr_api_key),
        ("plex", "url", updates.plex_url),
        ("plex", "token", updates.plex_token),
        ("plex", "client_id", updates.plex_client_id),
        ("sonarr", "url", updates.sonarr_url),
        ("sonarr", "api_key", updates.sonarr_api_key),
        ("cleanup", "default_max_size", updates.default_max_size),
        ("cleanup", "default_strategy", updates.default_strategy),
    ] {
        if let Some(val) = val {
            doc_table(doc, table)[key] = toml_edit::value(val);
        }
    }

    if let Some(tags) = updates.protected_tags {
        let arr: toml_edit::Array = tags.into_iter().collect();
        doc_table(doc, "cleanup")["protected_tags"] = toml_edit::value(arr);
    }
}

pub fn save_to_config(cli_path: Option<&str>, updates: ConfigUpdates) -> Result<PathBuf> {
    let path = config_path(cli_path).context("Could not determine config directory")?;

    let existing = if path.exists() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?
    } else {
        String::new()
    };

    let mut doc: toml_edit::DocumentMut = existing
        .parse()
        .with_context(|| format!("Invalid config file: {}", path.display()))?;
    apply_updates(&mut doc, updates);

    write_private_atomic(&path, &doc.to_string())?;

    Ok(path)
}

/// Write `content` to `path` via a temp file in the same directory followed by a
/// rename, so an interrupted write leaves the previous file intact. On Unix the
/// file is created with mode 0600 because it holds API keys and the Plex token.
pub(crate) fn write_private_atomic(path: &Path, content: &str) -> Result<()> {
    use std::io::Write;

    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".to_string());
    let tmp = parent.join(format!(".{}.{}.tmp", file_name, std::process::id()));

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }

    let write = (|| -> std::io::Result<()> {
        let mut file = opts.open(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()
    })();
    if let Err(e) = write {
        let _ = std::fs::remove_file(&tmp);
        return Err(anyhow::Error::from(e))
            .with_context(|| format!("Failed to write config file: {}", path.display()));
    }

    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(anyhow::Error::from(e))
            .with_context(|| format!("Failed to write config file: {}", path.display()));
    }

    Ok(())
}

pub fn load_config_file(cli_path: Option<&str>) -> Result<ConfigFile> {
    let Some(path) = config_path(cli_path) else {
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
    fn merge_tags_trims_and_drops_empty() {
        // `--exclude-tag "keep, kids"` splits into ["keep", " kids"].
        let cli = vec!["keep".to_string(), " kids".to_string(), "  ".to_string()];
        let env = vec![" KIDS ".to_string(), "4k\t".to_string()];
        assert_eq!(
            merge_tags([cli.as_slice(), env.as_slice()]),
            vec!["keep", "kids", "4k"]
        );
    }

    #[test]
    fn default_config_path_prefers_xdg_then_home() {
        assert_eq!(
            default_config_path(
                Some(PathBuf::from("/xdg")),
                Some(PathBuf::from("/home/u")),
                "cleanarr"
            ),
            Some(PathBuf::from("/xdg/cleanarr/config.toml"))
        );
        assert_eq!(
            default_config_path(None, Some(PathBuf::from("/home/u")), "cleanarr"),
            Some(PathBuf::from("/home/u/.config/cleanarr/config.toml"))
        );
        // A relative XDG_CONFIG_HOME is ignored per the XDG spec.
        assert_eq!(
            default_config_path(
                Some(PathBuf::from("relative")),
                Some(PathBuf::from("/home/u")),
                "radarr-cli"
            ),
            Some(PathBuf::from("/home/u/.config/radarr-cli/config.toml"))
        );
        assert_eq!(default_config_path(None, None, "cleanarr"), None);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cleanarr-test-{}-{}", name, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_preserves_comments_and_unknown_keys() {
        let dir = temp_dir("save-preserve");
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "# my notes\n[cleanup]\nnotify_email = \"me@example.com\"\n",
        )
        .unwrap();

        let updates = ConfigUpdates {
            plex_token: Some("TOKEN".into()),
            protected_tags: Some(vec!["keep".into()]),
            ..Default::default()
        };
        save_to_config(Some(path.to_str().unwrap()), updates).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# my notes"), "{text}");
        assert!(text.contains("notify_email = \"me@example.com\""), "{text}");

        let parsed: ConfigFile = toml::from_str(&text).unwrap();
        assert_eq!(parsed.plex.token.as_deref(), Some("TOKEN"));
        assert_eq!(parsed.cleanup.protected_tags, vec!["keep"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn save_creates_file_with_mode_600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("save-mode");
        let path = dir.join("config.toml");
        let updates = ConfigUpdates {
            radarr_api_key: Some("SECRET".into()),
            ..Default::default()
        };
        save_to_config(Some(path.to_str().unwrap()), updates).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode was {mode:o}");

        let parsed: ConfigFile = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed.radarr.api_key.as_deref(), Some("SECRET"));

        std::fs::remove_dir_all(&dir).ok();
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
