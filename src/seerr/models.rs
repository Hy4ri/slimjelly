#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Paginated search response from `GET /api/v1/search`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrSearchResponse {
    pub page: Option<i32>,
    pub total_pages: Option<i32>,
    pub total_results: Option<i32>,
    pub results: Vec<SeerrSearchResult>,
}

/// A single search result — may be a movie or tv show.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrSearchResult {
    pub id: Option<i64>,
    pub media_type: Option<String>,
    /// Movie title (present when `media_type == "movie"`).
    pub title: Option<String>,
    /// TV show name (present when `media_type == "tv"`).
    pub name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub media_info: Option<SeerrMediaInfo>,
}

impl SeerrSearchResult {
    /// Returns the display title regardless of media type.
    pub fn display_title(&self) -> String {
        self.title
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Returns the release year as a string.
    pub fn year(&self) -> Option<String> {
        self.release_date
            .as_deref()
            .or(self.first_air_date.as_deref())
            .and_then(|d| d.get(..4))
            .map(str::to_string)
    }
}

// ---------------------------------------------------------------------------
// Media info / status
// ---------------------------------------------------------------------------

/// Media info embedded in search results and detail responses.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrMediaInfo {
    pub id: Option<i64>,
    /// 1 = unknown, 2 = pending, 3 = processing, 4 = partially available, 5 = available.
    pub status: Option<i32>,
    pub requests: Option<Vec<SeerrRequestSummary>>,
}

/// Minimal request info nested in media info.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrRequestSummary {
    pub id: Option<i64>,
    /// 1 = pending, 2 = approved, 3 = declined.
    pub status: Option<i32>,
}

// ---------------------------------------------------------------------------
// Request list
// ---------------------------------------------------------------------------

/// Paginated response from `GET /api/v1/request`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrRequestResponse {
    pub page_info: Option<SeerrPageInfo>,
    pub results: Vec<SeerrRequest>,
}

/// Pagination metadata.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrPageInfo {
    pub pages: Option<i32>,
    pub page_size: Option<i32>,
    pub results: Option<i32>,
    pub page: Option<i32>,
}

/// A media request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrRequest {
    pub id: Option<i64>,
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    pub status: Option<i32>,
    pub media: Option<SeerrRequestMedia>,
    pub created_at: Option<String>,
    pub requested_by: Option<SeerrUser>,
}

/// Media information within a request item.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrRequestMedia {
    pub id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub status: Option<i32>,
    pub media_type: Option<String>,
}

/// User summary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrUser {
    pub id: Option<i64>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
}

// ---------------------------------------------------------------------------
// Create request
// ---------------------------------------------------------------------------

/// Request body for `POST /api/v1/request`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrCreateRequest {
    pub media_type: String,
    pub media_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<SeerrSeasons>,
}

/// Season selection — either "all" or a list of season numbers.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SeerrSeasons {
    All(String),
    Selected(Vec<i32>),
}

// ---------------------------------------------------------------------------
// Movie / TV detail (for checking status of a specific item)
// ---------------------------------------------------------------------------

/// Response from `GET /api/v1/movie/{tmdbId}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrMovieDetails {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub media_info: Option<SeerrMediaInfo>,
}

/// Response from `GET /api/v1/tv/{tmdbId}`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerrTvDetails {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub first_air_date: Option<String>,
    pub media_info: Option<SeerrMediaInfo>,
}

// ---------------------------------------------------------------------------
// Status helpers
// ---------------------------------------------------------------------------

/// Human-readable media availability status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaStatus {
    Unknown,
    Pending,
    Processing,
    PartiallyAvailable,
    Available,
}

impl MediaStatus {
    /// Parse from integer status code used by Jellyseerr / Overseerr.
    pub fn from_code(code: i32) -> Self {
        match code {
            2 => Self::Pending,
            3 => Self::Processing,
            4 => Self::PartiallyAvailable,
            5 => Self::Available,
            _ => Self::Unknown,
        }
    }

    /// Display label for UI badges.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Not Requested",
            Self::Pending => "Pending",
            Self::Processing => "Processing",
            Self::PartiallyAvailable => "Partial",
            Self::Available => "Available",
        }
    }
}

/// Human-readable request approval status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    Approved,
    Declined,
    Unknown,
}

impl RequestStatus {
    /// Parse from integer status code used by Jellyseerr / Overseerr.
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => Self::Pending,
            2 => Self::Approved,
            3 => Self::Declined,
            _ => Self::Unknown,
        }
    }

    /// Display label for UI badges.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Approved => "Approved",
            Self::Declined => "Declined",
            Self::Unknown => "Unknown",
        }
    }
}
