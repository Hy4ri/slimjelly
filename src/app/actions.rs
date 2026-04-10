use std::sync::{Arc, Mutex};

use crate::{
    config::{PreferredPlayer, save_config},
    jellyfin::models::{BaseItemDto, PlaybackInfoRequest, PlaybackStartInfo, PlaybackStopInfo},
    secure_store::clear_session,
};

use super::{
    LibrarySection, PlayerKind, Screen, SlimJellyApp, UiMessage,
    playback::{read_mpv_snapshot, run_player_exit_watcher},
};

impl SlimJellyApp {
    pub(super) fn load_playlists(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.playlists(&session.user_id).await {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::PlaylistsLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::PlaylistsFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn load_home_sections(&mut self) {
        self.load_home_continue_watching();
        self.load_home_recent_movies();
        self.load_home_recent_series();
    }

    fn load_home_continue_watching(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.continue_watching(&session.user_id, 20).await {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::HomeContinueWatchingLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::HomeLoadFailed(err.to_string()))
                }
            }
        });
    }

    fn load_home_recent_movies(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client
                .recent_items_by_types(&session.user_id, &["Movie"], 25)
                .await
            {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::HomeRecentMoviesLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::HomeLoadFailed(err.to_string()))
                }
            }
        });
    }

    fn load_home_recent_series(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client
                .recent_items_by_types(&session.user_id, &["Series"], 25)
                .await
            {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::HomeRecentSeriesLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::HomeLoadFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn load_library_items(&mut self, section: LibrarySection) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let include_types: Vec<&str> = match section {
            LibrarySection::Movies => vec!["Movie"],
            LibrarySection::TvShows => vec!["Series"],
            LibrarySection::Music => vec!["MusicAlbum", "Audio"],
            LibrarySection::Audiobooks => vec!["AudioBook"],
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client
                .library_items_by_types(&session.user_id, &include_types, 250)
                .await
            {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::LibraryItemsLoaded {
                        section,
                        items: result.items.unwrap_or_default(),
                    },
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::LibraryItemsFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn load_collections(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.collections(&session.user_id, 250).await {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::CollectionItemsLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::CollectionItemsFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn load_detail_sections(&mut self) {
        let Some(item) = self.selected_item.clone() else {
            self.detail_seasons.clear();
            self.detail_selected_season_id = None;
            self.detail_preferred_season_id = None;
            self.detail_pending_next_season_id = None;
            self.detail_episodes.clear();
            self.detail_related.clear();
            self.detail_media_source = None;
            return;
        };

        let item_type = item.r#type.clone().unwrap_or_default();
        if item_type.eq_ignore_ascii_case("Series") {
            self.detail_pending_next_season_id = None;
            self.load_detail_seasons();
        } else if item_type.eq_ignore_ascii_case("Episode") {
            self.detail_pending_next_season_id = None;
            let mut has_context = false;
            self.detail_seasons.clear();

            if let Some(season_id) = item
                .season_id
                .clone()
                .or_else(|| item.parent_id.clone())
            {
                self.detail_selected_season_id = Some(season_id.clone());
                self.detail_preferred_season_id = Some(season_id.clone());
                self.detail_episodes.clear();
                self.detail_pending_next_season_id = None;
                self.load_detail_episodes(season_id);
                has_context = true;
            } else {
                self.detail_selected_season_id = None;
                self.detail_preferred_season_id = None;
                self.detail_episodes.clear();
            }

            if let Some(series_id) = item.series_id.clone() {
                self.load_detail_seasons_for_series(series_id);
                has_context = true;
            }

            if !has_context {
                self.detail_selected_season_id = None;
                self.detail_preferred_season_id = None;
                self.detail_pending_next_season_id = None;
                self.detail_episodes.clear();
            }
        } else {
            self.detail_seasons.clear();
            self.detail_selected_season_id = None;
            self.detail_preferred_season_id = None;
            self.detail_pending_next_season_id = None;
            self.detail_episodes.clear();
        }

        self.load_detail_related();
        self.load_detail_tech();
    }

    pub(super) fn choose_detail_season(&mut self, season_id: String) {
        self.detail_selected_season_id = Some(season_id.clone());
        self.detail_preferred_season_id = Some(season_id.clone());
        self.detail_pending_next_season_id = None;
        self.detail_episodes.clear();
        self.load_detail_episodes(season_id);
    }

    fn load_detail_seasons_for_series(&mut self, series_id: String) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.seasons(&session.user_id, &series_id).await {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::DetailSeasonsLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::DetailSeasonsFailed(err.to_string()))
                }
            }
        });
    }

    fn load_detail_seasons(&mut self) {
        let Some(item_id) = self.selected_item.as_ref().and_then(|item| item.id.clone()) else {
            return;
        };

        self.load_detail_seasons_for_series(item_id);
    }

    pub(super) fn load_detail_episodes(&mut self, season_id: String) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client
                .episodes_for_season(&session.user_id, &season_id, 200)
                .await
            {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::DetailEpisodesLoaded {
                        season_id,
                        items: result.items.unwrap_or_default(),
                    },
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::DetailEpisodesFailed(err.to_string()))
                }
            }
        });
    }

    fn load_detail_related(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(item_id) = self.selected_item.as_ref().and_then(|item| item.id.clone()) else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.similar_items(&session.user_id, &item_id, 30).await {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::DetailRelatedLoaded(result.items.unwrap_or_default()),
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::DetailRelatedFailed(err.to_string()))
                }
            }
        });
    }

    fn load_detail_tech(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(item_id) = self.selected_item.as_ref().and_then(|item| item.id.clone()) else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            let request = PlaybackInfoRequest {
                user_id: Some(session.user_id.clone()),
                start_time_ticks: None,
                audio_stream_index: None,
                subtitle_stream_index: None,
                media_source_id: None,
                enable_direct_play: Some(true),
                enable_direct_stream: Some(true),
                enable_transcoding: Some(true),
                max_streaming_bitrate: None,
            };

            match client.playback_info(&item_id, &request).await {
                Ok(response) => {
                    let media = response
                        .media_sources
                        .as_ref()
                        .and_then(|sources| sources.first())
                        .cloned();
                    Self::push_message(&messages, UiMessage::DetailTechLoaded(media));
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::DetailTechFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn choose_playlist(&mut self, playlist_id: String) {
        self.selected_playlist_id = Some(playlist_id.clone());
        self.playlist_items.clear();
        self.load_playlist_items(playlist_id);
    }

    pub(super) fn load_playlist_items(&mut self, playlist_id: String) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client
                .playlist_items(&playlist_id, &session.user_id, 0, 500)
                .await
            {
                Ok(result) => Self::push_message(
                    &messages,
                    UiMessage::PlaylistItemsLoaded {
                        playlist_id,
                        items: result.items.unwrap_or_default(),
                    },
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::PlaylistItemsFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn load_last_played(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.last_played_item(&session.user_id).await {
                Ok(item) => Self::push_message(&messages, UiMessage::LastPlayedLoaded(item)),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::LastPlayedFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn request_thumbnail(
        &mut self,
        item_id: String,
        width: u32,
        height: u32,
        image_type: String,
        tag: Option<String>,
    ) {
        let Some(client) = self.active_client() else {
            return;
        };

        let key = Self::thumbnail_key(&item_id, width, height, &image_type, tag.as_deref());
        if self.thumbnail_textures.contains_key(&key)
            || self.thumbnail_pending.contains(&key)
            || self.thumbnail_failed.contains(&key)
        {
            return;
        }

        self.thumbnail_pending.insert(key.clone());
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            let url = match client.build_item_image_url(
                &item_id,
                &image_type,
                width,
                height,
                tag.as_deref(),
            ) {
                Ok(url) => url,
                Err(err) => {
                    Self::push_message(
                        &messages,
                        UiMessage::ThumbnailFailed {
                            key,
                            reason: err.to_string(),
                        },
                    );
                    return;
                }
            };

            match client.fetch_image_bytes(&url).await {
                Ok(bytes) => {
                    Self::push_message(&messages, UiMessage::ThumbnailLoaded { key, bytes });
                }
                Err(err) => {
                    Self::push_message(
                        &messages,
                        UiMessage::ThumbnailFailed {
                            key,
                            reason: err.to_string(),
                        },
                    );
                }
            }
        });
    }

    pub(super) fn do_login(&mut self) {
        let username = self.config.server.username.trim().to_string();
        let password = self.login_password.clone();

        if self.config.server.base_url.trim().is_empty()
            || username.is_empty()
            || password.is_empty()
        {
            self.status_line = "Set server URL, username, and password".to_string();
            return;
        }

        let client = match self.build_client() {
            Ok(client) => client,
            Err(err) => {
                self.status_line = format!("Config error: {err}");
                return;
            }
        };

        let messages = self.messages.clone();
        self.status_line = "Logging in...".to_string();

        self.runtime.spawn(async move {
            match client.authenticate_by_name(&username, &password).await {
                Ok(auth) => {
                    let token = auth.access_token.unwrap_or_default();
                    let user = auth.user;
                    let user_id = user.as_ref().and_then(|u| u.id.clone()).unwrap_or_default();
                    let user_name = user
                        .as_ref()
                        .and_then(|u| u.name.clone())
                        .unwrap_or_else(|| username.clone());
                    let is_admin = user
                        .as_ref()
                        .and_then(|u| u.policy.as_ref())
                        .and_then(|p| p.is_administrator)
                        .unwrap_or(false);

                    if token.is_empty() || user_id.is_empty() {
                        Self::push_message(
                            &messages,
                            UiMessage::LoginFailed(
                                "Jellyfin returned incomplete login payload".to_string(),
                            ),
                        );
                        return;
                    }

                    Self::push_message(
                        &messages,
                        UiMessage::LoggedIn {
                            token,
                            user_id,
                            user_name,
                            is_admin,
                            server_id: auth.server_id,
                        },
                    );
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::LoginFailed(err.to_string()));
                }
            }
        });
    }

    pub(super) fn refresh_health(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };

        self.status_line = "Refreshing server health...".to_string();
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            let ping = client.ping().await;
            let info = client.public_info().await;

            match (ping, info) {
                (Ok(ping), Ok(info)) => {
                    let _ = (ping, info);
                    Self::push_message(&messages, UiMessage::HealthResult);
                }
                (Err(err), _) | (_, Err(err)) => {
                    Self::push_message(&messages, UiMessage::HealthFailed(err.to_string()));
                }
            }
        });
    }

    pub(super) fn load_views(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        self.status_line = "Loading views...".to_string();
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            match client.user_views(&session.user_id).await {
                Ok(result) => Self::push_message(&messages, UiMessage::ViewsLoaded(result)),
                Err(err) => Self::push_message(&messages, UiMessage::SearchFailed(err.to_string())),
            }
        });
    }

    pub(super) fn search_items(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let term = self.search_term.trim().to_string();
        let selected_view = self.selected_view_id.clone();
        let hint_term = term.clone();
        let hint_messages = self.messages.clone();
        let hint_client = client.clone();
        let hint_user_id = session.user_id.clone();

        self.status_line = "Searching...".to_string();
        let messages = self.messages.clone();

        if !hint_term.is_empty() {
            self.runtime.spawn(async move {
                match hint_client.search_hints(&hint_user_id, &hint_term, 8).await {
                    Ok(result) => Self::push_message(
                        &hint_messages,
                        UiMessage::SearchHintsLoaded(result.search_hints.unwrap_or_default()),
                    ),
                    Err(err) => {
                        Self::push_message(&hint_messages, UiMessage::SearchFailed(err.to_string()))
                    }
                }
            });
        } else {
            self.search_hints.clear();
        }

        self.runtime.spawn(async move {
            let include_types = [
                "Movie",
                "Series",
                "Episode",
                "Audio",
                "AudioBook",
                "MusicAlbum",
                "Program",
                "BoxSet",
            ];

            match client
                .items(
                    &session.user_id,
                    selected_view.as_deref(),
                    if term.is_empty() { None } else { Some(&term) },
                    &include_types,
                    0,
                    100,
                )
                .await
            {
                Ok(result) => Self::push_message(&messages, UiMessage::SearchLoaded(result)),
                Err(err) => Self::push_message(&messages, UiMessage::SearchFailed(err.to_string())),
            }
        });
    }

    pub(super) fn mark_selected_item_played(&mut self) {
        let Some(item) = self.selected_item.clone() else {
            self.status_line = "Select an item first".to_string();
            return;
        };
        let Some(item_id) = item.id else {
            self.status_line = "Selected item has no id".to_string();
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(client) = self.active_client() else {
            return;
        };

        self.status_line = "Marking as played...".to_string();
        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.mark_played(&session.user_id, &item_id).await {
                Ok(()) => Self::push_message(&messages, UiMessage::MarkPlayedDone { item_id }),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::MarkPlayedFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn mark_selected_item_unplayed(&mut self) {
        let Some(item) = self.selected_item.clone() else {
            self.status_line = "Select an item first".to_string();
            return;
        };
        let Some(item_id) = item.id else {
            self.status_line = "Selected item has no id".to_string();
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(client) = self.active_client() else {
            return;
        };

        self.status_line = "Marking as unplayed...".to_string();
        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.mark_unplayed(&session.user_id, &item_id).await {
                Ok(()) => Self::push_message(&messages, UiMessage::MarkUnplayedDone { item_id }),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::MarkUnplayedFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn shuffle_play_selected_context(&mut self) {
        if let Some(item) = self.pick_shuffle_local_item() {
            self.selected_item = Some(item);
            self.start_playback();
            return;
        }

        let Some(client) = self.active_client() else {
            self.status_line = "No active session".to_string();
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let include_types = self
            .selected_item
            .as_ref()
            .and_then(|item| item.r#type.clone())
            .map(|item_type| vec![item_type])
            .unwrap_or_else(|| {
                vec![
                    "Movie".to_string(),
                    "Series".to_string(),
                    "Episode".to_string(),
                ]
            });
        let include_types_owned = include_types.clone();

        self.status_line = "Selecting a random item...".to_string();
        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            let include_types_refs = include_types_owned
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            match client
                .random_item_by_types(&session.user_id, &include_types_refs, 1)
                .await
            {
                Ok(result) => {
                    if let Some(item) = result.items.and_then(|items| items.into_iter().next()) {
                        Self::push_message(&messages, UiMessage::ShuffleItemReady(item));
                    } else {
                        Self::push_message(
                            &messages,
                            UiMessage::ShuffleItemFailed(
                                "No item available for shuffle".to_string(),
                            ),
                        );
                    }
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::ShuffleItemFailed(err.to_string()))
                }
            }
        });
    }

    fn pick_shuffle_local_item(&self) -> Option<BaseItemDto> {
        let pick_from = |items: &[BaseItemDto]| -> Option<BaseItemDto> {
            if items.is_empty() {
                return None;
            }
            let idx = SlimJellyApp::pseudo_random_index(items.len());
            items.get(idx).cloned()
        };

        let selected_type = self
            .selected_item
            .as_ref()
            .and_then(|item| item.r#type.as_deref())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        if selected_type == "series" {
            if let Some(item) = pick_from(&self.detail_episodes) {
                return Some(item);
            }
        }

        if let Some(item) = pick_from(&self.playlist_items) {
            return Some(item);
        }
        if let Some(item) = pick_from(&self.detail_related) {
            return Some(item);
        }

        match self.current_screen {
            Screen::Home => pick_from(&self.home_recent_movies)
                .or_else(|| pick_from(&self.home_recent_series))
                .or_else(|| pick_from(&self.home_continue_watching)),
            Screen::Libraries => pick_from(&self.library_items),
            Screen::Search => pick_from(&self.items),
            Screen::Collections => pick_from(&self.collection_items),
            Screen::Playlists => pick_from(&self.playlist_items),
            Screen::Details => {
                pick_from(&self.detail_related).or_else(|| pick_from(&self.detail_episodes))
            }
            Screen::Admin | Screen::Settings | Screen::Login | Screen::Requests => None,
        }
    }

    pub(super) fn add_selected_item_to_playlist(&mut self, playlist_id: String) {
        let Some(item) = self.selected_item.clone() else {
            self.status_line = "Select an item first".to_string();
            return;
        };
        let Some(item_id) = item.id else {
            self.status_line = "Selected item has no id".to_string();
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(client) = self.active_client() else {
            return;
        };

        let messages = self.messages.clone();
        self.status_line = "Adding to playlist...".to_string();
        self.runtime.spawn(async move {
            match client
                .add_items_to_playlist(&playlist_id, &session.user_id, &[&item_id])
                .await
            {
                Ok(()) => Self::push_message(
                    &messages,
                    UiMessage::PlaylistAddDone {
                        playlist_id,
                        item_id,
                    },
                ),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::PlaylistAddFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn load_virtual_folders(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        if !session.is_admin {
            return;
        }

        let Some(client) = self.active_client() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.virtual_folders().await {
                Ok(folders) => {
                    Self::push_message(&messages, UiMessage::VirtualFoldersLoaded(folders))
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::VirtualFoldersFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn delete_admin_item_by_id(&mut self, item_id: String) {
        let Some(session) = &self.session else {
            return;
        };
        if !session.is_admin {
            self.status_line = "Admin privileges required".to_string();
            return;
        }

        if item_id.trim().is_empty() {
            self.status_line = "Enter an item id".to_string();
            return;
        }

        let Some(client) = self.active_client() else {
            return;
        };
        let messages = self.messages.clone();
        self.status_line = "Deleting item...".to_string();
        self.runtime.spawn(async move {
            match client.delete_item(&item_id).await {
                Ok(()) => Self::push_message(&messages, UiMessage::DeleteItemDone { item_id }),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::DeleteItemFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn delete_admin_virtual_folder(&mut self, name: String) {
        let Some(session) = &self.session else {
            return;
        };
        if !session.is_admin {
            self.status_line = "Admin privileges required".to_string();
            return;
        }

        if name.trim().is_empty() {
            self.status_line = "Select a library".to_string();
            return;
        }

        let Some(client) = self.active_client() else {
            return;
        };
        let messages = self.messages.clone();
        self.status_line = "Deleting library...".to_string();
        self.runtime.spawn(async move {
            match client.remove_virtual_folder(&name, true).await {
                Ok(()) => Self::push_message(&messages, UiMessage::DeleteLibraryDone { name }),
                Err(err) => {
                    Self::push_message(&messages, UiMessage::DeleteLibraryFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn refresh_item_by_id(&mut self, item_id: String) {
        if item_id.trim().is_empty() {
            self.status_line = "Enter a library/item id".to_string();
            return;
        }

        let Some(client) = self.active_client() else {
            return;
        };
        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.item_refresh(&item_id).await {
                Ok(()) => Self::push_message(
                    &messages,
                    UiMessage::ActionDone("Refresh triggered".to_string()),
                ),
                Err(err) => Self::push_message(&messages, UiMessage::ActionFailed(err.to_string())),
            }
        });
    }

    pub(super) fn load_item_detail(&mut self, item_id: String) {
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.item(&session.user_id, &item_id).await {
                Ok(item) => Self::push_message(&messages, UiMessage::ItemLoaded(item)),
                Err(err) => Self::push_message(&messages, UiMessage::ItemFailed(err.to_string())),
            }
        });
    }

    pub(super) fn start_playback(&mut self) {
        let Some(item) = self.selected_item.clone() else {
            self.status_line = "Select an item first".to_string();
            return;
        };
        let Some(item_id) = item.id.clone() else {
            self.status_line = "Selected item has no id".to_string();
            return;
        };
        let Some(client) = self.active_client() else {
            return;
        };
        let Some(session) = self.session.clone() else {
            return;
        };

        let start_ticks = item
            .user_data
            .as_ref()
            .and_then(|u| u.playback_position_ticks)
            .filter(|ticks| *ticks > 0);

        let request = PlaybackInfoRequest {
            user_id: Some(session.user_id.clone()),
            start_time_ticks: start_ticks,
            audio_stream_index: None,
            subtitle_stream_index: None,
            media_source_id: None,
            enable_direct_play: Some(true),
            enable_direct_stream: Some(true),
            enable_transcoding: Some(true),
            max_streaming_bitrate: None,
        };

        let messages = self.messages.clone();
        let direct_first = self.config.playback.direct_first;
        let fallback_once = self.config.playback.fallback_once;
        self.status_line = "Preparing playback...".to_string();

        self.runtime.spawn(async move {
            match client.playback_info(&item_id, &request).await {
                Ok(response) => {
                    let media = response
                        .media_sources
                        .as_ref()
                        .and_then(|arr| arr.first())
                        .cloned();
                    let supports_direct = media
                        .as_ref()
                        .and_then(|m| m.supports_direct_play)
                        .unwrap_or(true);
                    let media_source_id = media.as_ref().and_then(|m| m.id.clone());
                    let use_transcode = !direct_first || !supports_direct;

                    let stream_url = match client.build_video_stream_url(
                        &item_id,
                        media_source_id.as_deref(),
                        response.play_session_id.as_deref(),
                        None,
                        None,
                        use_transcode,
                    ) {
                        Ok(url) => url,
                        Err(err) => {
                            Self::push_message(
                                &messages,
                                UiMessage::PlaybackPrepareFailed(err.to_string()),
                            );
                            return;
                        }
                    };

                    let transcode_stream_url = if fallback_once && !use_transcode {
                        client
                            .build_video_stream_url(
                                &item_id,
                                media_source_id.as_deref(),
                                response.play_session_id.as_deref(),
                                None,
                                None,
                                true,
                            )
                            .ok()
                    } else {
                        None
                    };

                    Self::push_message(
                        &messages,
                        UiMessage::PlaybackPrepared {
                            item_id,
                            run_time_ticks: item.run_time_ticks,
                            stream_url,
                            transcode_stream_url,
                            used_transcode: use_transcode,
                            media_source_id,
                            play_session_id: response.play_session_id,
                        },
                    );
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::PlaybackPrepareFailed(err.to_string()))
                }
            }
        });
    }

    pub(super) fn resolve_player_candidates(&self) -> Vec<PlayerKind> {
        match self.config.player.preferred {
            PreferredPlayer::Mpv => vec![PlayerKind::Mpv, PlayerKind::Vlc],
            PreferredPlayer::Vlc => vec![PlayerKind::Vlc, PlayerKind::Mpv],
        }
    }

    pub(super) fn player_command_for(
        &self,
        player_kind: PlayerKind,
        mpv_socket_path: Option<&str>,
    ) -> (String, Vec<String>) {
        match player_kind {
            PlayerKind::Mpv => {
                let mpv = self
                    .config
                    .player
                    .mpv_path
                    .clone()
                    .unwrap_or_else(|| "mpv".to_string());

                let mut args = vec!["--force-window=yes".to_string()];
                if let Some(socket_path) = mpv_socket_path {
                    args.push(format!("--input-ipc-server={socket_path}"));
                }
                (mpv, args)
            }
            PlayerKind::Vlc => {
                let vlc = self
                    .config
                    .player
                    .vlc_path
                    .clone()
                    .unwrap_or_else(|| "vlc".to_string());
                (vlc, Vec::new())
            }
        }
    }

    pub(super) fn launch_external_player(
        &mut self,
        item_id: String,
        run_time_ticks: Option<i64>,
        stream_url: String,
        media_source_id: Option<String>,
        play_session_id: Option<String>,
        used_transcode: bool,
        transcode_stream_url: Option<String>,
    ) {
        let mut last_error: Option<String> = None;
        let mut launched: Option<(PlayerKind, Option<String>, std::process::Child)> = None;

        let candidates = self.resolve_player_candidates();
        if candidates.is_empty() {
            self.status_line = "No supported player found (mpv/VLC)".to_string();
            return;
        }

        for player_kind in candidates {
            let mpv_socket_path = match player_kind {
                PlayerKind::Mpv => {
                    let path = format!("{}-{}", super::MPV_IPC_SOCKET_PATH, std::process::id());
                    if std::path::Path::new(&path).exists() {
                        let _ = std::fs::remove_file(&path);
                    }
                    Some(path)
                }
                PlayerKind::Vlc => None,
            };

            let (bin, args_prefix) =
                self.player_command_for(player_kind, mpv_socket_path.as_deref());
            let mut cmd = std::process::Command::new(&bin);
            for arg in args_prefix {
                cmd.arg(arg);
            }

            if let Some(sub_path) = &self.subtitle_temp_path {
                match player_kind {
                    PlayerKind::Mpv => {
                        cmd.arg(format!("--sub-file={}", sub_path));
                    }
                    PlayerKind::Vlc => {
                        cmd.arg("--sub-file");
                        cmd.arg(sub_path);
                    }
                }
            }

            cmd.arg(&stream_url);

            match cmd.spawn() {
                Ok(child) => {
                    launched = Some((player_kind, mpv_socket_path, child));
                    break;
                }
                Err(err) => {
                    if let Some(path) = mpv_socket_path.as_deref() {
                        let _ = std::fs::remove_file(path);
                    }
                    last_error = Some(format!("{bin}: {err}"));
                }
            }
        }

        let Some((player_kind, mpv_socket_path, child)) = launched else {
            if !used_transcode {
                if let Some(retry_url) = transcode_stream_url {
                    self.status_line =
                        "Direct launch failed, retrying with transcode...".to_string();
                    self.launch_external_player(
                        item_id,
                        run_time_ticks,
                        retry_url,
                        media_source_id,
                        play_session_id,
                        true,
                        None,
                    );
                    return;
                }
            }

            self.status_line = format!(
                "Failed to launch external player: {}",
                last_error.unwrap_or_else(|| "unknown error".to_string())
            );
            return;
        };

        let generation = self
            .progress_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .saturating_add(1);

        self.status_line = if used_transcode {
            format!(
                "Playback launched with {} (transcode fallback)",
                Self::describe_player_kind(player_kind)
            )
        } else {
            format!(
                "Playback launched with {}",
                Self::describe_player_kind(player_kind)
            )
        };

        let play_session_id_for_state = play_session_id.clone();
        let mpv_socket_for_loop = mpv_socket_path.clone();
        self.playback = Some(super::PlaybackView {
            generation,
            item_id: item_id.clone(),
            run_time_ticks,
            player_kind,
            mpv_socket_path: mpv_socket_path.clone(),
            used_transcode,
            transcode_stream_url: transcode_stream_url.clone(),
            media_source_id: media_source_id.clone(),
            play_session_id,
            position_ticks: 0,
            is_paused: false,
            status_text: if used_transcode {
                "Playing (transcode)".to_string()
            } else {
                "Playing".to_string()
            },
        });

        self.report_play_start(
            item_id.clone(),
            media_source_id.clone(),
            play_session_id_for_state,
        );
        self.spawn_progress_loop(
            item_id,
            media_source_id,
            self.playback
                .as_ref()
                .and_then(|p| p.play_session_id.clone()),
            player_kind,
            mpv_socket_for_loop,
            generation,
        );
        run_player_exit_watcher(self.messages.clone(), child, generation);
    }

    pub(super) fn report_play_start(
        &self,
        item_id: String,
        media_source_id: Option<String>,
        play_session_id: Option<String>,
    ) {
        let Some(client) = self.active_client() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            let payload = PlaybackStartInfo {
                item_id,
                play_session_id,
                can_seek: Some(true),
                is_paused: Some(false),
                position_ticks: Some(0),
                media_source_id,
            };

            if let Err(err) = client.report_playing_start(&payload).await {
                Self::push_message(&messages, UiMessage::ProgressFailed(err.to_string()));
            }
        });
    }

    pub(super) fn report_stop_for_playback(
        &self,
        playback: super::PlaybackView,
        position_ticks: i64,
    ) {
        let Some(client) = self.active_client() else {
            return;
        };
        let user_id = self.session.as_ref().map(|session| session.user_id.clone());
        let should_mark_played = SlimJellyApp::should_mark_played_on_stop(&playback, position_ticks);
        let item_id = playback.item_id;
        let play_session_id = playback.play_session_id;
        let media_source_id = playback.media_source_id;

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            let payload = PlaybackStopInfo {
                item_id: item_id.clone(),
                play_session_id,
                position_ticks: Some(position_ticks),
                media_source_id,
            };

            if let Err(err) = client.report_playing_stopped(&payload).await {
                Self::push_message(&messages, UiMessage::ProgressFailed(err.to_string()));
            }

            if should_mark_played {
                if let Some(user_id) = user_id.as_deref() {
                    if let Err(err) = client.mark_played(user_id, &item_id).await {
                        Self::push_message(&messages, UiMessage::ProgressFailed(err.to_string()));
                    }
                }
            }

            Self::push_message(&messages, UiMessage::PlaybackStopped { item_id });
        });
    }

    pub(super) fn status_text_for_playback(playback: &super::PlaybackView) -> String {
        if playback.is_paused {
            if playback.used_transcode {
                "Paused (transcode)".to_string()
            } else {
                "Paused".to_string()
            }
        } else if playback.used_transcode {
            "Playing (transcode)".to_string()
        } else {
            "Playing".to_string()
        }
    }

    pub(super) fn spawn_progress_loop(
        &self,
        item_id: String,
        media_source_id: Option<String>,
        play_session_id: Option<String>,
        player_kind: PlayerKind,
        mpv_socket_path: Option<String>,
        generation: u64,
    ) {
        let Some(client) = self.active_client() else {
            return;
        };

        let messages = self.messages.clone();
        let base_interval = self.config.playback.base_sync_interval_seconds.max(5);
        let progress_generation = self.progress_generation.clone();

        self.runtime.spawn(async move {
            let mut estimated_secs = 0_f64;
            let mut last_known_secs = 0_f64;
            let mut stagnant_count = 0_u8;
            let max_interval = 60_u64;

            loop {
                if progress_generation.load(std::sync::atomic::Ordering::SeqCst) != generation {
                    break;
                }

                let snapshot = match (player_kind, mpv_socket_path.as_deref()) {
                    (PlayerKind::Mpv, Some(path)) => read_mpv_snapshot(path).await,
                    _ => None,
                };

                let (position_secs, is_paused, used_mpv_snapshot) = if let Some(snap) = snapshot {
                    if (snap.position_secs - last_known_secs).abs() < 0.2 {
                        stagnant_count = stagnant_count.saturating_add(1);
                    } else {
                        stagnant_count = 0;
                    }
                    last_known_secs = snap.position_secs.max(last_known_secs);
                    (last_known_secs, snap.is_paused, true)
                } else {
                    let inferred_paused = stagnant_count >= 3;
                    if !inferred_paused {
                        estimated_secs += base_interval as f64;
                    }
                    (estimated_secs, inferred_paused, false)
                };

                let ticks = (position_secs.max(0.0) * 10_000_000.0) as i64;
                let progress = crate::jellyfin::models::PlaybackProgressInfo {
                    item_id: item_id.clone(),
                    play_session_id: play_session_id.clone(),
                    position_ticks: Some(ticks),
                    is_paused: Some(is_paused),
                    media_source_id: media_source_id.clone(),
                };

                if let Err(err) = client.report_playing_progress(&progress).await {
                    Self::push_message(&messages, UiMessage::ProgressFailed(err.to_string()));
                    break;
                }

                if let Some(session_id) = play_session_id.as_deref() {
                    if let Err(err) = client.report_playing_ping(session_id).await {
                        Self::push_message(&messages, UiMessage::ProgressFailed(err.to_string()));
                        break;
                    }
                }

                Self::push_message(
                    &messages,
                    UiMessage::ProgressTick {
                        position_ticks: ticks,
                        is_paused,
                    },
                );

                let sleep_seconds = if used_mpv_snapshot {
                    if stagnant_count >= 3 {
                        (base_interval.saturating_mul(2)).min(max_interval)
                    } else {
                        base_interval
                    }
                } else {
                    (base_interval.saturating_mul(2)).min(max_interval)
                };

                tokio::time::sleep(std::time::Duration::from_secs(sleep_seconds)).await;
            }
        });
    }

    pub(super) fn stop_playback(&mut self) {
        self.progress_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let Some(playback) = self.playback.take() else {
            return;
        };

        if let Some(path) = playback.mpv_socket_path.as_deref() {
            let _ = std::fs::remove_file(path);
        }

        self.status_line = "Stopping playback...".to_string();
        self.report_stop_for_playback(playback.clone(), playback.position_ticks);
    }

    pub(super) fn refresh_tasks(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        if !session.is_admin {
            return;
        }

        let Some(client) = self.active_client() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.scheduled_tasks().await {
                Ok(tasks) => Self::push_message(&messages, UiMessage::TasksLoaded(tasks)),
                Err(err) => Self::push_message(&messages, UiMessage::TasksFailed(err.to_string())),
            }
        });

        self.load_virtual_folders();
    }

    pub(super) fn trigger_scan_all(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };
        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.library_refresh_all().await {
                Ok(()) => Self::push_message(
                    &messages,
                    UiMessage::ActionDone("Library scan started".to_string()),
                ),
                Err(err) => Self::push_message(&messages, UiMessage::ActionFailed(err.to_string())),
            }
        });
    }

    pub(super) fn trigger_refresh_item(&mut self) {
        let item_id = self.selected_library_id.trim().to_string();
        self.refresh_item_by_id(item_id);
    }

    pub(super) fn save_settings(&mut self) {
        let username = self.config.subtitles.username.trim().to_string();
        let password = self.config.subtitles.password.clone();
        let api_key = self.config.subtitles.api_key.trim().to_string();

        match save_config(&self.paths, &self.config) {
            Ok(()) => self.status_line = "Settings saved. Validating OpenSubtitles credentials...".to_string(),
            Err(err) => self.status_line = format!("Failed to save settings: {err}"),
        }

        if !username.is_empty() && !password.is_empty() && !api_key.is_empty() {
            let messages = self.messages.clone();
            self.runtime.spawn(async move {
                match crate::subtitles::OpenSubtitlesClient::new(&api_key) {
                    Ok(client) => match client.login(&username, &password).await {
                        Ok(_) => Self::push_message(
                            &messages,
                            UiMessage::ActionDone("OpenSubtitles OpenSubtitles credentials are correct.".to_string()),
                        ),
                        Err(err) => Self::push_message(
                            &messages,
                            UiMessage::ActionFailed(format!("OpenSubtitles login failed: {err}")),
                        ),
                    },
                    Err(e) => Self::push_message(
                        &messages,
                        UiMessage::ActionFailed(format!("OpenSubtitles config error: {e}")),
                    ),
                }
            });
        }
    }

    pub(super) fn do_logout(&mut self) {
        self.progress_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if let Some(playback) = self.playback.as_ref() {
            if let Some(path) = playback.mpv_socket_path.as_deref() {
                let _ = std::fs::remove_file(path);
            }
        }

        if let Err(err) = clear_session(&self.paths.session_file) {
            self.status_line = format!("Failed to clear local session: {err}");
        }

        self.client = Arc::new(Mutex::new(None));
        self.session = None;
        self.current_screen = Screen::Login;
        self.detail_return_screen = Screen::Home;
        self.items.clear();
        self.library_items.clear();
        self.collection_items.clear();
        self.home_continue_watching.clear();
        self.home_recent_movies.clear();
        self.home_recent_series.clear();
        self.detail_seasons.clear();
        self.detail_selected_season_id = None;
        self.detail_preferred_season_id = None;
        self.detail_pending_next_season_id = None;
        self.detail_episodes.clear();
        self.detail_related.clear();
        self.detail_media_source = None;
        self.views.clear();
        self.selected_item = None;
        self.playback = None;
        self.admin_virtual_folders.clear();
        self.admin_selected_virtual_folder_name = None;
        self.admin_delete_item_confirm.clear();
        self.admin_delete_library_confirm.clear();
        self.cleanup_subtitle_temp();
        self.subtitle_search_results.clear();
        self.subtitle_panel_open = false;
        self.subtitle_os_token = None;
        self.status_line = "Logged out".to_string();
    }

    pub(super) fn search_subtitles(&mut self) {
        let api_key = self.config.subtitles.api_key.trim().to_string();
        if api_key.is_empty() {
            self.status_line = "Set OpenSubtitles API key in Settings first".to_string();
            return;
        }

        let item = match &self.selected_item {
            Some(i) => i,
            _ => {
                self.status_line = "No item selected".to_string();
                return;
            }
        };

        let query = if item.r#type.as_deref() == Some("Episode") {
            let series_name = item.series_name.as_deref().unwrap_or("");
            let s = item.parent_index_number.unwrap_or(0);
            let e = item.index_number.unwrap_or(0);
            if !series_name.is_empty() && s > 0 && e > 0 {
                format!("{} S{:02}E{:02}", series_name, s, e)
            } else if !series_name.is_empty() {
                format!("{} {}", series_name, item.name.as_deref().unwrap_or(""))
            } else {
                item.name.clone().unwrap_or_default()
            }
        } else {
            item.name.clone().unwrap_or_default()
        };

        if query.is_empty() {
            self.status_line = "Selected item has no name to search".to_string();
            return;
        }

        let language = self.subtitle_search_language.trim().to_string();
        self.subtitle_search_loading = true;
        self.subtitle_search_results.clear();
        self.status_line = format!("Searching subtitles for \"{query}\"...");

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            let client = match crate::subtitles::OpenSubtitlesClient::new(&api_key) {
                Ok(c) => c,
                Err(err) => {
                    Self::push_message(&messages, UiMessage::SubtitleSearchFailed(err.to_string()));
                    return;
                }
            };

            match client.search(&query, &language).await {
                Ok(response) => {
                    let results = response.data.unwrap_or_default();
                    Self::push_message(&messages, UiMessage::SubtitleSearchResults(results));
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::SubtitleSearchFailed(err.to_string()));
                }
            }
        });
    }

    pub(super) fn download_subtitle(&mut self, file_id: i64, file_name: String) {
        let api_key = self.config.subtitles.api_key.trim().to_string();
        if api_key.is_empty() {
            self.status_line = "Set OpenSubtitles API key in Settings first".to_string();
            return;
        }

        let os_username = self.config.subtitles.username.trim().to_string();
        let os_password = self.config.subtitles.password.clone();
        let cached_token = self.subtitle_os_token.clone();

        if cached_token.is_none() && (os_username.is_empty() || os_password.is_empty()) {
            self.status_line =
                "Set OpenSubtitles username and password in Settings for downloads".to_string();
            return;
        }

        self.status_line = format!("Downloading subtitle: {file_name}...");
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            let client = match crate::subtitles::OpenSubtitlesClient::new(&api_key) {
                Ok(c) => c,
                Err(err) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SubtitleDownloadFailed(err.to_string()),
                    );
                    return;
                }
            };

            // Login if no cached token
            let token = if let Some(tok) = cached_token {
                tok
            } else {
                match client.login(&os_username, &os_password).await {
                    Ok(tok) => tok,
                    Err(err) => {
                        Self::push_message(
                            &messages,
                            UiMessage::SubtitleDownloadFailed(format!("OS login failed: {err}")),
                        );
                        return;
                    }
                }
            };

            // Request download link
            let download_resp = match client.download(file_id, &token).await {
                Ok(resp) => resp,
                Err(err) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SubtitleDownloadFailed(err.to_string()),
                    );
                    return;
                }
            };

            let Some(link) = download_resp.link else {
                Self::push_message(
                    &messages,
                    UiMessage::SubtitleDownloadFailed("No download link in response".to_string()),
                );
                return;
            };

            // Fetch the actual subtitle file
            let bytes = match client.fetch_subtitle_bytes(&link).await {
                Ok(b) => b,
                Err(err) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SubtitleDownloadFailed(err.to_string()),
                    );
                    return;
                }
            };

            // Save to temp directory
            let actual_name = download_resp
                .file_name
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| file_name.clone());
            let temp_path = std::env::temp_dir().join(format!("slimjelly-sub-{actual_name}"));

            if let Err(err) = std::fs::write(&temp_path, &bytes) {
                Self::push_message(
                    &messages,
                    UiMessage::SubtitleDownloadFailed(format!("Failed to write file: {err}")),
                );
                return;
            }

            let path_str = temp_path.to_string_lossy().to_string();
            log::info!("subtitle saved to temp: {path_str}");

            Self::push_message(
                &messages,
                UiMessage::SubtitleDownloaded {
                    file_name: actual_name,
                    path: path_str,
                },
            );
        });
    }

    pub(super) fn cleanup_subtitle_temp(&mut self) {
        if let Some(path) = self.subtitle_temp_path.take() {
            if let Err(err) = std::fs::remove_file(&path) {
                log::debug!("failed to remove temp subtitle {path}: {err}");
            } else {
                log::info!("cleaned up temp subtitle: {path}");
            }
        }
        self.subtitle_search_results.clear();
        self.subtitle_panel_open = false;
    }

    // -----------------------------------------------------------------------
    // Jellyseerr / Overseerr
    // -----------------------------------------------------------------------

    fn build_seerr_client(&self) -> Result<crate::seerr::SeerrClient, crate::error::AppError> {
        crate::seerr::SeerrClient::new(
            &self.config.seerr.base_url,
            &self.config.seerr.api_key,
        )
    }

    pub(super) fn seerr_search(&mut self) {
        let term = self.seerr_search_term.trim().to_string();
        if term.is_empty() {
            self.status_line = "Enter a search term".to_string();
            return;
        }

        let client = match self.build_seerr_client() {
            Ok(c) => c,
            Err(err) => {
                self.status_line = format!("Seerr config error: {err}");
                return;
            }
        };

        self.seerr_search_loading = true;
        self.status_line = "Searching Jellyseerr...".to_string();
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            match client.search(&term, 1).await {
                Ok(response) => {
                    Self::push_message(&messages, UiMessage::SeerrSearchLoaded(response.results));
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::SeerrSearchFailed(err.to_string()));
                }
            }
        });
    }

    pub(super) fn seerr_load_requests(&mut self) {
        let client = match self.build_seerr_client() {
            Ok(c) => c,
            Err(err) => {
                self.status_line = format!("Seerr config error: {err}");
                return;
            }
        };

        self.seerr_requests_loading = true;
        self.status_line = "Loading requests...".to_string();
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            match client.get_requests(1, 50).await {
                Ok(response) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SeerrRequestsLoaded(response.results),
                    );
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::SeerrRequestsFailed(err.to_string()));
                }
            }
        });
    }

    pub(super) fn seerr_request_movie(&mut self, tmdb_id: i64, title: String) {
        let client = match self.build_seerr_client() {
            Ok(c) => c,
            Err(err) => {
                self.status_line = format!("Seerr config error: {err}");
                return;
            }
        };

        self.status_line = format!("Requesting \"{title}\"...");
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            match client.request_movie(tmdb_id).await {
                Ok(_) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SeerrRequestCreated(format!("Requested: {title}")),
                    );
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::SeerrRequestFailed(err.to_string()));
                }
            }
        });
    }

    pub(super) fn seerr_request_tv(
        &mut self,
        tmdb_id: i64,
        title: String,
        seasons: Option<Vec<i32>>,
    ) {
        let client = match self.build_seerr_client() {
            Ok(c) => c,
            Err(err) => {
                self.status_line = format!("Seerr config error: {err}");
                return;
            }
        };

        self.status_line = format!("Requesting \"{title}\"...");
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            match client.request_tv(tmdb_id, seasons).await {
                Ok(_) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SeerrRequestCreated(format!("Requested: {title}")),
                    );
                }
                Err(err) => {
                    Self::push_message(&messages, UiMessage::SeerrRequestFailed(err.to_string()));
                }
            }
        });
    }
}
