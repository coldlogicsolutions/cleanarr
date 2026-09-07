use anyhow::{Context, Result, bail};
use clap::Subcommand;

use crate::config::{self, PlexConfig};
use crate::output::{self, OutputFormat};
use crate::plex::PlexClient;
use radarr_api::RadarrClient;
use radarr_api::types::MovieResource;

#[derive(Subcommand)]
pub enum PlexCommand {
    /// Authenticate with Plex via browser and save token to config
    Login,
    /// List Plex library sections
    Libraries,
    /// List movies from a Plex library with watch stats
    Movies {
        /// Plex library section key
        #[arg(long)]
        section: Option<String>,
    },
    /// Show Plex watch stats for a Radarr movie
    Stats {
        /// Radarr movie ID
        #[arg(long)]
        movie_id: i64,
        /// Plex library section key
        #[arg(long)]
        section: Option<String>,
    },
}

pub async fn run(
    cmd: PlexCommand,
    radarr: &RadarrClient,
    plex_config: &Option<PlexConfig>,
    fmt: OutputFormat,
) -> Result<()> {
    let plex_config = plex_config.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "Plex is not configured. Run `cleanarr plex login`, or set --plex-url and --plex-token, \
             PLEX_URL/PLEX_TOKEN env vars, or add [plex] section to config file."
        )
    })?;
    let plex = PlexClient::new(plex_config)?;

    match cmd {
        PlexCommand::Login => unreachable!(),
        PlexCommand::Libraries => {
            let libs = plex.get_libraries().await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = libs
                        .iter()
                        .map(|d| vec![d.key.clone(), d.title.clone(), d.section_type.clone()])
                        .collect();
                    output::print_table(&["Key", "Title", "Type"], rows);
                }
                OutputFormat::Json => {
                    let items: Vec<serde_json::Value> = libs
                        .iter()
                        .map(|d| {
                            serde_json::json!({
                                "key": d.key,
                                "title": d.title,
                                "type": d.section_type,
                            })
                        })
                        .collect();
                    output::print_json(&items)?;
                }
                OutputFormat::Quiet => {
                    let keys: Vec<String> = libs.iter().map(|d| d.key.clone()).collect();
                    output::print_quiet(&keys);
                }
            }
            Ok(())
        }
        PlexCommand::Movies { section } => {
            let section_id = plex.resolve_section_id(section.as_deref()).await?;
            let movies = plex.get_movies(&section_id).await?;
            match fmt {
                OutputFormat::Table => {
                    let rows: Vec<Vec<String>> = movies
                        .iter()
                        .map(|v| {
                            vec![
                                v.rating_key.clone(),
                                v.title.clone(),
                                v.year.clone().unwrap_or_default(),
                                v.parsed_view_count().to_string(),
                                format_epoch(v.parsed_last_viewed_at()),
                            ]
                        })
                        .collect();
                    output::print_table(
                        &["Rating Key", "Title", "Year", "View Count", "Last Viewed"],
                        rows,
                    );
                }
                OutputFormat::Json => {
                    let items: Vec<serde_json::Value> = movies
                        .iter()
                        .map(|v| {
                            serde_json::json!({
                                "ratingKey": v.rating_key,
                                "title": v.title,
                                "year": v.year,
                                "viewCount": v.parsed_view_count(),
                                "lastViewedAt": v.parsed_last_viewed_at(),
                                "guids": v.guids.iter().map(|g| &g.id).collect::<Vec<_>>(),
                            })
                        })
                        .collect();
                    output::print_json(&items)?;
                }
                OutputFormat::Quiet => {
                    let keys: Vec<String> = movies.iter().map(|v| v.rating_key.clone()).collect();
                    output::print_quiet(&keys);
                }
            }
            Ok(())
        }
        PlexCommand::Stats { movie_id, section } => {
            // Fetch the Radarr movie to get TMDB/IMDB IDs
            let movie: MovieResource = radarr.get(&format!("/api/v3/movie/{movie_id}")).await?;

            let tmdb_id = movie.tmdb_id;
            let imdb_id = movie.imdb_id.clone();

            if tmdb_id.is_none() && imdb_id.is_none() {
                bail!(
                    "Radarr movie {} ({}) has no TMDB or IMDB ID for Plex matching.",
                    movie_id,
                    movie.title.as_deref().unwrap_or("?")
                );
            }

            let section_id = plex.resolve_section_id(section.as_deref()).await?;
            let plex_movies = plex.get_movies(&section_id).await?;

            let radarr_entries = vec![(movie_id, tmdb_id, imdb_id)];
            let matches = crate::plex::match_movies(&radarr_entries, &plex_movies);

            match matches.get(&movie_id) {
                Some(watch) => match fmt {
                    OutputFormat::Table => {
                        println!(
                            "Movie: {} (Radarr ID: {})",
                            movie.title.as_deref().unwrap_or("?"),
                            movie_id
                        );
                        println!("View Count: {}", watch.view_count);
                        println!("Last Watched: {}", format_epoch(watch.last_viewed_at));
                    }
                    OutputFormat::Json => {
                        let data = serde_json::json!({
                            "radarrId": movie_id,
                            "title": movie.title,
                            "viewCount": watch.view_count,
                            "lastViewedAt": watch.last_viewed_at,
                        });
                        output::print_json(&data)?;
                    }
                    OutputFormat::Quiet => {
                        println!("{}", watch.view_count);
                    }
                },
                None => match fmt {
                    OutputFormat::Table => {
                        println!(
                            "Movie \"{}\" (Radarr ID: {}) not found in Plex.",
                            movie.title.as_deref().unwrap_or("?"),
                            movie_id
                        );
                    }
                    OutputFormat::Json => {
                        let data = serde_json::json!({
                            "radarrId": movie_id,
                            "title": movie.title,
                            "viewCount": null,
                            "lastViewedAt": null,
                            "matched": false,
                        });
                        output::print_json(&data)?;
                    }
                    OutputFormat::Quiet => {}
                },
            }
            Ok(())
        }
    }
}

fn format_epoch(ts: Option<i64>) -> String {
    match ts {
        Some(epoch) => {
            // Format as ISO 8601 date
            let secs = epoch;
            let days = secs / 86400;
            // Simple epoch-to-date: days since 1970-01-01
            let (y, m, d) = days_to_ymd(days);
            format!("{y:04}-{m:02}-{d:02}")
        }
        None => "-".to_string(),
    }
}

fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    // Algorithm from Howard Hinnant's civil_from_days
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub async fn login(plex_url: Option<&str>, cli_config_path: Option<&str>) -> Result<()> {
    let existing = config::load_config_file(cli_config_path)?;

    // Resolve Plex URL: CLI flag → existing config
    let plex_url = plex_url.map(String::from).or(existing.plex.url);

    // Reuse this install's client identifier, or mint one on first login.
    let (client_id, new_client_id) = match existing.plex.client_id {
        Some(id) => (id, None),
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            (id.clone(), Some(id))
        }
    };
    let client_id = client_id.as_str();

    let http = reqwest::Client::new();

    // Step 1: Create a PIN
    eprintln!("Requesting Plex authentication PIN...");
    let pin_resp: serde_json::Value = http
        .post("https://plex.tv/api/v2/pins")
        .header("Accept", "application/json")
        .header("X-Plex-Product", "Cleanarr")
        .header("X-Plex-Client-Identifier", client_id)
        .form(&[("strong", "true")])
        .send()
        .await
        .context("Failed to connect to plex.tv")?
        .json()
        .await
        .context("Failed to parse PIN response")?;

    let pin_id = pin_resp["id"]
        .as_i64()
        .context("No PIN ID in plex.tv response")?;
    let code = pin_resp["code"]
        .as_str()
        .context("No code in plex.tv response")?;

    // Step 2: Open browser for authentication
    let auth_url = format!(
        "https://app.plex.tv/auth#?clientID={client_id}&code={code}&context%5Bdevice%5D%5Bproduct%5D=Cleanarr"
    );

    eprintln!("Opening browser for Plex authentication...");
    eprintln!("If your browser doesn't open, visit:");
    eprintln!("  {auth_url}");

    open_browser(&auth_url);

    // Step 3: Poll for auth token (timeout after 5 minutes)
    eprintln!("Waiting for authentication (up to 5 minutes)...");
    let max_attempts = 150; // 5 minutes at 2-second intervals
    let mut token = None;

    for _ in 0..max_attempts {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let check: serde_json::Value = http
            .get(format!("https://plex.tv/api/v2/pins/{pin_id}"))
            .header("Accept", "application/json")
            .header("X-Plex-Client-Identifier", client_id)
            .send()
            .await?
            .json()
            .await?;

        if let Some(t) = check["authToken"].as_str().filter(|t| !t.is_empty()) {
            token = Some(t.to_string());
            break;
        }
    }

    let token =
        token.ok_or_else(|| anyhow::anyhow!("Authentication timed out. Please try again."))?;

    // Step 4: Save to config
    let updates = config::ConfigUpdates {
        plex_token: Some(token),
        plex_url,
        plex_client_id: new_client_id,
        ..Default::default()
    };

    let path = config::save_to_config(cli_config_path, updates)?;
    eprintln!("Plex authentication successful!");
    eprintln!("Token saved to {}", path.display());

    if config::config_path(cli_config_path)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| toml::from_str::<config::ConfigFile>(&c).ok())
        .map(|c| c.plex.url.is_none())
        .unwrap_or(true)
    {
        eprintln!();
        eprintln!(
            "Note: No Plex server URL configured. Set it with:\n  \
             cleanarr init --plex-url http://your-plex-server:32400"
        );
    }

    Ok(())
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", url])
        .spawn();
}
