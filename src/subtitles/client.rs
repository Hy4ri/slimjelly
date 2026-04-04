use reqwest::Client;

use crate::error::AppError;

use super::models::{
    DownloadRequest, DownloadResponse, LoginRequest, LoginResponse, SubtitleSearchResponse,
};

const BASE_URL: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = "slimjelly v0.2.0";

/// HTTP client for the OpenSubtitles REST API.
#[derive(Debug, Clone)]
pub struct OpenSubtitlesClient {
    client: Client,
    api_key: String,
}

impl OpenSubtitlesClient {
    /// Create a new client with the given API key.
    pub fn new(api_key: &str) -> Result<Self, AppError> {
        let client = Client::builder().build()?;
        Ok(Self {
            client,
            api_key: api_key.to_string(),
        })
    }

    /// Search subtitles by text query (video name) and language.
    ///
    /// `languages` is a comma-separated string of ISO 639-1 codes (e.g. `"en"`, `"ar,en"`).
    pub async fn search(
        &self,
        query: &str,
        languages: &str,
    ) -> Result<SubtitleSearchResponse, AppError> {
        let mut params = vec![("query", query.to_string())];
        let languages_trimmed = languages.trim();
        if !languages_trimmed.is_empty() {
            params.push(("languages", languages_trimmed.to_string()));
        }

        let response = self
            .client
            .get(format!("{BASE_URL}/subtitles"))
            .header("Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
            .query(&params)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiStatus {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.json::<SubtitleSearchResponse>().await?)
    }

    /// Authenticate with OpenSubtitles to get a JWT token for downloads.
    pub async fn login(&self, username: &str, password: &str) -> Result<String, AppError> {
        let payload = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(format!("{BASE_URL}/login"))
            .header("Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiStatus {
                status: status.as_u16(),
                message: body,
            });
        }

        let login_resp: LoginResponse = response.json().await?;
        login_resp.token.ok_or_else(|| AppError::ApiStatus {
            status: 200,
            message: "login response contained no token".to_string(),
        })
    }

    /// Request a temporary download link for a subtitle file.
    ///
    /// Requires a valid JWT `token` from [`Self::login`].
    pub async fn download(&self, file_id: i64, token: &str) -> Result<DownloadResponse, AppError> {
        let payload = DownloadRequest {
            file_id,
            sub_format: Some("srt".to_string()),
        };

        let response = self
            .client
            .post(format!("{BASE_URL}/download"))
            .header("Api-Key", &self.api_key)
            .header("User-Agent", USER_AGENT)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiStatus {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.json::<DownloadResponse>().await?)
    }

    /// Download the actual subtitle file bytes from a temporary link.
    pub async fn fetch_subtitle_bytes(&self, url: &str) -> Result<Vec<u8>, AppError> {
        let response = self.client.get(url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ApiStatus {
                status: status.as_u16(),
                message: body,
            });
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}
