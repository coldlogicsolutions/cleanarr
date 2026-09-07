use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;

use crate::config::PlexConfig;

#[derive(Clone)]
pub struct PlexClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

// XML deserialization structs

#[derive(Debug, Deserialize)]
pub struct SectionsContainer {
    #[serde(rename = "Directory", default)]
    pub directories: Vec<Directory>,
}

#[derive(Debug, Deserialize)]
pub struct VideosContainer {
    #[serde(rename = "Video", default)]
    pub videos: Vec<Video>,
}

#[derive(Debug, Deserialize)]
pub struct Directory {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@title")]
    pub title: String,
    #[serde(rename = "@type")]
    pub section_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Video {
    #[serde(rename = "@ratingKey")]
    pub rating_key: String,
    #[serde(rename = "@title")]
    pub title: String,
    #[serde(rename = "@year", default)]
    pub year: Option<String>,
    #[serde(rename = "@viewCount", default)]
    pub view_count: Option<String>,
    #[serde(rename = "@lastViewedAt", default)]
    pub last_viewed_at: Option<String>,
    #[serde(rename = "Guid", default)]
    pub guids: Vec<GuidElement>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodesContainer {
    #[serde(rename = "Video", default)]
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
pub struct ShowsContainer {
    #[serde(rename = "Directory", default)]
    pub shows: Vec<Show>,
}

#[derive(Debug, Deserialize)]
pub struct Show {
    #[serde(rename = "@ratingKey")]
    pub rating_key: String,
    #[serde(rename = "Guid", default)]
    pub guids: Vec<GuidElement>,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    #[serde(rename = "@grandparentRatingKey", default)]
    pub show_rating_key: Option<String>,
    #[serde(rename = "@parentIndex", default)]
    pub season_number: Option<String>,
    #[serde(rename = "@index", default)]
    pub episode_number: Option<String>,
    #[serde(rename = "@viewCount", default)]
    pub view_count: Option<String>,
    #[serde(rename = "@lastViewedAt", default)]
    pub last_viewed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GuidElement {
    #[serde(rename = "@id")]
    pub id: String,
}

// Parsed external IDs

#[derive(Debug, Default)]
pub struct PlexExternalIds {
    pub tvdb_id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
}

fn parse_external_ids(guids: &[GuidElement]) -> PlexExternalIds {
    let mut ids = PlexExternalIds::default();
    for guid in guids {
        if let Some(rest) = guid.id.strip_prefix("tmdb://") {
            if let Ok(id) = rest.parse::<i64>() {
                ids.tmdb_id = Some(id);
            }
        } else if let Some(rest) = guid.id.strip_prefix("tvdb://") {
            if let Ok(id) = rest.parse::<i64>() {
                ids.tvdb_id = Some(id);
            }
        } else if let Some(rest) = guid.id.strip_prefix("imdb://") {
            ids.imdb_id = Some(rest.to_string());
        }
    }
    ids
}

// Watch data for matching

#[derive(Debug, Clone)]
pub struct PlexWatchData {
    pub view_count: u64,
    pub last_viewed_at: Option<i64>,
}

impl PlexClient {
    pub fn new(config: &PlexConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .build()
            .context("Failed to create Plex HTTP client")?;

        let base_url = config.url.trim_end_matches('/').to_string();

        Ok(Self {
            http,
            base_url,
            token: config.token.clone(),
        })
    }

    async fn get_xml(&self, path: &str, extra_params: &[(&str, &str)]) -> Result<String> {
        let url = format!("{}{}", self.base_url, path);
        let mut query: Vec<(&str, &str)> = vec![("X-Plex-Token", &self.token)];
        query.extend_from_slice(extra_params);
        let resp = self
            .http
            .get(&url)
            .query(&query)
            .header("Accept", "application/xml")
            .send()
            .await
            .with_context(|| format!("Failed to connect to Plex at {url}"))?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            bail!("Plex authentication failed. Check your Plex token.");
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("Plex API error ({}): {}", status, body);
        }

        resp.text()
            .await
            .with_context(|| format!("Failed to read Plex response from {url}"))
    }

    pub async fn get_libraries(&self) -> Result<Vec<Directory>> {
        let xml = self.get_xml("/library/sections", &[]).await?;
        let container: SectionsContainer =
            quick_xml::de::from_str(&xml).context("Failed to parse Plex library sections XML")?;
        Ok(container.directories)
    }

    pub async fn get_movies(&self, section_id: &str) -> Result<Vec<Video>> {
        let xml = self
            .get_xml(
                &format!("/library/sections/{section_id}/all"),
                &[("includeGuids", "1")],
            )
            .await?;
        let container: VideosContainer =
            quick_xml::de::from_str(&xml).context("Failed to parse Plex movies XML")?;
        Ok(container.videos)
    }

    pub async fn get_shows(&self, section_id: &str) -> Result<Vec<Show>> {
        let xml = self
            .get_xml(
                &format!("/library/sections/{section_id}/all"),
                &[("type", "2"), ("includeGuids", "1")],
            )
            .await?;
        let container: ShowsContainer =
            quick_xml::de::from_str(&xml).context("Failed to parse Plex shows XML")?;
        Ok(container.shows)
    }

    pub async fn get_episodes(&self, section_id: &str) -> Result<Vec<Episode>> {
        let xml = self
            .get_xml(
                &format!("/library/sections/{section_id}/all"),
                &[("type", "4")],
            )
            .await?;
        let container: EpisodesContainer =
            quick_xml::de::from_str(&xml).context("Failed to parse Plex episodes XML")?;
        Ok(container.episodes)
    }

    pub async fn find_show_sections(&self) -> Result<Vec<Directory>> {
        let libs = self.get_libraries().await?;
        Ok(libs
            .into_iter()
            .filter(|d| d.section_type == "show")
            .collect())
    }

    pub async fn resolve_tv_section_id(&self, plex_section: Option<&str>) -> Result<String> {
        if let Some(id) = plex_section {
            return Ok(id.to_string());
        }

        let sections = self.find_show_sections().await?;
        match sections.len() {
            0 => bail!("No TV show libraries found on Plex server."),
            1 => Ok(sections[0].key.clone()),
            _ => {
                let list: Vec<String> = sections
                    .iter()
                    .map(|s| format!("  {} — {}", s.key, s.title))
                    .collect();
                bail!(
                    "Multiple TV libraries found on Plex. Specify one with --plex-tv-section:\n{}",
                    list.join("\n")
                );
            }
        }
    }

    pub async fn find_movie_sections(&self) -> Result<Vec<Directory>> {
        let libs = self.get_libraries().await?;
        Ok(libs
            .into_iter()
            .filter(|d| d.section_type == "movie")
            .collect())
    }

    pub async fn resolve_section_id(&self, plex_section: Option<&str>) -> Result<String> {
        if let Some(id) = plex_section {
            return Ok(id.to_string());
        }

        let movie_sections = self.find_movie_sections().await?;
        match movie_sections.len() {
            0 => bail!("No movie libraries found on Plex server."),
            1 => Ok(movie_sections[0].key.clone()),
            _ => {
                let list: Vec<String> = movie_sections
                    .iter()
                    .map(|s| format!("  {} — {}", s.key, s.title))
                    .collect();
                bail!(
                    "Multiple movie libraries found on Plex. Specify one with --plex-section:\n{}",
                    list.join("\n")
                );
            }
        }
    }
}

impl Video {
    pub fn external_ids(&self) -> PlexExternalIds {
        parse_external_ids(&self.guids)
    }

    pub fn parsed_view_count(&self) -> u64 {
        self.view_count
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    pub fn parsed_last_viewed_at(&self) -> Option<i64> {
        self.last_viewed_at.as_deref().and_then(|s| s.parse().ok())
    }

    pub fn watch_data(&self) -> PlexWatchData {
        PlexWatchData {
            view_count: self.parsed_view_count(),
            last_viewed_at: self.parsed_last_viewed_at(),
        }
    }
}

/// Build a map from Radarr movie ID to Plex watch data.
/// Matches by TMDB ID first, then IMDB ID.
pub fn match_movies(
    radarr_movies: &[(i64, Option<i64>, Option<String>)], // (id, tmdb_id, imdb_id)
    plex_movies: &[Video],
) -> HashMap<i64, PlexWatchData> {
    // Build Plex lookup indexes
    let mut plex_by_tmdb: HashMap<i64, &Video> = HashMap::new();
    let mut plex_by_imdb: HashMap<String, &Video> = HashMap::new();

    for video in plex_movies {
        let ids = video.external_ids();
        if let Some(tmdb) = ids.tmdb_id {
            plex_by_tmdb.insert(tmdb, video);
        }
        if let Some(imdb) = ids.imdb_id {
            plex_by_imdb.insert(imdb, video);
        }
    }

    let mut result = HashMap::new();
    for (radarr_id, tmdb_id, imdb_id) in radarr_movies {
        let matched = tmdb_id.and_then(|id| plex_by_tmdb.get(&id)).or_else(|| {
            imdb_id
                .as_ref()
                .and_then(|id| plex_by_imdb.get(id.as_str()))
        });

        if let Some(video) = matched {
            result.insert(*radarr_id, video.watch_data());
        }
    }

    result
}

impl Show {
    pub fn external_ids(&self) -> PlexExternalIds {
        parse_external_ids(&self.guids)
    }
}

impl Episode {
    fn parse_i64(v: &Option<String>) -> Option<i64> {
        v.as_deref().and_then(|s| s.parse().ok())
    }

    pub fn season_episode(&self) -> Option<(i64, i64)> {
        Some((
            Self::parse_i64(&self.season_number)?,
            Self::parse_i64(&self.episode_number)?,
        ))
    }

    pub fn watch_data(&self) -> PlexWatchData {
        PlexWatchData {
            view_count: Self::parse_i64(&self.view_count).unwrap_or(0).max(0) as u64,
            last_viewed_at: Self::parse_i64(&self.last_viewed_at),
        }
    }
}

/// Identifiers for a Sonarr series used to match against a Plex show.
#[derive(Debug, Clone, Default)]
pub struct SeriesIds {
    pub tvdb_id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
}

/// Sonarr episode reference: (episode_id, series_id, season, episode).
pub type EpisodeRef = (i64, i64, i64, i64);

/// Build a map from Sonarr episode ID to Plex watch data.
/// Series are matched by TVDB, then TMDB, then IMDB; episodes by (season, episode).
pub fn match_episodes(
    series: &[(i64, SeriesIds)],
    episodes: &[EpisodeRef],
    plex_shows: &[Show],
    plex_episodes: &[Episode],
) -> HashMap<i64, PlexWatchData> {
    let mut show_by_tvdb: HashMap<i64, &str> = HashMap::new();
    let mut show_by_tmdb: HashMap<i64, &str> = HashMap::new();
    let mut show_by_imdb: HashMap<String, &str> = HashMap::new();
    for show in plex_shows {
        let ids = show.external_ids();
        if let Some(id) = ids.tvdb_id {
            show_by_tvdb.insert(id, &show.rating_key);
        }
        if let Some(id) = ids.tmdb_id {
            show_by_tmdb.insert(id, &show.rating_key);
        }
        if let Some(id) = ids.imdb_id {
            show_by_imdb.insert(id, &show.rating_key);
        }
    }

    // Sonarr series id -> Plex show rating key
    let mut series_to_show: HashMap<i64, &str> = HashMap::new();
    for (series_id, ids) in series {
        let key = ids
            .tvdb_id
            .and_then(|id| show_by_tvdb.get(&id))
            .or_else(|| ids.tmdb_id.and_then(|id| show_by_tmdb.get(&id)))
            .or_else(|| ids.imdb_id.as_deref().and_then(|id| show_by_imdb.get(id)));
        if let Some(key) = key {
            series_to_show.insert(*series_id, key);
        }
    }

    // (show rating key, season, episode) -> Plex episode
    let mut plex_index: HashMap<(&str, i64, i64), &Episode> = HashMap::new();
    for ep in plex_episodes {
        if let (Some(show_key), Some((s, e))) = (ep.show_rating_key.as_deref(), ep.season_episode())
        {
            plex_index.insert((show_key, s, e), ep);
        }
    }

    let mut result = HashMap::new();
    for (episode_id, series_id, season, episode) in episodes {
        if let Some(ep) = series_to_show
            .get(series_id)
            .and_then(|show_key| plex_index.get(&(*show_key, *season, *episode)))
        {
            result.insert(*episode_id, ep.watch_data());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(id: &str) -> GuidElement {
        GuidElement { id: id.to_string() }
    }

    fn show(key: &str, guids: &[&str]) -> Show {
        Show {
            rating_key: key.into(),
            guids: guids.iter().map(|g| guid(g)).collect(),
        }
    }

    fn ep(show: &str, s: i64, e: i64, views: &str, last: Option<&str>) -> Episode {
        Episode {
            show_rating_key: Some(show.into()),
            season_number: Some(s.to_string()),
            episode_number: Some(e.to_string()),
            view_count: Some(views.into()),
            last_viewed_at: last.map(String::from),
        }
    }

    #[test]
    fn matches_by_tvdb_then_tmdb_then_imdb() {
        let shows = vec![
            show("10", &["tvdb://100"]),
            show("20", &["tmdb://200"]),
            show("30", &["imdb://tt300"]),
        ];
        let eps = vec![
            ep("10", 1, 1, "2", Some("1000")),
            ep("20", 2, 3, "0", None),
            ep("30", 1, 1, "5", Some("50")),
        ];
        let series = vec![
            (
                1,
                SeriesIds {
                    tvdb_id: Some(100),
                    ..Default::default()
                },
            ),
            (
                2,
                SeriesIds {
                    tmdb_id: Some(200),
                    ..Default::default()
                },
            ),
            (
                3,
                SeriesIds {
                    imdb_id: Some("tt300".into()),
                    ..Default::default()
                },
            ),
            (
                4,
                SeriesIds {
                    tvdb_id: Some(999),
                    ..Default::default()
                },
            ),
        ];
        let refs = vec![
            (11, 1, 1, 1),
            (22, 2, 2, 3),
            (33, 3, 1, 1),
            (44, 4, 1, 1),
            (12, 1, 9, 9),
        ];
        let m = match_episodes(&series, &refs, &shows, &eps);
        assert_eq!(m[&11].view_count, 2);
        assert_eq!(m[&11].last_viewed_at, Some(1000));
        assert_eq!(m[&22].view_count, 0);
        assert_eq!(m[&33].view_count, 5);
        assert!(!m.contains_key(&44), "unmatched show");
        assert!(!m.contains_key(&12), "unmatched episode number");
    }

    #[test]
    fn parses_show_guids() {
        let s = show(
            "1",
            &[
                "plex://show/abc",
                "tvdb://81189",
                "tmdb://1396",
                "imdb://tt0903747",
            ],
        );
        let ids = s.external_ids();
        assert_eq!(ids.tvdb_id, Some(81189));
        assert_eq!(ids.tmdb_id, Some(1396));
        assert_eq!(ids.imdb_id.as_deref(), Some("tt0903747"));
    }
}

#[cfg(test)]
mod xml_tests {
    use super::*;

    #[test]
    fn parses_plex_video_and_episode_xml() {
        let xml = r#"<MediaContainer size="1">
  <Video ratingKey="1" title="Heat" year="1995" viewCount="2" lastViewedAt="1700000000" guid="plex://movie/abc">
    <Guid id="tmdb://949"/><Guid id="imdb://tt0113277"/>
  </Video>
</MediaContainer>"#;
        let c: VideosContainer = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(c.videos.len(), 1);
        let v = &c.videos[0];
        assert_eq!(v.external_ids().tmdb_id, Some(949));
        assert_eq!(v.parsed_view_count(), 2);

        let xml = r#"<MediaContainer>
  <Video ratingKey="9" grandparentRatingKey="5" parentIndex="2" index="3" viewCount="1" lastViewedAt="10"/>
  <Video ratingKey="10" grandparentRatingKey="5" parentIndex="2" index="4"/>
</MediaContainer>"#;
        let c: EpisodesContainer = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(c.episodes[0].season_episode(), Some((2, 3)));
        assert_eq!(c.episodes[1].watch_data().view_count, 0);

        let xml = r#"<MediaContainer><Directory key="1" title="Movies" type="movie"/><Directory key="2" title="TV" type="show"/></MediaContainer>"#;
        let c: SectionsContainer = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(c.directories[1].section_type, "show");
    }
}
