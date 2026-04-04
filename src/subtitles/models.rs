#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Response from `GET /api/v1/subtitles`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleSearchResponse {
    pub total_pages: Option<i32>,
    pub total_count: Option<i32>,
    pub page: Option<i32>,
    pub data: Option<Vec<SubtitleResult>>,
}

/// A single subtitle entry from search results.
#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleResult {
    pub id: Option<String>,
    pub attributes: Option<SubtitleAttributes>,
}

/// Attributes of a subtitle result.
#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleAttributes {
    pub subtitle_id: Option<String>,
    pub language: Option<String>,
    pub release: Option<String>,
    pub download_count: Option<i64>,
    pub ratings: Option<f64>,
    pub from_trusted: Option<bool>,
    pub files: Option<Vec<SubtitleFile>>,
    pub feature_details: Option<FeatureDetails>,
}

/// Individual file within a subtitle result.
#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleFile {
    pub file_id: Option<i64>,
    pub file_name: Option<String>,
}

/// Feature (movie/episode) details embedded in the result.
#[derive(Debug, Clone, Deserialize)]
pub struct FeatureDetails {
    pub title: Option<String>,
    pub year: Option<i32>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
}

/// Request body for `POST /api/v1/login`.
#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response from `POST /api/v1/login`.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub token: Option<String>,
    pub status: Option<i32>,
}

/// Request body for `POST /api/v1/download`.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadRequest {
    pub file_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_format: Option<String>,
}

/// Response from `POST /api/v1/download`.
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadResponse {
    pub link: Option<String>,
    pub file_name: Option<String>,
    pub requests: Option<i32>,
    pub remaining: Option<i32>,
    pub message: Option<String>,
}
