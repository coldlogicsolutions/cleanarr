use anyhow::Result;
use serde::Deserialize;

use crate::config::SonarrConfig;
use radarr_api::RadarrClient;

/// Thin Sonarr v3 client. Sonarr's API shares Radarr's transport conventions
/// (X-Api-Key header, JSON error envelope), so we reuse the generic client.
#[derive(Clone)]
pub struct SonarrClient {
    api: RadarrClient,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesStatistics {
    #[serde(default)]
    pub size_on_disk: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesResource {
    pub id: i64,
    pub title: String,
    pub tvdb_id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<i64>,
    pub statistics: Option<SeriesStatistics>,
}

impl SeriesResource {
    pub fn size_on_disk(&self) -> i64 {
        self.statistics
            .as_ref()
            .map(|s| s.size_on_disk)
            .unwrap_or(0)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFileResource {
    pub id: i64,
    #[serde(default)]
    pub size: i64,
    pub date_added: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeResource {
    pub id: i64,
    pub season_number: i64,
    pub episode_number: i64,
    pub title: Option<String>,
    #[serde(default)]
    pub has_file: bool,
    pub episode_file: Option<EpisodeFileResource>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagResource {
    pub id: Option<i64>,
    pub label: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EpisodesMonitored<'a> {
    episode_ids: &'a [i64],
    monitored: bool,
}

impl SonarrClient {
    pub fn new(config: &SonarrConfig) -> Result<Self> {
        Ok(Self {
            api: RadarrClient::new(&config.url, &config.api_key)?,
        })
    }

    pub async fn get_series(&self) -> Result<Vec<SeriesResource>> {
        self.api.get("/api/v3/series").await
    }

    pub async fn get_episodes(&self, series_id: i64) -> Result<Vec<EpisodeResource>> {
        self.api
            .get_with_query(
                "/api/v3/episode",
                &[
                    ("seriesId", &series_id.to_string()),
                    ("includeEpisodeFile", "true"),
                ],
            )
            .await
    }

    pub async fn get_tags(&self) -> Result<Vec<TagResource>> {
        self.api.get("/api/v3/tag").await
    }

    pub async fn delete_episode_file(&self, episode_file_id: i64) -> Result<()> {
        self.api
            .delete(&format!("/api/v3/episodefile/{episode_file_id}"))
            .await
    }

    pub async fn unmonitor_episodes(&self, episode_ids: &[i64]) -> Result<()> {
        let body = EpisodesMonitored {
            episode_ids,
            monitored: false,
        };
        // Sonarr returns 202 Accepted with the updated episodes; we don't need the body.
        let _: serde_json::Value = self.api.put("/api/v3/episode/monitor", &body).await?;
        Ok(())
    }
}
