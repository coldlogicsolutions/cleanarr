use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::config::{self, CleanupDefaults, PlexConfig, SonarrConfig};
use crate::output::{self, OutputFormat};
use crate::plex::{self, PlexClient, PlexWatchData, SeriesIds};
use crate::sonarr::{self, SonarrClient};
use radarr_api::RadarrClient;

#[derive(Args)]
pub struct CleanupArgs {
    /// Maximum media library size threshold (e.g. "500GB", "1TB"), movies + TV combined.
    /// Falls back to CLEANARR_CLEANUP_MAX_SIZE or [cleanup] default_max_size in the config file.
    pub max_size: Option<String>,

    /// Cleanup strategy: oldest, oldest-added, never-watched, least-watched, oldest-watched.
    /// Falls back to CLEANARR_CLEANUP_STRATEGY or [cleanup] default_strategy, then "oldest".
    #[arg(long)]
    pub strategy: Option<String>,

    /// Which media types may be deleted (disk usage always counts both)
    #[arg(long, value_enum, default_value_t = Media::All)]
    pub media: Media,

    /// Actually delete (dry-run by default)
    #[arg(long)]
    pub confirm: bool,

    /// Comma-separated tag names to exclude from cleanup (Radarr movie tags and Sonarr series
    /// tags). Added to the protected_tags configured in the config file / CLEANARR_PROTECTED_TAGS.
    #[arg(long, value_delimiter = ',')]
    pub exclude_tag: Vec<String>,

    /// Plex movie library section key (auto-detected if only one movie library)
    #[arg(long)]
    pub plex_section: Option<String>,

    /// Plex TV library section key (auto-detected if only one show library)
    #[arg(long)]
    pub plex_tv_section: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Media {
    All,
    Movies,
    Tv,
}

impl Media {
    fn movies(self) -> bool {
        matches!(self, Media::All | Media::Movies)
    }
    fn tv(self) -> bool {
        matches!(self, Media::All | Media::Tv)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiskSpaceResource {
    path: Option<String>,
    free_space: Option<i64>,
    total_space: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovieResource {
    id: Option<i64>,
    title: Option<String>,
    tmdb_id: Option<i64>,
    imdb_id: Option<String>,
    size_on_disk: Option<i64>,
    added: Option<String>,
    #[serde(default)]
    tags: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagResource {
    id: Option<i64>,
    label: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CandidateKind {
    Movie {
        movie_id: i64,
    },
    Episode {
        episode_id: i64,
        episode_file_id: i64,
        series_id: i64,
        series_title: String,
        season: i64,
        episode: i64,
    },
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub kind: CandidateKind,
    pub title: String,
    pub size: i64,
    /// ISO-8601 date string; only the leading `YYYY-MM-DD` is used for ordering/display.
    pub added: String,
    pub view_count: Option<u64>,
    pub last_watched: Option<i64>,
}

impl Candidate {
    fn type_name(&self) -> &'static str {
        match self.kind {
            CandidateKind::Movie { .. } => "movie",
            CandidateKind::Episode { .. } => "episode",
        }
    }

    fn id(&self) -> i64 {
        match self.kind {
            CandidateKind::Movie { movie_id } => movie_id,
            CandidateKind::Episode { episode_id, .. } => episode_id,
        }
    }

    /// Human-readable identity for error messages, e.g. `episode 123 (file 456, series 7 S02E05)`.
    fn describe(&self) -> String {
        match &self.kind {
            CandidateKind::Movie { movie_id } => format!("movie {movie_id}"),
            CandidateKind::Episode {
                episode_id,
                episode_file_id,
                series_id,
                season,
                episode,
                ..
            } => format!(
                "episode {episode_id} (file {episode_file_id}, series {series_id} S{season:02}E{episode:02})"
            ),
        }
    }

    fn added_date(&self) -> String {
        self.added.chars().take(10).collect()
    }

    /// For the "oldest" strategy: last watched time if available, otherwise date added.
    fn effective_date(&self) -> String {
        match self.last_watched {
            Some(epoch) => format_epoch(Some(epoch)),
            None => self.added_date(),
        }
    }

    fn apply_watch(&mut self, watch: Option<&PlexWatchData>) {
        self.view_count = watch.map(|w| w.view_count);
        self.last_watched = watch.and_then(|w| w.last_viewed_at);
    }
}

fn is_plex_strategy(strategy: &str) -> bool {
    matches!(
        strategy,
        "never-watched" | "least-watched" | "oldest-watched"
    )
}

fn parse_size(s: &str) -> Result<i64> {
    let s = s.trim();
    let (num_str, unit) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        (&s[..pos], s[pos..].to_uppercase())
    } else {
        bail!("Invalid size format '{}'. Use e.g. '500GB', '1TB'", s);
    };

    let num: f64 = num_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid number in size '{}'", s))?;

    let multiplier: i64 = match unit.as_str() {
        "B" => 1,
        "KB" => 1_000,
        "MB" => 1_000_000,
        "GB" => 1_000_000_000,
        "TB" => 1_000_000_000_000,
        "PB" => 1_000_000_000_000_000,
        _ => bail!("Unknown size unit '{}'. Use B, KB, MB, GB, TB, PB", unit),
    };

    Ok((num * multiplier as f64) as i64)
}

fn format_size(bytes: i64) -> String {
    const KB: i64 = 1_000;
    const MB: i64 = 1_000_000;
    const GB: i64 = 1_000_000_000;
    const TB: i64 = 1_000_000_000_000;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn disk_json(disks: &[&DiskSpaceResource]) -> Vec<serde_json::Value> {
    disks
        .iter()
        .map(|d| {
            serde_json::json!({
                "path": d.path,
                "totalSpace": d.total_space,
                "freeSpace": d.free_space,
            })
        })
        .collect()
}

fn format_epoch(ts: Option<i64>) -> String {
    match ts {
        Some(epoch) => {
            let days = epoch / 86400;
            let (y, m, d) = days_to_ymd(days);
            format!("{y:04}-{m:02}-{d:02}")
        }
        None => "-".to_string(),
    }
}

fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
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

/// Resolve tag labels (case-insensitive) to ids.
fn resolve_tag_ids(names: &[String], tags: &[(Option<i64>, Option<String>)]) -> Vec<i64> {
    names
        .iter()
        .filter_map(|name| {
            let lower = name.to_lowercase();
            tags.iter().find_map(|(id, label)| match (id, label) {
                (Some(id), Some(label)) if label.to_lowercase() == lower => Some(*id),
                _ => None,
            })
        })
        .collect()
}

/// Sort/filter candidates in place according to the strategy.
/// Returns a message for the table output when the strategy yields nothing.
pub fn apply_strategy(strategy: &str, candidates: &mut Vec<Candidate>) -> Option<&'static str> {
    match strategy {
        "oldest" => candidates.sort_by_key(|c| c.effective_date()),
        "oldest-added" => candidates.sort_by_key(|c| c.added_date()),
        "never-watched" => {
            candidates.retain(|c| c.view_count.unwrap_or(0) == 0);
            if candidates.is_empty() {
                return Some("No unwatched items found for never-watched strategy.");
            }
            candidates.sort_by_key(|c| c.added_date());
        }
        "least-watched" => {
            candidates.sort_by(|a, b| {
                a.view_count
                    .unwrap_or(0)
                    .cmp(&b.view_count.unwrap_or(0))
                    .then_with(|| a.last_watched.cmp(&b.last_watched))
            });
        }
        "oldest-watched" => {
            candidates.retain(|c| c.view_count.unwrap_or(0) > 0);
            if candidates.is_empty() {
                return Some("No watched items found for oldest-watched strategy.");
            }
            candidates.sort_by_key(|a| a.last_watched);
        }
        _ => unreachable!(),
    }
    None
}

fn print_disks(disks: &[&DiskSpaceResource]) {
    for d in disks {
        let total = d.total_space.unwrap_or(0);
        let free = d.free_space.unwrap_or(0);
        println!(
            "Disk {}: {} used / {} total ({} free)",
            d.path.as_deref().unwrap_or("?"),
            format_size(total - free),
            format_size(total),
            format_size(free),
        );
    }
}

fn library_line(movie_size: i64, tv_size: Option<i64>) -> String {
    match tv_size {
        Some(tv) => format!(
            "Library: {} (movies {}, TV {})",
            format_size(movie_size + tv),
            format_size(movie_size),
            format_size(tv)
        ),
        None => format!("Radarr library: {}", format_size(movie_size)),
    }
}

fn candidate_json(c: &Candidate, has_plex: bool) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "type": c.type_name(),
        "id": c.id(),
        "title": c.title,
        "sizeOnDisk": c.size,
        "added": c.added,
    });
    if let CandidateKind::Episode {
        episode_file_id,
        series_id,
        series_title,
        season,
        episode,
        ..
    } = &c.kind
    {
        obj["episodeFileId"] = serde_json::json!(episode_file_id);
        obj["seriesId"] = serde_json::json!(series_id);
        obj["seriesTitle"] = serde_json::json!(series_title);
        obj["seasonNumber"] = serde_json::json!(season);
        obj["episodeNumber"] = serde_json::json!(episode);
    }
    if has_plex {
        obj["viewCount"] = serde_json::json!(c.view_count);
        obj["lastWatched"] = serde_json::json!(c.last_watched);
    }
    obj
}

/// Build episode candidates from Sonarr for every series that has files on disk.
async fn episode_candidates(
    sonarr: &SonarrClient,
    series: &[sonarr::SeriesResource],
    exclude_tag_ids: &[i64],
) -> Result<Vec<Candidate>> {
    let mut out = Vec::new();
    for s in series {
        if s.size_on_disk() <= 0 || s.tags.iter().any(|t| exclude_tag_ids.contains(t)) {
            continue;
        }
        let episodes = sonarr.get_episodes(s.id).await?;
        for ep in episodes {
            let Some(file) = ep
                .episode_file
                .as_ref()
                .filter(|f| ep.has_file && f.size > 0)
            else {
                continue;
            };
            let ep_title = ep.title.clone().unwrap_or_default();
            out.push(Candidate {
                kind: CandidateKind::Episode {
                    episode_id: ep.id,
                    episode_file_id: file.id,
                    series_id: s.id,
                    series_title: s.title.clone(),
                    season: ep.season_number,
                    episode: ep.episode_number,
                },
                title: format!(
                    "{} - S{:02}E{:02} - {}",
                    s.title, ep.season_number, ep.episode_number, ep_title
                ),
                size: file.size,
                added: file.date_added.clone().unwrap_or_default(),
                view_count: None,
                last_watched: None,
            });
        }
    }
    Ok(out)
}

const STRATEGIES: [&str; 5] = [
    "oldest",
    "oldest-added",
    "never-watched",
    "least-watched",
    "oldest-watched",
];

/// Resolve threshold and strategy: CLI > env/config defaults > built-in.
fn resolve_defaults(args: &CleanupArgs, defaults: &CleanupDefaults) -> Result<(String, String)> {
    let max_size = args
        .max_size
        .clone()
        .or_else(|| defaults.max_size.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No size threshold given. Pass it as an argument (e.g. `cleanarr cleanup 4TB`), \
                 set CLEANARR_CLEANUP_MAX_SIZE, or add default_max_size to the [cleanup] config section."
            )
        })?;
    let strategy = args
        .strategy
        .clone()
        .or_else(|| defaults.strategy.clone())
        .unwrap_or_else(|| "oldest".to_string());
    if !STRATEGIES.contains(&strategy.as_str()) {
        bail!(
            "Unknown strategy '{}'. Use: {}",
            strategy,
            STRATEGIES.join(", ")
        );
    }
    Ok((max_size, strategy))
}

/// Delete one candidate. Errors carry the failing step so the caller can report it.
async fn delete_candidate(
    c: &Candidate,
    api: &RadarrClient,
    sonarr: Option<&SonarrClient>,
) -> Result<()> {
    match &c.kind {
        CandidateKind::Movie { movie_id } => api
            .delete_with_query(
                &format!("/api/v3/movie/{movie_id}"),
                &[("deleteFiles", "true")],
            )
            .await
            .context("Radarr movie delete failed"),
        CandidateKind::Episode {
            episode_id,
            episode_file_id,
            ..
        } => {
            let sonarr = sonarr.context("Sonarr is not configured")?;
            sonarr
                .delete_episode_file(*episode_file_id)
                .await
                .context("Sonarr episode file delete failed")?;
            sonarr
                .unmonitor_episodes(&[*episode_id])
                .await
                .context("Sonarr unmonitor failed (file was deleted)")
        }
    }
}

pub async fn run(
    args: CleanupArgs,
    api: &RadarrClient,
    plex_config: &Option<PlexConfig>,
    sonarr_config: &Option<SonarrConfig>,
    protected_tags: &[String],
    defaults: &CleanupDefaults,
    fmt: OutputFormat,
) -> Result<()> {
    let protected = config::merge_tags([protected_tags, args.exclude_tag.as_slice()]);
    let (max_size, strategy) = resolve_defaults(&args, defaults)?;
    let strategy = strategy.as_str();

    if is_plex_strategy(strategy) && plex_config.is_none() {
        bail!(
            "Strategy '{}' requires Plex configuration. Set --plex-url and --plex-token, \
             PLEX_URL/PLEX_TOKEN env vars, or add [plex] section to config file.",
            strategy
        );
    }

    if args.media == Media::Tv && sonarr_config.is_none() {
        bail!(
            "--media tv requires Sonarr configuration. Set --sonarr-url and --sonarr-api-key, \
             SONARR_URL/SONARR_API_KEY env vars, or add [sonarr] section to config file."
        );
    }

    let sonarr = sonarr_config.as_ref().map(SonarrClient::new).transpose()?;
    let include_tv = sonarr.is_some() && args.media.tv();
    let include_movies = args.media.movies();

    // 1. Parse max size threshold
    let max_bytes = parse_size(&max_size)?;

    // 2. Fetch disk space and library contents
    let disks: Vec<DiskSpaceResource> = api.get("/api/v3/diskspace").await?;
    let movies: Vec<MovieResource> = api.get("/api/v3/movie").await?;
    let movie_size: i64 = movies.iter().map(|m| m.size_on_disk.unwrap_or(0)).sum();

    let series: Vec<sonarr::SeriesResource> = match &sonarr {
        Some(s) => s.get_series().await?,
        None => Vec::new(),
    };
    let tv_size: Option<i64> = sonarr
        .as_ref()
        .map(|_| series.iter().map(|s| s.size_on_disk()).sum());
    let library_size = movie_size + tv_size.unwrap_or(0);

    // Deduplicate disk mounts (same total+free = same device)
    let mut seen = HashSet::new();
    let unique_disks: Vec<&DiskSpaceResource> = disks
        .iter()
        .filter(|d| seen.insert((d.total_space.unwrap_or(0), d.free_space.unwrap_or(0))))
        .collect();

    let summary_base = |cleanup_needed: bool| {
        serde_json::json!({
            "disks": disk_json(&unique_disks),
            "movieSize": movie_size,
            "tvSize": tv_size,
            "librarySize": library_size,
            "threshold": max_bytes,
            "cleanup_needed": cleanup_needed,
        })
    };

    // 3. If under threshold, print status and exit
    if library_size <= max_bytes {
        match fmt {
            OutputFormat::Table => {
                print_disks(&unique_disks);
                println!(
                    "{} (threshold: {}). No cleanup needed.",
                    library_line(movie_size, tv_size),
                    format_size(max_bytes),
                );
            }
            OutputFormat::Json => output::print_json(&summary_base(false))?,
            OutputFormat::Quiet => {}
        }
        return Ok(());
    }

    let space_to_free = library_size - max_bytes;
    if fmt == OutputFormat::Table {
        print_disks(&unique_disks);
        println!(
            "{} (threshold: {}). Need to free: {}",
            library_line(movie_size, tv_size),
            format_size(max_bytes),
            format_size(space_to_free),
        );
        println!();
    }

    // 4. Resolve protected tags against Radarr and Sonarr
    if fmt == OutputFormat::Table && !protected.is_empty() {
        println!("Protected tags: {}", protected.join(", "));
        println!();
    }
    let (radarr_exclude, sonarr_exclude) = if protected.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let tags: Vec<TagResource> = api.get("/api/v3/tag").await?;
        let radarr_tags: Vec<_> = tags.into_iter().map(|t| (t.id, t.label)).collect();
        let sonarr_tags: Vec<_> = match &sonarr {
            Some(s) if include_tv => s
                .get_tags()
                .await?
                .into_iter()
                .map(|t| (t.id, t.label))
                .collect(),
            _ => Vec::new(),
        };
        (
            resolve_tag_ids(&protected, &radarr_tags),
            resolve_tag_ids(&protected, &sonarr_tags),
        )
    };

    // 5. Build candidates
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut movie_ids: Vec<(i64, Option<i64>, Option<String>)> = Vec::new();
    if include_movies {
        for m in &movies {
            let Some(id) = m.id else { continue };
            let size = m.size_on_disk.unwrap_or(0);
            if size <= 0 || m.tags.iter().any(|t| radarr_exclude.contains(t)) {
                continue;
            }
            movie_ids.push((id, m.tmdb_id, m.imdb_id.clone()));
            candidates.push(Candidate {
                kind: CandidateKind::Movie { movie_id: id },
                title: m.title.clone().unwrap_or_default(),
                size,
                added: m.added.clone().unwrap_or_default(),
                view_count: None,
                last_watched: None,
            });
        }
    }
    if include_tv {
        let sonarr = sonarr.as_ref().unwrap();
        candidates.extend(episode_candidates(sonarr, &series, &sonarr_exclude).await?);
    }

    if candidates.is_empty() {
        if fmt == OutputFormat::Table {
            println!("No eligible items found for cleanup.");
        }
        return Ok(());
    }

    // 6. Fetch Plex watch data if needed or beneficial
    let fetch_plex = is_plex_strategy(strategy) || (strategy == "oldest" && plex_config.is_some());

    if fetch_plex {
        let plex = PlexClient::new(plex_config.as_ref().unwrap())?;

        let movie_watch: HashMap<i64, PlexWatchData> = if !movie_ids.is_empty() {
            let section_id = plex
                .resolve_section_id(args.plex_section.as_deref())
                .await?;
            let plex_movies = plex.get_movies(&section_id).await?;
            plex::match_movies(&movie_ids, &plex_movies)
        } else {
            HashMap::new()
        };

        let has_episodes = candidates
            .iter()
            .any(|c| matches!(c.kind, CandidateKind::Episode { .. }));
        let episode_watch: HashMap<i64, PlexWatchData> = if has_episodes {
            let section_id = plex
                .resolve_tv_section_id(args.plex_tv_section.as_deref())
                .await?;
            let plex_shows = plex.get_shows(&section_id).await?;
            let plex_episodes = plex.get_episodes(&section_id).await?;
            let series_ids: Vec<(i64, SeriesIds)> = series
                .iter()
                .map(|s| {
                    (
                        s.id,
                        SeriesIds {
                            tvdb_id: s.tvdb_id,
                            tmdb_id: s.tmdb_id,
                            imdb_id: s.imdb_id.clone(),
                        },
                    )
                })
                .collect();
            let episode_refs: Vec<plex::EpisodeRef> = candidates
                .iter()
                .filter_map(|c| match &c.kind {
                    CandidateKind::Episode {
                        episode_id,
                        series_id,
                        season,
                        episode,
                        ..
                    } => Some((*episode_id, *series_id, *season, *episode)),
                    _ => None,
                })
                .collect();
            plex::match_episodes(&series_ids, &episode_refs, &plex_shows, &plex_episodes)
        } else {
            HashMap::new()
        };

        for c in &mut candidates {
            let watch = match c.kind {
                CandidateKind::Movie { movie_id } => movie_watch.get(&movie_id),
                CandidateKind::Episode { episode_id, .. } => episode_watch.get(&episode_id),
            };
            c.apply_watch(watch);
        }
    }
    let has_plex = fetch_plex;

    // 7. Apply strategy
    if let Some(msg) = apply_strategy(strategy, &mut candidates) {
        if fmt == OutputFormat::Table {
            println!("{msg}");
        }
        return Ok(());
    }

    // 8. Accumulate until enough space freed
    let mut to_delete: Vec<&Candidate> = Vec::new();
    let mut accumulated: i64 = 0;
    for candidate in &candidates {
        if accumulated >= space_to_free {
            break;
        }
        to_delete.push(candidate);
        accumulated += candidate.size;
    }

    if to_delete.is_empty() {
        if fmt == OutputFormat::Table {
            println!("Nothing to delete.");
        }
        return Ok(());
    }

    let n_movies = to_delete
        .iter()
        .filter(|c| matches!(c.kind, CandidateKind::Movie { .. }))
        .count();
    let n_episodes = to_delete.len() - n_movies;
    let count_line = |movies: usize, episodes: usize| match (include_tv, movies, episodes) {
        (false, m, _) => format!("{m} movies"),
        (true, m, e) => format!("{m} movies, {e} episodes"),
    };

    // 9. Display summary
    match fmt {
        OutputFormat::Table => {
            let mut headers = vec!["Type", "ID", "Title", "Size", "Added"];
            if has_plex {
                headers.extend(["Views", "Last Watched"]);
            }
            let rows: Vec<Vec<String>> = to_delete
                .iter()
                .map(|c| {
                    let mut row = vec![
                        c.type_name().to_string(),
                        c.id().to_string(),
                        c.title.clone(),
                        format_size(c.size),
                        c.added_date(),
                    ];
                    if has_plex {
                        row.push(
                            c.view_count
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".to_string()),
                        );
                        row.push(format_epoch(c.last_watched));
                    }
                    row
                })
                .collect();
            output::print_table(&headers, rows);
            println!();
            println!(
                "Total: {}, {} to free",
                count_line(n_movies, n_episodes),
                format_size(accumulated)
            );
            println!();
        }
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = to_delete
                .iter()
                .map(|c| candidate_json(c, has_plex))
                .collect();
            let mut summary = summary_base(true);
            summary["toFree"] = serde_json::json!(space_to_free);
            summary["strategy"] = serde_json::json!(strategy);
            summary["media"] = serde_json::json!(format!("{:?}", args.media).to_lowercase());
            summary["protectedTags"] = serde_json::json!(protected);
            // `movies` kept for backwards compatibility; `items` carries every candidate type.
            summary["movies"] = serde_json::json!(
                items
                    .iter()
                    .filter(|i| i["type"] == "movie")
                    .cloned()
                    .collect::<Vec<_>>()
            );
            summary["items"] = serde_json::json!(items);
            summary["totalSize"] = serde_json::json!(accumulated);
            summary["confirm"] = serde_json::json!(args.confirm);
            output::print_json(&summary)?;
        }
        OutputFormat::Quiet => {
            let ids: Vec<String> = to_delete
                .iter()
                .map(|c| {
                    if include_tv {
                        format!("{}:{}", c.type_name(), c.id())
                    } else {
                        c.id().to_string()
                    }
                })
                .collect();
            output::print_quiet(&ids);
        }
    }

    // 10. Delete or dry-run
    if !args.confirm {
        if fmt == OutputFormat::Table {
            println!("Dry run -- nothing deleted. Use --confirm to delete.");
        }
        return Ok(());
    }

    let total = to_delete.len();
    let mut freed: i64 = 0;
    let mut deleted_movies = 0usize;
    let mut deleted_episodes = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (i, c) in to_delete.iter().enumerate() {
        let result = delete_candidate(c, api, sonarr.as_ref()).await;
        match result {
            Ok(()) => {
                freed += c.size;
                match c.kind {
                    CandidateKind::Movie { .. } => deleted_movies += 1,
                    CandidateKind::Episode { .. } => deleted_episodes += 1,
                }
                if fmt == OutputFormat::Table {
                    println!(
                        "[{}/{}] Deleted \"{}\" ({} freed)",
                        i + 1,
                        total,
                        c.title,
                        format_size(freed)
                    );
                }
            }
            Err(e) => {
                let msg = format!("{} \"{}\": {e:#}", c.describe(), c.title);
                if fmt == OutputFormat::Table {
                    println!("[{}/{}] FAILED {msg}", i + 1, total);
                } else {
                    eprintln!("FAILED {msg}");
                }
                failures.push(msg);
            }
        }
    }

    if fmt == OutputFormat::Table {
        println!();
        println!(
            "Cleanup complete. {} deleted, {} freed.",
            count_line(deleted_movies, deleted_episodes),
            format_size(freed)
        );
    }

    if !failures.is_empty() {
        bail!(
            "{} of {} deletions failed:\n  {}",
            failures.len(),
            total,
            failures.join("\n  ")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movie(id: i64, added: &str, views: Option<u64>, last: Option<i64>) -> Candidate {
        Candidate {
            kind: CandidateKind::Movie { movie_id: id },
            title: format!("m{id}"),
            size: 1,
            added: format!("{added}T00:00:00Z"),
            view_count: views,
            last_watched: last,
        }
    }

    fn episode(id: i64, added: &str, views: Option<u64>, last: Option<i64>) -> Candidate {
        Candidate {
            kind: CandidateKind::Episode {
                episode_id: id,
                episode_file_id: id * 10,
                series_id: 1,
                series_title: "S".into(),
                season: 1,
                episode: id,
            },
            title: format!("e{id}"),
            size: 1,
            added: format!("{added}T00:00:00Z"),
            view_count: views,
            last_watched: last,
        }
    }

    fn ids(c: &[Candidate]) -> Vec<i64> {
        c.iter().map(|c| c.id()).collect()
    }

    #[test]
    fn oldest_interleaves_movies_and_episodes() {
        // 2020-01-01 = 1577836800
        let mut c = vec![
            movie(1, "2023-05-01", None, Some(1577836800)),
            episode(2, "2019-01-01", None, None),
            movie(3, "2021-01-01", None, None),
        ];
        apply_strategy("oldest", &mut c);
        assert_eq!(ids(&c), vec![2, 1, 3]);
    }

    #[test]
    fn oldest_added_ignores_watch() {
        let mut c = vec![
            movie(1, "2023-05-01", None, Some(0)),
            episode(2, "2019-01-01", None, None),
        ];
        apply_strategy("oldest-added", &mut c);
        assert_eq!(ids(&c), vec![2, 1]);
    }

    #[test]
    fn never_watched_filters() {
        let mut c = vec![
            movie(1, "2020-01-01", Some(2), Some(5)),
            episode(2, "2021-01-01", Some(0), None),
            episode(3, "2019-01-01", None, None),
        ];
        assert!(apply_strategy("never-watched", &mut c).is_none());
        assert_eq!(ids(&c), vec![3, 2]);
    }

    #[test]
    fn oldest_watched_filters_and_reports_empty() {
        let mut c = vec![episode(1, "2020-01-01", Some(0), None)];
        assert!(apply_strategy("oldest-watched", &mut c).is_some());
        let mut c = vec![
            movie(1, "2020-01-01", Some(1), Some(200)),
            episode(2, "2020-01-01", Some(3), Some(100)),
        ];
        assert!(apply_strategy("oldest-watched", &mut c).is_none());
        assert_eq!(ids(&c), vec![2, 1]);
    }

    #[test]
    fn least_watched_tiebreaks_on_last_watched() {
        let mut c = vec![
            movie(1, "2020-01-01", Some(1), Some(200)),
            episode(2, "2020-01-01", Some(1), Some(100)),
            episode(3, "2020-01-01", Some(0), None),
        ];
        apply_strategy("least-watched", &mut c);
        assert_eq!(ids(&c), vec![3, 2, 1]);
    }

    fn cli(max: Option<&str>, strategy: Option<&str>) -> CleanupArgs {
        CleanupArgs {
            max_size: max.map(String::from),
            strategy: strategy.map(String::from),
            media: Media::All,
            confirm: false,
            exclude_tag: vec![],
            plex_section: None,
            plex_tv_section: None,
        }
    }

    #[test]
    fn defaults_resolution_order() {
        let d = CleanupDefaults {
            max_size: Some("4TB".into()),
            strategy: Some("never-watched".into()),
        };
        assert_eq!(
            resolve_defaults(&cli(None, None), &d).unwrap(),
            ("4TB".to_string(), "never-watched".to_string())
        );
        assert_eq!(
            resolve_defaults(&cli(Some("2TB"), Some("oldest-added")), &d).unwrap(),
            ("2TB".to_string(), "oldest-added".to_string())
        );
        let none = CleanupDefaults::default();
        assert_eq!(
            resolve_defaults(&cli(Some("1TB"), None), &none).unwrap().1,
            "oldest"
        );
        assert!(resolve_defaults(&cli(None, None), &none).is_err());
        assert!(resolve_defaults(&cli(Some("1TB"), Some("bogus")), &none).is_err());
    }

    #[test]
    fn parse_size_units() {
        assert_eq!(parse_size("1TB").unwrap(), 1_000_000_000_000);
        assert_eq!(parse_size("1.5 GB").unwrap(), 1_500_000_000);
        assert!(parse_size("12").is_err());
    }

    #[test]
    fn resolve_tags_case_insensitive() {
        let tags = vec![
            (Some(1), Some("Keep".to_string())),
            (Some(2), Some("x".to_string())),
        ];
        assert_eq!(resolve_tag_ids(&["keep".to_string()], &tags), vec![1]);
        assert!(resolve_tag_ids(&["nope".to_string()], &tags).is_empty());
    }
}
