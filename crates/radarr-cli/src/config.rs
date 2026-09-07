use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
        None => default_config_path(env_path("XDG_CONFIG_HOME"), home_dir(), "radarr-cli"),
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

    let existing = if path.exists() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?
    } else {
        String::new()
    };
    // Edit the document rather than round-tripping a struct, so comments and
    // unknown keys survive.
    let mut doc: toml_edit::DocumentMut = existing
        .parse()
        .with_context(|| format!("Invalid config file: {}", path.display()))?;
    if let Some(url) = url {
        doc["url"] = toml_edit::value(url);
    }
    if let Some(api_key) = api_key {
        doc["api_key"] = toml_edit::value(api_key);
    }

    write_private_atomic(&path, &doc.to_string())?;
    Ok(path)
}

/// Write `content` to `path` via a temp file in the same directory followed by a
/// rename, so an interrupted write leaves the previous file intact. On Unix the
/// file is created with mode 0600 because it holds the Radarr API key.
fn write_private_atomic(path: &Path, content: &str) -> Result<()> {
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

    let written = (|| -> std::io::Result<()> {
        let mut file = opts.open(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(anyhow::Error::from(e))
            .with_context(|| format!("Failed to write config file: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_path_prefers_xdg_then_home() {
        assert_eq!(
            default_config_path(
                Some(PathBuf::from("/xdg")),
                Some(PathBuf::from("/home/u")),
                "radarr-cli"
            ),
            Some(PathBuf::from("/xdg/radarr-cli/config.toml"))
        );
        assert_eq!(
            default_config_path(None, Some(PathBuf::from("/home/u")), "radarr-cli"),
            Some(PathBuf::from("/home/u/.config/radarr-cli/config.toml"))
        );
        assert_eq!(
            default_config_path(Some(PathBuf::from("rel")), Some(PathBuf::from("/h")), "x"),
            Some(PathBuf::from("/h/.config/x/config.toml"))
        );
        assert_eq!(default_config_path(None, None, "radarr-cli"), None);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("radarr-cli-test-{}-{}", name, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_preserves_comments_and_unknown_keys() {
        let dir = temp_dir("save-preserve");
        let path = dir.join("config.toml");
        std::fs::write(&path, "# my notes\nextra_key = \"keepme\"\n").unwrap();

        save(
            Some(path.to_str().unwrap()),
            Some("http://r:7878".into()),
            None,
        )
        .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# my notes"), "{text}");
        assert!(text.contains("extra_key = \"keepme\""), "{text}");

        let parsed: ConfigFile = toml::from_str(&text).unwrap();
        assert_eq!(parsed.url.as_deref(), Some("http://r:7878"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn save_creates_file_with_mode_600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir("save-mode");
        let path = dir.join("config.toml");
        save(Some(path.to_str().unwrap()), None, Some("SECRET".into())).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode was {mode:o}");

        let parsed: ConfigFile = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed.api_key.as_deref(), Some("SECRET"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
