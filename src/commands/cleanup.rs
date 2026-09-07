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

    /// Maximum media library size threshold, as a flag. Alternative spelling of the positional
    /// MAX_SIZE argument; the two are mutually exclusive.
    #[arg(long = "max-size", value_name = "SIZE", conflicts_with = "max_size")]
    pub max_size_flag: Option<String>,

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

    /// Proceed with --confirm even when a protected tag name matches no tag in a service in scope
    #[arg(long)]
    pub allow_unresolved_tags: bool,

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
        /// Every Sonarr episode backed by `episode_file_id` (multi-episode files have several).
        episode_ids: Vec<i64>,
        episode_file_id: i64,
        series_id: i64,
        series_title: String,
        season: i64,
        /// Episode numbers, parallel to `episode_ids`.
        episodes: Vec<i64>,
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

    /// Primary id: the movie id, or the first episode id sharing the file.
    fn id(&self) -> i64 {
        match &self.kind {
            CandidateKind::Movie { movie_id } => *movie_id,
            CandidateKind::Episode { episode_ids, .. } => episode_ids.first().copied().unwrap_or(0),
        }
    }

    /// Human-readable identity for error messages, e.g. `episode 123 (file 456, series 7 S02E05)`.
    fn describe(&self) -> String {
        match &self.kind {
            CandidateKind::Movie { movie_id } => format!("movie {movie_id}"),
            CandidateKind::Episode {
                episode_ids,
                episode_file_id,
                series_id,
                season,
                episodes,
                ..
            } => {
                let ids = episode_ids
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "episode {ids} (file {episode_file_id}, series {series_id} {})",
                    season_episode_label(*season, episodes)
                )
            }
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

    /// Apply the Plex watch data matched for this candidate. A candidate backed by several Plex
    /// items (a multi-episode file) takes the highest view count and the most recent view time.
    /// No matches at all leaves `view_count` as `None`, i.e. "unknown", not "never watched".
    fn apply_watches(&mut self, watches: &[&PlexWatchData]) {
        self.view_count = watches.iter().map(|w| w.view_count).max();
        self.last_watched = watches.iter().filter_map(|w| w.last_viewed_at).max();
    }
}

/// `S01E01` for a single episode, `S01E01-E02` for a multi-episode file.
fn season_episode_label(season: i64, episodes: &[i64]) -> String {
    let mut out = String::new();
    for (i, ep) in episodes.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!("S{season:02}E{ep:02}"));
        } else {
            out.push_str(&format!("-E{ep:02}"));
        }
    }
    out
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

/// Resolve one tag label to an id, ignoring case and surrounding whitespace on both sides.
fn find_tag_id(name: &str, tags: &[(Option<i64>, Option<String>)]) -> Option<i64> {
    let wanted = name.trim().to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    tags.iter().find_map(|(id, label)| match (id, label) {
        (Some(id), Some(label)) if label.trim().to_lowercase() == wanted => Some(*id),
        _ => None,
    })
}

/// Resolve tag labels (case- and whitespace-insensitive) to ids.
fn resolve_tag_ids(names: &[String], tags: &[(Option<i64>, Option<String>)]) -> Vec<i64> {
    names
        .iter()
        .filter_map(|name| find_tag_id(name, tags))
        .collect()
}

/// Protected tag names that matched no tag in a service that is in scope for this run.
/// Returns `(name, service)` pairs in the order the names were given.
fn unresolved_protected_tags(
    names: &[String],
    radarr_tags: &[(Option<i64>, Option<String>)],
    sonarr_tags: &[(Option<i64>, Option<String>)],
    include_movies: bool,
    include_tv: bool,
) -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    for name in names {
        if name.trim().is_empty() {
            continue;
        }
        if include_movies && find_tag_id(name, radarr_tags).is_none() {
            out.push((name.trim().to_string(), "Radarr"));
        }
        if include_tv && find_tag_id(name, sonarr_tags).is_none() {
            out.push((name.trim().to_string(), "Sonarr"));
        }
    }
    out
}

/// Outcome of applying a strategy to the candidate list.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct StrategyOutcome {
    /// Message for the table output when the strategy yields nothing.
    pub message: Option<&'static str>,
    /// Candidates dropped because Plex had no watch data for them (view count unknown).
    pub unmatched: usize,
}

/// Sort/filter candidates in place according to the strategy.
///
/// `view_count == None` means "not found in Plex", which is not the same as "never watched":
/// the watch-based strategies drop those candidates and report how many were dropped.
pub fn apply_strategy(strategy: &str, candidates: &mut Vec<Candidate>) -> StrategyOutcome {
    /// Drop candidates Plex knew nothing about, returning how many were dropped.
    fn drop_unmatched(candidates: &mut Vec<Candidate>) -> usize {
        let before = candidates.len();
        candidates.retain(|c| c.view_count.is_some());
        before - candidates.len()
    }

    let mut unmatched = 0usize;
    let message = match strategy {
        "oldest" => {
            candidates.sort_by_key(|c| c.effective_date());
            None
        }
        "oldest-added" => {
            candidates.sort_by_key(|c| c.added_date());
            None
        }
        "never-watched" => {
            unmatched = drop_unmatched(candidates);
            candidates.retain(|c| c.view_count == Some(0));
            if candidates.is_empty() {
                Some("No unwatched items found for never-watched strategy.")
            } else {
                candidates.sort_by_key(|c| c.added_date());
                None
            }
        }
        "least-watched" => {
            unmatched = drop_unmatched(candidates);
            if candidates.is_empty() {
                Some("No items with Plex watch data found for least-watched strategy.")
            } else {
                candidates.sort_by(|a, b| {
                    a.view_count
                        .cmp(&b.view_count)
                        .then_with(|| a.last_watched.cmp(&b.last_watched))
                });
                None
            }
        }
        "oldest-watched" => {
            unmatched = drop_unmatched(candidates);
            candidates.retain(|c| c.view_count.is_some_and(|v| v > 0));
            if candidates.is_empty() {
                Some("No watched items found for oldest-watched strategy.")
            } else {
                candidates.sort_by_key(|a| a.last_watched);
                None
            }
        }
        _ => unreachable!(),
    };
    StrategyOutcome { message, unmatched }
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
        episode_ids,
        episode_file_id,
        series_id,
        series_title,
        season,
        episodes,
    } = &c.kind
    {
        obj["episodeFileId"] = serde_json::json!(episode_file_id);
        obj["seriesId"] = serde_json::json!(series_id);
        obj["seriesTitle"] = serde_json::json!(series_title);
        obj["seasonNumber"] = serde_json::json!(season);
        // `episodeNumber` / `id` stay scalar (first episode) for backwards compatibility;
        // the arrays carry every episode backed by this file.
        obj["episodeNumber"] = serde_json::json!(episodes.first());
        obj["episodeNumbers"] = serde_json::json!(episodes);
        obj["episodeIds"] = serde_json::json!(episode_ids);
    }
    if has_plex {
        obj["viewCount"] = serde_json::json!(c.view_count);
        obj["lastWatched"] = serde_json::json!(c.last_watched);
    }
    obj
}

/// `(radarrId, tmdbId, imdbId)` triples handed to the Plex matcher.
type MovieMatchIds = Vec<(i64, Option<i64>, Option<String>)>;

/// Movie candidates plus the `(id, tmdbId, imdbId)` triples used for Plex matching.
/// Movies with no id, nothing on disk, or a protected tag are skipped.
fn movie_candidates(movies: &[MovieResource], exclude: &[i64]) -> (Vec<Candidate>, MovieMatchIds) {
    let mut candidates = Vec::new();
    let mut ids = Vec::new();
    for m in movies {
        let Some(id) = m.id else { continue };
        let size = m.size_on_disk.unwrap_or(0);
        if size <= 0 || m.tags.iter().any(|t| exclude.contains(t)) {
            continue;
        }
        ids.push((id, m.tmdb_id, m.imdb_id.clone()));
        candidates.push(Candidate {
            kind: CandidateKind::Movie { movie_id: id },
            title: m.title.clone().unwrap_or_default(),
            size,
            added: m.added.clone().unwrap_or_default(),
            view_count: None,
            last_watched: None,
        });
    }
    (candidates, ids)
}

/// A series is eligible for cleanup when it has files on disk and carries no protected tag.
fn series_eligible(s: &sonarr::SeriesResource, exclude: &[i64]) -> bool {
    s.size_on_disk() > 0 && !s.tags.iter().any(|t| exclude.contains(t))
}

/// Build one candidate per episode *file*: episodes sharing a multi-episode file collapse into a
/// single candidate so the size is counted once, the file is deleted once, and every episode
/// backed by it gets unmonitored.
fn episode_candidates_for_series(
    s: &sonarr::SeriesResource,
    episodes: Vec<sonarr::EpisodeResource>,
) -> Vec<Candidate> {
    struct Group {
        episode_ids: Vec<i64>,
        numbers: Vec<i64>,
        titles: Vec<String>,
        season: i64,
        size: i64,
        added: String,
    }

    let mut order: Vec<i64> = Vec::new();
    let mut groups: HashMap<i64, Group> = HashMap::new();

    for ep in episodes {
        let Some(file) = ep
            .episode_file
            .as_ref()
            .filter(|f| ep.has_file && f.size > 0)
        else {
            continue;
        };
        let entry = groups.entry(file.id).or_insert_with(|| {
            order.push(file.id);
            Group {
                episode_ids: Vec::new(),
                numbers: Vec::new(),
                titles: Vec::new(),
                season: ep.season_number,
                size: file.size,
                added: file.date_added.clone().unwrap_or_default(),
            }
        });
        entry.episode_ids.push(ep.id);
        entry.numbers.push(ep.episode_number);
        entry.titles.push(ep.title.clone().unwrap_or_default());
    }

    order
        .into_iter()
        .map(|file_id| {
            let g = groups.remove(&file_id).expect("group recorded in order");
            Candidate {
                title: format!(
                    "{} - {} - {}",
                    s.title,
                    season_episode_label(g.season, &g.numbers),
                    g.titles.join(" / ")
                ),
                kind: CandidateKind::Episode {
                    episode_ids: g.episode_ids,
                    episode_file_id: file_id,
                    series_id: s.id,
                    series_title: s.title.clone(),
                    season: g.season,
                    episodes: g.numbers,
                },
                size: g.size,
                added: g.added,
                view_count: None,
                last_watched: None,
            }
        })
        .collect()
}

/// Take candidates in order until the accumulated size covers `space_to_free`.
fn select_to_delete(candidates: &[Candidate], space_to_free: i64) -> (Vec<&Candidate>, i64) {
    let mut to_delete = Vec::new();
    let mut accumulated: i64 = 0;
    for candidate in candidates {
        if accumulated >= space_to_free {
            break;
        }
        to_delete.push(candidate);
        accumulated += candidate.size;
    }
    (to_delete, accumulated)
}

/// Build episode candidates from Sonarr for every eligible series.
async fn episode_candidates(
    sonarr: &SonarrClient,
    series: &[sonarr::SeriesResource],
    exclude_tag_ids: &[i64],
) -> Result<Vec<Candidate>> {
    let mut out = Vec::new();
    for s in series {
        if !series_eligible(s, exclude_tag_ids) {
            continue;
        }
        let episodes = sonarr.get_episodes(s.id).await?;
        out.extend(episode_candidates_for_series(s, episodes));
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
        .or_else(|| args.max_size_flag.clone())
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
            episode_ids,
            episode_file_id,
            ..
        } => {
            let sonarr = sonarr.context("Sonarr is not configured")?;
            sonarr
                .delete_episode_file(*episode_file_id)
                .await
                .context("Sonarr episode file delete failed")?;
            // Unmonitor every episode backed by the deleted file, or Sonarr re-grabs it.
            sonarr
                .unmonitor_episodes(episode_ids)
                .await
                .context("Sonarr unmonitor failed (file was deleted)")
        }
    }
}

/// Resolve the Plex movie section and match it against the Radarr movie ids.
async fn fetch_movie_watch(
    plex: &PlexClient,
    section: Option<&str>,
    movie_ids: &[(i64, Option<i64>, Option<String>)],
) -> Result<HashMap<i64, PlexWatchData>> {
    let section_id = plex.resolve_section_id(section).await?;
    let plex_movies = plex.get_movies(&section_id).await?;
    Ok(plex::match_movies(movie_ids, &plex_movies))
}

/// Resolve the Plex TV section and match it against the Sonarr series/episodes.
async fn fetch_episode_watch(
    plex: &PlexClient,
    section: Option<&str>,
    series: &[sonarr::SeriesResource],
    episode_refs: &[plex::EpisodeRef],
) -> Result<HashMap<i64, PlexWatchData>> {
    let section_id = plex.resolve_tv_section_id(section).await?;
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
    Ok(plex::match_episodes(
        &series_ids,
        episode_refs,
        &plex_shows,
        &plex_episodes,
    ))
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
    let mut unresolved_tags: Vec<(String, &'static str)> = Vec::new();
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
        unresolved_tags = unresolved_protected_tags(
            &protected,
            &radarr_tags,
            &sonarr_tags,
            include_movies,
            include_tv,
        );
        (
            resolve_tag_ids(&protected, &radarr_tags),
            resolve_tag_ids(&protected, &sonarr_tags),
        )
    };

    for (name, service) in &unresolved_tags {
        eprintln!(
            "Warning: protected tag '{name}' matches no {service} tag; nothing is protected by it there."
        );
    }
    if args.confirm && !unresolved_tags.is_empty() && !args.allow_unresolved_tags {
        let list = unresolved_tags
            .iter()
            .map(|(name, service)| format!("'{name}' ({service})"))
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "Refusing to delete: protected tag(s) {list} match no tag in that service, so nothing \
             is protected by them. Fix the tag name(s) or pass --allow-unresolved-tags."
        );
    }

    // 5. Build candidates
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut movie_ids: Vec<(i64, Option<i64>, Option<String>)> = Vec::new();
    if include_movies {
        let (movie_cands, ids) = movie_candidates(&movies, &radarr_exclude);
        candidates = movie_cands;
        movie_ids = ids;
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

    // `oldest` only uses Plex as an enhancement, so a Plex failure degrades to "no watch data"
    // for that media type; the Plex-dependent strategies keep the hard error.
    let plex_optional = strategy == "oldest";
    /// Either propagate the error or warn and fall back to no watch data.
    macro_rules! soft_plex {
        ($expr:expr, $what:literal) => {
            match $expr {
                Ok(v) => v,
                Err(e) if plex_optional => {
                    eprintln!(
                        "Warning: could not read Plex {} watch data ({e:#}); continuing without it.",
                        $what
                    );
                    Default::default()
                }
                Err(e) => return Err(e),
            }
        };
    }

    if fetch_plex {
        let plex = match PlexClient::new(plex_config.as_ref().unwrap()) {
            Ok(p) => Some(p),
            Err(e) if plex_optional => {
                eprintln!(
                    "Warning: could not connect to Plex ({e:#}); continuing without watch data."
                );
                None
            }
            Err(e) => return Err(e),
        };

        if let Some(plex) = plex {
            let movie_watch: HashMap<i64, PlexWatchData> = if movie_ids.is_empty() {
                HashMap::new()
            } else {
                soft_plex!(
                    fetch_movie_watch(&plex, args.plex_section.as_deref(), &movie_ids).await,
                    "movie"
                )
            };

            let episode_refs: Vec<plex::EpisodeRef> = candidates
                .iter()
                .flat_map(|c| match &c.kind {
                    CandidateKind::Episode {
                        episode_ids,
                        series_id,
                        season,
                        episodes,
                        ..
                    } => episode_ids
                        .iter()
                        .zip(episodes.iter())
                        .map(|(id, ep)| (*id, *series_id, *season, *ep))
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                })
                .collect();
            let episode_watch: HashMap<i64, PlexWatchData> = if episode_refs.is_empty() {
                HashMap::new()
            } else {
                soft_plex!(
                    fetch_episode_watch(
                        &plex,
                        args.plex_tv_section.as_deref(),
                        &series,
                        &episode_refs
                    )
                    .await,
                    "TV"
                )
            };

            for c in &mut candidates {
                let watches: Vec<&PlexWatchData> = match &c.kind {
                    CandidateKind::Movie { movie_id } => {
                        movie_watch.get(movie_id).into_iter().collect()
                    }
                    CandidateKind::Episode { episode_ids, .. } => episode_ids
                        .iter()
                        .filter_map(|id| episode_watch.get(id))
                        .collect(),
                };
                c.apply_watches(&watches);
            }
        }
    }
    let has_plex = fetch_plex;

    // 7. Apply strategy
    let outcome = apply_strategy(strategy, &mut candidates);
    if outcome.unmatched > 0 && fmt == OutputFormat::Table {
        println!(
            "{} candidates not found in Plex — skipped for this strategy",
            outcome.unmatched
        );
        println!();
    }
    if let Some(msg) = outcome.message {
        if fmt == OutputFormat::Table {
            println!("{msg}");
        }
        return Ok(());
    }

    // 8. Accumulate until enough space freed
    let (to_delete, accumulated) = select_to_delete(&candidates, space_to_free);

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

    let json_summary = |items: Vec<serde_json::Value>, total_size: i64| -> serde_json::Value {
        let mut summary = summary_base(true);
        summary["toFree"] = serde_json::json!(space_to_free);
        summary["strategy"] = serde_json::json!(strategy);
        summary["media"] = serde_json::json!(format!("{:?}", args.media).to_lowercase());
        summary["protectedTags"] = serde_json::json!(protected);
        summary["unmatched"] = serde_json::json!(outcome.unmatched);
        // `movies` kept for backwards compatibility; `items` carries every candidate type.
        summary["movies"] = serde_json::json!(
            items
                .iter()
                .filter(|i| i["type"] == "movie")
                .cloned()
                .collect::<Vec<_>>()
        );
        summary["items"] = serde_json::json!(items);
        summary["totalSize"] = serde_json::json!(total_size);
        summary["confirm"] = serde_json::json!(args.confirm);
        summary
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
            // Under --confirm the JSON document is emitted after the deletion loop so it can
            // report the outcome; without it the plan is printed here, marked "planned".
            if !args.confirm {
                let items: Vec<serde_json::Value> = to_delete
                    .iter()
                    .map(|c| {
                        let mut item = candidate_json(c, has_plex);
                        item["status"] = serde_json::json!("planned");
                        item
                    })
                    .collect();
                output::print_json(&json_summary(items, accumulated))?;
            }
        }
        OutputFormat::Quiet => {
            // Under --confirm only ids that were actually deleted are printed, after the loop.
            if !args.confirm {
                let ids: Vec<String> = to_delete
                    .iter()
                    .map(|c| format!("{}:{}", c.type_name(), c.id()))
                    .collect();
                output::print_quiet(&ids);
            }
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
    let mut results: Vec<Option<String>> = Vec::with_capacity(total);
    for (i, c) in to_delete.iter().enumerate() {
        let result = delete_candidate(c, api, sonarr.as_ref()).await;
        match result {
            Ok(()) => {
                freed += c.size;
                match c.kind {
                    CandidateKind::Movie { .. } => deleted_movies += 1,
                    CandidateKind::Episode { .. } => deleted_episodes += 1,
                }
                results.push(None);
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
                results.push(Some(format!("{e:#}")));
                if fmt == OutputFormat::Table {
                    println!("[{}/{}] FAILED {msg}", i + 1, total);
                } else {
                    eprintln!("FAILED {msg}");
                }
                failures.push(msg);
            }
        }
    }

    match fmt {
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = to_delete
                .iter()
                .zip(results.iter())
                .map(|(c, error)| {
                    let mut item = candidate_json(c, has_plex);
                    item["status"] = serde_json::json!(match error {
                        None => "deleted",
                        Some(_) => "failed",
                    });
                    item["error"] = serde_json::json!(error);
                    item
                })
                .collect();
            let mut summary = json_summary(items, freed);
            summary["deleted"] = serde_json::json!(total - failures.len());
            summary["failed"] = serde_json::json!(failures.len());
            summary["freed"] = serde_json::json!(freed);
            output::print_json(&summary)?;
        }
        OutputFormat::Quiet => {
            let ids: Vec<String> = to_delete
                .iter()
                .zip(results.iter())
                .filter(|(_, error)| error.is_none())
                .map(|(c, _)| format!("{}:{}", c.type_name(), c.id()))
                .collect();
            output::print_quiet(&ids);
        }
        OutputFormat::Table => {}
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
    use crate::sonarr::{EpisodeFileResource, EpisodeResource, SeriesResource, SeriesStatistics};

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
                episode_ids: vec![id],
                episode_file_id: id * 10,
                series_id: 1,
                series_title: "S".into(),
                season: 1,
                episodes: vec![id],
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

    fn movie_resource(id: Option<i64>, size: Option<i64>, tags: Vec<i64>) -> MovieResource {
        MovieResource {
            id,
            title: id.map(|i| format!("m{i}")),
            tmdb_id: id,
            imdb_id: None,
            size_on_disk: size,
            added: Some("2020-01-01T00:00:00Z".into()),
            tags,
        }
    }

    fn series_resource(id: i64, size: i64, tags: Vec<i64>) -> SeriesResource {
        SeriesResource {
            id,
            title: format!("Series {id}"),
            tvdb_id: Some(id),
            tmdb_id: None,
            imdb_id: None,
            tags,
            statistics: Some(SeriesStatistics { size_on_disk: size }),
        }
    }

    fn episode_resource(
        id: i64,
        season: i64,
        number: i64,
        title: &str,
        file: Option<(i64, i64)>,
    ) -> EpisodeResource {
        EpisodeResource {
            id,
            season_number: season,
            episode_number: number,
            title: Some(title.to_string()),
            has_file: file.is_some(),
            episode_file: file.map(|(file_id, size)| EpisodeFileResource {
                id: file_id,
                size,
                date_added: Some("2021-02-03T00:00:00Z".into()),
            }),
        }
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
    fn never_watched_excludes_plex_unmatched() {
        let mut c = vec![
            movie(1, "2020-01-01", Some(2), Some(5)),
            episode(2, "2021-01-01", Some(0), None),
            // Not found in Plex: view count unknown, not zero.
            episode(3, "2019-01-01", None, None),
        ];
        let outcome = apply_strategy("never-watched", &mut c);
        assert!(outcome.message.is_none());
        assert_eq!(outcome.unmatched, 1);
        assert_eq!(ids(&c), vec![2]);
    }

    #[test]
    fn never_watched_reports_empty_when_all_unmatched() {
        let mut c = vec![movie(1, "2020-01-01", None, None)];
        let outcome = apply_strategy("never-watched", &mut c);
        assert!(outcome.message.is_some());
        assert_eq!(outcome.unmatched, 1);
        assert!(c.is_empty());
    }

    #[test]
    fn oldest_watched_filters_and_reports_empty() {
        let mut c = vec![episode(1, "2020-01-01", Some(0), None)];
        assert!(apply_strategy("oldest-watched", &mut c).message.is_some());
        let mut c = vec![
            movie(1, "2020-01-01", Some(1), Some(200)),
            episode(2, "2020-01-01", Some(3), Some(100)),
            episode(4, "2020-01-01", None, None),
        ];
        let outcome = apply_strategy("oldest-watched", &mut c);
        assert!(outcome.message.is_none());
        assert_eq!(outcome.unmatched, 1);
        assert_eq!(ids(&c), vec![2, 1]);
    }

    #[test]
    fn least_watched_tiebreaks_on_last_watched_and_skips_unmatched() {
        let mut c = vec![
            movie(1, "2020-01-01", Some(1), Some(200)),
            episode(2, "2020-01-01", Some(1), Some(100)),
            episode(3, "2020-01-01", Some(0), None),
            episode(5, "2020-01-01", None, None),
        ];
        let outcome = apply_strategy("least-watched", &mut c);
        assert_eq!(outcome.unmatched, 1);
        assert_eq!(ids(&c), vec![3, 2, 1]);
    }

    fn cli(max: Option<&str>, strategy: Option<&str>) -> CleanupArgs {
        CleanupArgs {
            max_size: max.map(String::from),
            max_size_flag: None,
            strategy: strategy.map(String::from),
            media: Media::All,
            confirm: false,
            exclude_tag: vec![],
            allow_unresolved_tags: false,
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
    fn max_size_flag_is_accepted_like_the_positional() {
        let none = CleanupDefaults::default();
        let mut args = cli(None, None);
        args.max_size_flag = Some("2TB".into());
        assert_eq!(resolve_defaults(&args, &none).unwrap().0, "2TB");
    }

    #[test]
    fn max_size_flag_parses_and_conflicts_with_positional() {
        #[derive(clap::Parser)]
        struct Wrapper {
            #[command(flatten)]
            args: CleanupArgs,
        }
        use clap::Parser;

        let args = Wrapper::try_parse_from(["cleanup", "--max-size", "2TB"])
            .expect("--max-size must parse")
            .args;
        assert_eq!(args.max_size_flag.as_deref(), Some("2TB"));
        assert!(
            Wrapper::try_parse_from(["cleanup", "1TB", "--max-size", "2TB"]).is_err(),
            "positional MAX_SIZE and --max-size must conflict"
        );
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

    #[test]
    fn resolve_tags_trims_whitespace() {
        let tags = vec![(Some(2), Some("kids".to_string()))];
        assert_eq!(resolve_tag_ids(&[" Kids ".to_string()], &tags), vec![2]);
    }

    #[test]
    fn unresolved_protected_tags_names_tag_and_service() {
        let radarr = vec![(Some(1), Some("keep".to_string()))];
        let sonarr = vec![(Some(9), Some("kids".to_string()))];
        let names = vec!["keep".to_string(), " kids ".to_string(), "".to_string()];

        assert_eq!(
            unresolved_protected_tags(&names, &radarr, &sonarr, true, true),
            vec![
                ("keep".to_string(), "Sonarr"),
                ("kids".to_string(), "Radarr"),
            ]
        );
        // TV out of scope: only Radarr is checked.
        assert_eq!(
            unresolved_protected_tags(&names, &radarr, &sonarr, true, false),
            vec![("kids".to_string(), "Radarr")]
        );
        // Everything resolves in the services in scope.
        assert!(
            unresolved_protected_tags(&["keep".to_string()], &radarr, &sonarr, true, false)
                .is_empty()
        );
    }

    #[test]
    fn movie_candidates_skip_protected_and_empty_movies() {
        let movies = vec![
            movie_resource(Some(1), Some(100), vec![]),
            movie_resource(Some(2), Some(100), vec![7]), // protected
            movie_resource(Some(3), Some(0), vec![]),    // nothing on disk
            movie_resource(None, Some(100), vec![]),     // no id
        ];
        let (candidates, ids) = movie_candidates(&movies, &[7]);
        assert_eq!(
            candidates.iter().map(|c| c.id()).collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(ids.iter().map(|(id, ..)| *id).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn tagged_series_yields_no_candidates_while_sibling_does() {
        let protected = series_resource(1, 1_000, vec![7]);
        let sibling = series_resource(2, 1_000, vec![3]);
        assert!(!series_eligible(&protected, &[7]));
        assert!(series_eligible(&sibling, &[7]));

        let episodes = || vec![episode_resource(10, 1, 1, "Pilot", Some((100, 500)))];
        let from_protected: Vec<Candidate> = if series_eligible(&protected, &[7]) {
            episode_candidates_for_series(&protected, episodes())
        } else {
            Vec::new()
        };
        assert!(from_protected.is_empty());
        assert_eq!(episode_candidates_for_series(&sibling, episodes()).len(), 1);
    }

    #[test]
    fn series_without_files_is_not_eligible() {
        assert!(!series_eligible(&series_resource(1, 0, vec![]), &[]));
    }

    #[test]
    fn episodes_without_usable_files_are_skipped() {
        let s = series_resource(1, 1_000, vec![]);
        let episodes = vec![
            episode_resource(10, 1, 1, "No file", None),
            EpisodeResource {
                has_file: false,
                ..episode_resource(11, 1, 2, "Not on disk", Some((101, 500)))
            },
            episode_resource(12, 1, 3, "Zero size", Some((102, 0))),
            episode_resource(13, 1, 4, "Good", Some((103, 500))),
        ];
        let candidates = episode_candidates_for_series(&s, episodes);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id(), 13);
    }

    #[test]
    fn multi_episode_file_yields_one_candidate() {
        let s = series_resource(4, 8_000, vec![]);
        let episodes = vec![
            episode_resource(10, 1, 1, "Part One", Some((77, 4_000))),
            episode_resource(11, 1, 2, "Part Two", Some((77, 4_000))),
            episode_resource(12, 1, 3, "Solo", Some((78, 1_000))),
        ];
        let candidates = episode_candidates_for_series(&s, episodes);
        assert_eq!(candidates.len(), 2);

        let multi = &candidates[0];
        // Size counted once, not once per episode.
        assert_eq!(multi.size, 4_000);
        assert_eq!(multi.title, "Series 4 - S01E01-E02 - Part One / Part Two");
        assert_eq!(multi.id(), 10);
        match &multi.kind {
            CandidateKind::Episode {
                episode_ids,
                episode_file_id,
                episodes,
                ..
            } => {
                assert_eq!(episode_ids, &vec![10, 11]);
                assert_eq!(*episode_file_id, 77);
                assert_eq!(episodes, &vec![1, 2]);
            }
            _ => panic!("expected an episode candidate"),
        }

        assert_eq!(candidates[1].title, "Series 4 - S01E03 - Solo");
        let json = candidate_json(multi, false);
        assert_eq!(json["id"], 10);
        assert_eq!(json["episodeIds"], serde_json::json!([10, 11]));
        assert_eq!(json["episodeNumber"], 1);
        assert_eq!(json["episodeNumbers"], serde_json::json!([1, 2]));
    }

    #[test]
    fn select_to_delete_stops_once_space_is_covered() {
        let mut c = vec![
            movie(1, "2020-01-01", None, None),
            movie(2, "2020-01-01", None, None),
            movie(3, "2020-01-01", None, None),
        ];
        for (i, cand) in c.iter_mut().enumerate() {
            cand.size = 100 * (i as i64 + 1);
        }
        let (selected, accumulated) = select_to_delete(&c, 250);
        assert_eq!(
            selected.iter().map(|c| c.id()).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(accumulated, 300);

        let (selected, accumulated) = select_to_delete(&c, 100);
        assert_eq!(selected.len(), 1);
        assert_eq!(accumulated, 100);

        let (selected, accumulated) = select_to_delete(&c, 0);
        assert!(selected.is_empty());
        assert_eq!(accumulated, 0);

        // More space needed than the library has: take everything, don't loop forever.
        let (selected, accumulated) = select_to_delete(&c, 10_000);
        assert_eq!(selected.len(), 3);
        assert_eq!(accumulated, 600);
    }

    #[test]
    fn season_episode_labels() {
        assert_eq!(season_episode_label(1, &[1]), "S01E01");
        assert_eq!(season_episode_label(1, &[1, 2]), "S01E01-E02");
    }
}
