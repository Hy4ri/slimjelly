use reqwest::Client;
use url::Url;

use crate::error::AppError;

use super::models::{
    SeerrCreateRequest, SeerrRequest, SeerrRequestResponse, SeerrSearchResponse, SeerrSeasons,
};

const USER_AGENT: &str = "slimjelly v0.2.0";

/// HTTP client for the Jellyseerr / Overseerr REST API.
#[derive(Debug, Clone)]
pub struct SeerrClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl SeerrClient {
    /// Create a new client.
    ///
    /// `base_url` should be the root URL of the Jellyseerr instance
    /// (e.g. `http://localhost:5055`). Trailing slashes and `/api/v1`
    /// suffixes are normalized automatically.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        let normalized = Self::normalize_url(base_url)?;
        let client = Client::builder().build()?;
        Ok(Self {
            client,
            base_url: normalized,
            api_key: api_key.to_string(),
        })
    }

    /// Search for movies and TV shows.
    pub async fn search(&self, query: &str, page: i32) -> Result<SeerrSearchResponse, AppError> {
        let url = format!("{}/api/v1/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
            .query(&[
                ("query", query),
                ("page", &page.to_string()),
                ("language", "en"),
            ])
            .send()
            .await?;

        Self::check_status(&response)?;
        Ok(response.json::<SeerrSearchResponse>().await?)
    }

    /// Fetch the user's media requests.
    pub async fn get_requests(
        &self,
        page: i32,
        page_size: i32,
    ) -> Result<SeerrRequestResponse, AppError> {
        let url = format!("{}/api/v1/request", self.base_url);
        let skip = (page.saturating_sub(1)) * page_size;
        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
            .query(&[
                ("take", &page_size.to_string()),
                ("skip", &skip.to_string()),
                ("sort", &"added".to_string()),
                ("filter", &"all".to_string()),
            ])
            .send()
            .await?;

        Self::check_status(&response)?;
        Ok(response.json::<SeerrRequestResponse>().await?)
    }

    /// Submit a movie request.
    pub async fn request_movie(&self, tmdb_id: i64) -> Result<SeerrRequest, AppError> {
        self.create_request(SeerrCreateRequest {
            media_type: "movie".to_string(),
            media_id: tmdb_id,
            seasons: None,
        })
        .await
    }

    /// Submit a TV show request (all seasons).
    pub async fn request_tv(
        &self,
        tmdb_id: i64,
        seasons: Option<Vec<i32>>,
    ) -> Result<SeerrRequest, AppError> {
        let season_value = seasons
            .map(SeerrSeasons::Selected)
            .unwrap_or_else(|| SeerrSeasons::All("all".to_string()));

        self.create_request(SeerrCreateRequest {
            media_type: "tv".to_string(),
            media_id: tmdb_id,
            seasons: Some(season_value),
        })
        .await
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    async fn create_request(&self, body: SeerrCreateRequest) -> Result<SeerrRequest, AppError> {
        let url = format!("{}/api/v1/request", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        Self::check_status(&response)?;
        Ok(response.json::<SeerrRequest>().await?)
    }

    fn check_status(response: &reqwest::Response) -> Result<(), AppError> {
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::ApiStatus {
                status: status.as_u16(),
                message: format!("Jellyseerr API returned {status}"),
            });
        }
        Ok(())
    }

    fn normalize_url(raw: &str) -> Result<String, AppError> {
        let trimmed = raw.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AppError::Config(
                "Jellyseerr URL cannot be empty".to_string(),
            ));
        }

        // Strip /api/v1 suffix if the user pasted the full API path
        let trimmed = trimmed
            .strip_suffix("/api/v1")
            .or_else(|| trimmed.strip_suffix("/api"))
            .unwrap_or(trimmed);

        let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{trimmed}")
        };

        // Validate the URL
        let _parsed = Url::parse(&with_scheme)?;
        Ok(with_scheme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_strips_trailing_slash() {
        let result = SeerrClient::normalize_url("http://localhost:5055/").unwrap();
        assert_eq!(result, "http://localhost:5055");
    }

    #[test]
    fn normalize_url_strips_api_v1_suffix() {
        let result = SeerrClient::normalize_url("http://seerr.local/api/v1").unwrap();
        assert_eq!(result, "http://seerr.local");
    }

    #[test]
    fn normalize_url_adds_http_scheme() {
        let result = SeerrClient::normalize_url("seerr.local:5055").unwrap();
        assert_eq!(result, "http://seerr.local:5055");
    }

    #[test]
    fn normalize_url_preserves_https() {
        let result = SeerrClient::normalize_url("https://seerr.example.com").unwrap();
        assert_eq!(result, "https://seerr.example.com");
    }

    #[test]
    fn normalize_url_rejects_empty() {
        let result = SeerrClient::normalize_url("   ");
        assert!(result.is_err());
    }
}
