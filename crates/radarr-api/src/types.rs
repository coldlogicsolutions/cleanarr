//! Resource types shared by more than one consumer of the API.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieResource {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub sort_title: Option<String>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub year: Option<i64>,
    pub monitored: Option<bool>,
    pub status: Option<String>,
    pub has_file: Option<bool>,
    pub size_on_disk: Option<i64>,
    pub added: Option<String>,
    pub quality_profile_id: Option<i64>,
    pub minimum_availability: Option<String>,
    pub root_folder_path: Option<String>,
    pub path: Option<String>,
    pub folder_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<i64>,
    // Many more fields exist; serde will ignore unknown fields by default
}
