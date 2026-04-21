use reqwest::{Client, Url};
use serde::{Serialize, de::DeserializeOwned};

use crate::{config::ServerConfig, error::AppError};

use super::models::{
    AuthenticateUserByName, AuthenticationResult, BaseItemDto, BaseItemDtoQueryResult,
    PlaybackInfoRequest, PlaybackInfoResponse, PlaybackProgressInfo, PlaybackStartInfo,
    PlaybackStopInfo, PublicSystemInfo, SearchHintResult, TaskInfo, UserDto, VirtualFolderInfo,
};

const CLIENT_DEVICE: &str = "slimjelly";

#[derive(Debug, Clone)]
pub struct JellyfinClient {
    base_url: Url,
    client: Client,
    token: Option<String>,
    device_id: String,
}

impl JellyfinClient {
    pub fn new(server: &ServerConfig, device_id: String) -> Result<Self, AppError> {
        let mut builder = reqwest::Client::builder();
        if server.allow_self_signed {
            builder = builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        let base_url = normalize_base_url(&server.base_url)?;

        Ok(Self {
            base_url,
            client: builder.build()?,
            token: None,
            device_id,
        })
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub async fn authenticate_by_name(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthenticationResult, AppError> {
        let req = AuthenticateUserByName {
            username: username.to_string(),
            pw: password.to_string(),
        };

        self.post_json("Users/AuthenticateByName", &req, false)
            .await
    }

    pub async fn get_me(&self) -> Result<UserDto, AppError> {
        self.get_json("Users/Me", &[]).await
    }

    pub async fn ping(&self) -> Result<String, AppError> {
        self.get_text("System/Ping", &[]).await
    }

    pub async fn public_info(&self) -> Result<PublicSystemInfo, AppError> {
        self.get_json("System/Info/Public", &[]).await
    }

    pub async fn user_views(&self, user_id: &str) -> Result<BaseItemDtoQueryResult, AppError> {
        self.get_json("UserViews", &[("userId", user_id)]).await
    }

    pub async fn playlists(&self, user_id: &str) -> Result<BaseItemDtoQueryResult, AppError> {
        let params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("includeItemTypes".to_string(), "Playlist".to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("sortBy".to_string(), "SortName".to_string()),
            ("sortOrder".to_string(), "Ascending".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("limit".to_string(), "200".to_string()),
        ];

        self.get_json_owned("Items", &params).await
    }

    pub async fn last_played_item(&self, user_id: &str) -> Result<Option<BaseItemDto>, AppError> {
        let params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("sortBy".to_string(), "DatePlayed".to_string()),
            ("sortOrder".to_string(), "Descending".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("limit".to_string(), "1".to_string()),
            (
                "includeItemTypes".to_string(),
                "Movie,Episode,Audio,AudioBook,Program".to_string(),
            ),
        ];

        let result: BaseItemDtoQueryResult = self.get_json_owned("Items", &params).await?;
        Ok(result.items.and_then(|items| items.into_iter().next()))
    }

    pub async fn items(
        &self,
        user_id: &str,
        parent_id: Option<&str>,
        search: Option<&str>,
        include_item_types: &[&str],
        start_index: i32,
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let mut params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("startIndex".to_string(), start_index.to_string()),
            ("limit".to_string(), limit.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
        ];

        if let Some(parent) = parent_id {
            params.push(("parentId".to_string(), parent.to_string()));
        }
        if let Some(search_term) = search {
            if !search_term.is_empty() {
                params.push(("searchTerm".to_string(), search_term.to_string()));
            }
        }
        if !include_item_types.is_empty() {
            params.push(("includeItemTypes".to_string(), include_item_types.join(",")));
        }

        self.get_json_owned("Items", &params).await
    }

    pub async fn item(&self, user_id: &str, item_id: &str) -> Result<BaseItemDto, AppError> {
        let path = format!("Items/{item_id}");
        self.get_json(&path, &[("userId", user_id)]).await
    }

    pub async fn continue_watching(
        &self,
        user_id: &str,
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("isResumable".to_string(), "true".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            (
                "includeItemTypes".to_string(),
                "Movie,Episode,AudioBook,Audio".to_string(),
            ),
            ("sortBy".to_string(), "DatePlayed".to_string()),
            ("sortOrder".to_string(), "Descending".to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        self.get_json_owned("Items", &params).await
    }

    pub async fn recent_items_by_types(
        &self,
        user_id: &str,
        include_types: &[&str],
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let mut params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("sortBy".to_string(), "DateCreated".to_string()),
            ("sortOrder".to_string(), "Descending".to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        if !include_types.is_empty() {
            params.push(("includeItemTypes".to_string(), include_types.join(",")));
        }

        self.get_json_owned("Items", &params).await
    }

    pub async fn random_item_by_types(
        &self,
        user_id: &str,
        include_types: &[&str],
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let mut params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("sortBy".to_string(), "Random".to_string()),
            ("sortOrder".to_string(), "Ascending".to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        if !include_types.is_empty() {
            params.push(("includeItemTypes".to_string(), include_types.join(",")));
        }

        self.get_json_owned("Items", &params).await
    }

    pub async fn library_items_by_types(
        &self,
        user_id: &str,
        include_types: &[&str],
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let mut params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("sortBy".to_string(), "SortName".to_string()),
            ("sortOrder".to_string(), "Ascending".to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        if !include_types.is_empty() {
            params.push(("includeItemTypes".to_string(), include_types.join(",")));
        }

        self.get_json_owned("Items", &params).await
    }

    pub async fn collections(
        &self,
        user_id: &str,
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("recursive".to_string(), "true".to_string()),
            ("includeItemTypes".to_string(), "BoxSet".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("sortBy".to_string(), "SortName".to_string()),
            ("sortOrder".to_string(), "Ascending".to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        self.get_json_owned("Items", &params).await
    }

    pub async fn mark_played(&self, user_id: &str, item_id: &str) -> Result<(), AppError> {
        let path = format!("Users/{user_id}/PlayedItems/{item_id}");
        self.post_no_body_no_content(&path, &[]).await
    }

    pub async fn mark_unplayed(&self, user_id: &str, item_id: &str) -> Result<(), AppError> {
        let path = format!("UserPlayedItems/{item_id}");
        self.delete_no_body_no_content(&path, &[("userId", user_id)])
            .await
    }

    pub async fn add_items_to_playlist(
        &self,
        playlist_id: &str,
        user_id: &str,
        item_ids: &[&str],
    ) -> Result<(), AppError> {
        let path = format!("Playlists/{playlist_id}/Items");
        let ids = item_ids.join(",");
        self.post_no_body_no_content(&path, &[("userId", user_id), ("ids", &ids)])
            .await
    }

    pub async fn delete_item(&self, item_id: &str) -> Result<(), AppError> {
        let path = format!("Items/{item_id}");
        self.delete_no_body_no_content(&path, &[]).await
    }

    pub async fn virtual_folders(&self) -> Result<Vec<VirtualFolderInfo>, AppError> {
        self.get_json("Library/VirtualFolders", &[]).await
    }

    pub async fn remove_virtual_folder(
        &self,
        name: &str,
        refresh_library: bool,
    ) -> Result<(), AppError> {
        self.delete_no_body_no_content(
            "Library/VirtualFolders",
            &[
                ("name", name),
                (
                    "refreshLibrary",
                    if refresh_library { "true" } else { "false" },
                ),
            ],
        )
        .await
    }

    pub async fn seasons(
        &self,
        user_id: &str,
        series_id: &str,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let path = format!("Shows/{series_id}/Seasons");
        self.get_json(&path, &[("userId", user_id)]).await
    }

    pub async fn episodes_for_season(
        &self,
        user_id: &str,
        season_id: &str,
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("parentId".to_string(), season_id.to_string()),
            ("recursive".to_string(), "false".to_string()),
            ("includeItemTypes".to_string(), "Episode".to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
            ("sortBy".to_string(), "SortName".to_string()),
            ("sortOrder".to_string(), "Ascending".to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        self.get_json_owned("Items", &params).await
    }

    pub async fn similar_items(
        &self,
        user_id: &str,
        item_id: &str,
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let path = format!("Items/{item_id}/Similar");
        let limit_string = limit.to_string();
        self.get_json(
            &path,
            &[
                ("userId", user_id),
                ("limit", &limit_string),
                ("enableUserData", "true"),
                ("enableImages", "true"),
                ("imageTypeLimit", "1"),
            ],
        )
        .await
    }

    pub async fn search_hints(
        &self,
        user_id: &str,
        term: &str,
        limit: i32,
    ) -> Result<SearchHintResult, AppError> {
        self.get_json(
            "Search/Hints",
            &[
                ("userId", user_id),
                ("searchTerm", term),
                ("limit", &limit.to_string()),
            ],
        )
        .await
    }

    pub async fn playback_info(
        &self,
        item_id: &str,
        request: &PlaybackInfoRequest,
    ) -> Result<PlaybackInfoResponse, AppError> {
        let path = format!("Items/{item_id}/PlaybackInfo");
        self.post_json(&path, request, true).await
    }

    pub fn build_video_stream_url(
        &self,
        item_id: &str,
        media_source_id: Option<&str>,
        play_session_id: Option<&str>,
        audio_stream_index: Option<i32>,
        subtitle_stream_index: Option<i32>,
        transcode: bool,
    ) -> Result<String, AppError> {
        let mut url = self.base_url.join(&format!("Videos/{item_id}/stream"))?;

        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("static", if transcode { "false" } else { "true" });
            if let Some(source_id) = media_source_id {
                qp.append_pair("mediaSourceId", source_id);
            }
            if let Some(session_id) = play_session_id {
                qp.append_pair("playSessionId", session_id);
            }
            if let Some(audio_idx) = audio_stream_index {
                qp.append_pair("audioStreamIndex", &audio_idx.to_string());
            }
            if let Some(sub_idx) = subtitle_stream_index {
                qp.append_pair("subtitleStreamIndex", &sub_idx.to_string());
            }
            if let Some(token) = self.token() {
                qp.append_pair("api_key", token);
            }
        }

        Ok(url.to_string())
    }

    /// Build a URL to fetch a subtitle stream file from the Jellyfin server.
    ///
    /// `format` is the desired output format, e.g. `"srt"`, `"vtt"`, or `"ass"`.
    pub fn build_subtitle_url(
        &self,
        item_id: &str,
        media_source_id: &str,
        stream_index: i32,
        format: &str,
    ) -> Result<String, AppError> {
        let path = format!(
            "Videos/{item_id}/{media_source_id}/Subtitles/{stream_index}/Stream.{format}"
        );
        let mut url = self.base_url.join(&path)?;
        if let Some(token) = self.token() {
            url.query_pairs_mut().append_pair("api_key", token);
        }
        Ok(url.to_string())
    }

    pub fn build_item_image_url(
        &self,
        item_id: &str,
        image_type: &str,
        max_width: u32,
        max_height: u32,
        tag: Option<&str>,
    ) -> Result<String, AppError> {
        let mut url = self
            .base_url
            .join(&format!("Items/{item_id}/Images/{image_type}"))?;

        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("maxWidth", &max_width.to_string());
            qp.append_pair("maxHeight", &max_height.to_string());
            qp.append_pair("quality", "90");
            qp.append_pair("fillHeight", &max_height.to_string());
            if let Some(tag) = tag {
                qp.append_pair("tag", tag);
            }
            if let Some(token) = self.token() {
                qp.append_pair("api_key", token);
            }
        }

        Ok(url.to_string())
    }

    pub async fn fetch_image_bytes(&self, url: &str) -> Result<Vec<u8>, AppError> {
        let mut req = self.client.get(url);
        req = self.with_auth(req);

        let response = req.send().await?;
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

    pub async fn report_playing_start(&self, payload: &PlaybackStartInfo) -> Result<(), AppError> {
        self.post_json_no_content("Sessions/Playing", payload).await
    }

    pub async fn report_playing_progress(
        &self,
        payload: &PlaybackProgressInfo,
    ) -> Result<(), AppError> {
        self.post_json_no_content("Sessions/Playing/Progress", payload)
            .await
    }

    pub async fn report_playing_stopped(&self, payload: &PlaybackStopInfo) -> Result<(), AppError> {
        self.post_json_no_content("Sessions/Playing/Stopped", payload)
            .await
    }

    pub async fn report_playing_ping(&self, play_session_id: &str) -> Result<(), AppError> {
        self.post_no_body_no_content(
            "Sessions/Playing/Ping",
            &[("playSessionId", play_session_id)],
        )
        .await
    }

    pub async fn library_refresh_all(&self) -> Result<(), AppError> {
        self.post_no_body_no_content("Library/Refresh", &[]).await
    }

    pub async fn item_refresh(&self, item_id: &str) -> Result<(), AppError> {
        let path = format!("Items/{item_id}/Refresh");
        self.post_no_body_no_content(&path, &[]).await
    }

    pub async fn playlist_items(
        &self,
        playlist_id: &str,
        user_id: &str,
        start_index: i32,
        limit: i32,
    ) -> Result<BaseItemDtoQueryResult, AppError> {
        let path = format!("Playlists/{playlist_id}/Items");
        let params = vec![
            ("userId".to_string(), user_id.to_string()),
            ("startIndex".to_string(), start_index.to_string()),
            ("limit".to_string(), limit.to_string()),
            ("enableUserData".to_string(), "true".to_string()),
            ("enableImages".to_string(), "true".to_string()),
            ("imageTypeLimit".to_string(), "1".to_string()),
        ];

        self.get_json_owned(&path, &params).await
    }

    pub async fn scheduled_tasks(&self) -> Result<Vec<TaskInfo>, AppError> {
        self.get_json("ScheduledTasks", &[]).await
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, AppError> {
        let mut req = self.client.get(self.make_url(path)?).query(query);
        req = self.with_auth(req);

        let response = req.send().await?;
        parse_json_response(response).await
    }

    async fn get_json_owned<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, AppError> {
        let mut req = self.client.get(self.make_url(path)?).query(query);
        req = self.with_auth(req);

        let response = req.send().await?;
        parse_json_response(response).await
    }

    async fn get_text(&self, path: &str, query: &[(&str, &str)]) -> Result<String, AppError> {
        let mut req = self.client.get(self.make_url(path)?).query(query);
        req = self.with_auth(req);

        let response = req.send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::ApiStatus {
                status: status.as_u16(),
                message: body,
            });
        }
        Ok(body)
    }

    async fn post_json<TReq: Serialize, TResp: DeserializeOwned>(
        &self,
        path: &str,
        payload: &TReq,
        requires_auth: bool,
    ) -> Result<TResp, AppError> {
        let mut req = self.client.post(self.make_url(path)?).json(payload);
        if requires_auth {
            req = self.with_auth(req);
        } else {
            req = req.header("X-Emby-Authorization", self.authorization_header(None));
        }

        let response = req.send().await?;
        parse_json_response(response).await
    }

    async fn post_json_no_content<TReq: Serialize>(
        &self,
        path: &str,
        payload: &TReq,
    ) -> Result<(), AppError> {
        let mut req = self.client.post(self.make_url(path)?).json(payload);
        req = self.with_auth(req);

        let response = req.send().await?;
        parse_empty_response(response).await
    }

    async fn post_no_body_no_content(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<(), AppError> {
        let mut req = self.client.post(self.make_url(path)?).query(query);
        req = self.with_auth(req);

        let response = req.send().await?;
        parse_empty_response(response).await
    }

    async fn delete_no_body_no_content(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<(), AppError> {
        let mut req = self.client.delete(self.make_url(path)?).query(query);
        req = self.with_auth(req);

        let response = req.send().await?;
        parse_empty_response(response).await
    }

    fn with_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header(
            "X-Emby-Authorization",
            self.authorization_header(self.token()),
        )
    }

    fn authorization_header(&self, token: Option<&str>) -> String {
        match token {
            Some(value) => format!(
                "MediaBrowser Client=\"slimjelly\", Device=\"{CLIENT_DEVICE}\", DeviceId=\"{}\", Version=\"0.1.0\", Token=\"{value}\"",
                self.device_id
            ),
            None => format!(
                "MediaBrowser Client=\"slimjelly\", Device=\"{CLIENT_DEVICE}\", DeviceId=\"{}\", Version=\"0.1.0\"",
                self.device_id
            ),
        }
    }

    fn make_url(&self, path: &str) -> Result<Url, AppError> {
        self.base_url
            .join(path)
            .map_err(|err| AppError::Config(format!("invalid endpoint path '{path}': {err}")))
    }
}

fn normalize_base_url(input: &str) -> Result<Url, AppError> {
    let mut raw = input.trim().to_string();
    if raw.is_empty() {
        return Err(AppError::Config(
            "server.base_url must not be empty".to_string(),
        ));
    }

    if !raw.starts_with("http://") && !raw.starts_with("https://") {
        let scheme = inferred_scheme_for_host(&raw);
        raw = format!("{scheme}://{raw}");
    }

    if !raw.ends_with('/') {
        raw.push('/');
    }

    Url::parse(&raw)
        .map_err(|err| AppError::Config(format!("invalid server.base_url '{input}': {err}")))
}

fn inferred_scheme_for_host(raw: &str) -> &'static str {
    if is_local_host(raw) { "http" } else { "https" }
}

fn is_local_host(raw: &str) -> bool {
    let host_with_port = raw.split('/').next().unwrap_or(raw);
    let host = if host_with_port.starts_with('[') {
        host_with_port
            .split_once(']')
            .map(|(inside, _)| inside.trim_start_matches('['))
            .unwrap_or(host_with_port)
    } else {
        host_with_port.split(':').next().unwrap_or(host_with_port)
    };

    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    if let Ok(ipv4) = host.parse::<std::net::Ipv4Addr>() {
        let [a, b, ..] = ipv4.octets();
        return a == 10
            || (a == 172 && (16..=31).contains(&b))
            || (a == 192 && b == 168)
            || a == 127
            || (a == 169 && b == 254);
    }

    if let Ok(ipv6) = host.parse::<std::net::Ipv6Addr>() {
        return ipv6.is_loopback() || ipv6.is_unique_local() || ipv6.is_unicast_link_local();
    }

    false
}

async fn parse_json_response<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, AppError> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::ApiStatus {
            status: status.as_u16(),
            message: body,
        });
    }

    Ok(response.json::<T>().await?)
}

async fn parse_empty_response(response: reqwest::Response) -> Result<(), AppError> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::ApiStatus {
            status: status.as_u16(),
            message: body,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::ServerConfig;

    use super::*;

    fn server(base_url: &str) -> ServerConfig {
        ServerConfig {
            base_url: base_url.to_string(),
            username: "user".to_string(),
            allow_self_signed: false,
        }
    }

    #[test]
    fn normalize_base_url_adds_https_and_trailing_slash() -> Result<(), AppError> {
        let normalized = normalize_base_url("jellyfin.local:8096")?;
        assert_eq!(normalized.as_str(), "https://jellyfin.local:8096/");
        Ok(())
    }

    #[test]
    fn normalize_base_url_uses_http_for_private_ipv4() -> Result<(), AppError> {
        let normalized = normalize_base_url("192.168.1.48:8096")?;
        assert_eq!(normalized.as_str(), "http://192.168.1.48:8096/");
        Ok(())
    }

    #[test]
    fn normalize_base_url_uses_http_for_localhost() -> Result<(), AppError> {
        let normalized = normalize_base_url("localhost:8096")?;
        assert_eq!(normalized.as_str(), "http://localhost:8096/");
        Ok(())
    }

    #[test]
    fn normalize_base_url_trims_whitespace() -> Result<(), AppError> {
        let normalized = normalize_base_url("  https://example.com/base  ")?;
        assert_eq!(normalized.as_str(), "https://example.com/base/");
        Ok(())
    }

    #[test]
    fn normalize_base_url_rejects_empty_input() {
        let result = normalize_base_url("   ");
        assert!(matches!(
            result,
            Err(AppError::Config(message)) if message.contains("must not be empty")
        ));
    }

    #[test]
    fn authorization_header_without_token_omits_token_field() -> Result<(), AppError> {
        let client = JellyfinClient::new(&server("https://example.com"), "device-1".to_string())?;
        let header = client.authorization_header(None);
        assert!(header.contains("Client=\"slimjelly\""));
        assert!(header.contains("DeviceId=\"device-1\""));
        assert!(!header.contains("Token=\""));
        Ok(())
    }

    #[test]
    fn authorization_header_with_token_includes_token_field() -> Result<(), AppError> {
        let client = JellyfinClient::new(&server("https://example.com"), "device-2".to_string())?;
        let header = client.authorization_header(Some("abc123"));
        assert!(header.contains("DeviceId=\"device-2\""));
        assert!(header.contains("Token=\"abc123\""));
        Ok(())
    }

    #[test]
    fn build_video_stream_url_contains_expected_query_values() -> Result<(), AppError> {
        let mut client =
            JellyfinClient::new(&server("https://example.com"), "device-3".to_string())?;
        client.set_token(Some("tok".to_string()));

        let url = client.build_video_stream_url(
            "item1",
            Some("source1"),
            Some("session1"),
            Some(2),
            Some(4),
            true,
        )?;

        let parsed = Url::parse(&url).expect("generated URL must parse");
        assert_eq!(parsed.path(), "/Videos/item1/stream");

        let pairs: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();

        assert_eq!(pairs.get("static").map(String::as_str), Some("false"));
        assert_eq!(
            pairs.get("mediaSourceId").map(String::as_str),
            Some("source1")
        );
        assert_eq!(
            pairs.get("playSessionId").map(String::as_str),
            Some("session1")
        );
        assert_eq!(pairs.get("audioStreamIndex").map(String::as_str), Some("2"));
        assert_eq!(
            pairs.get("subtitleStreamIndex").map(String::as_str),
            Some("4")
        );
        assert_eq!(pairs.get("api_key").map(String::as_str), Some("tok"));

        Ok(())
    }
}
