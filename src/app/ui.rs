use std::{collections::HashSet, sync::atomic::Ordering};

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::{config::PreferredPlayer, jellyfin::models::BaseItemDto};

use super::{LibrarySection, Screen, SessionView, SlimJellyApp, UiMessage};

impl SlimJellyApp {
    fn refresh_media_views_after_item_state_change(
        &mut self,
        detail_item_id: Option<String>,
        refresh_collections: bool,
        refresh_search: bool,
        refresh_playlists: bool,
    ) {
        if let Some(item_id) = detail_item_id {
            if self.current_screen == Screen::Details {
                self.load_item_detail(item_id);
            }
        }

        self.load_home_sections();
        self.load_last_played();

        if self.current_screen == Screen::Libraries {
            self.load_library_items(self.current_library_section);
        }
        if refresh_collections && self.current_screen == Screen::Collections {
            self.load_collections();
        }
        if refresh_search && self.current_screen == Screen::Search {
            self.search_items();
        }
        if refresh_playlists && self.current_screen == Screen::Playlists {
            if let Some(playlist_id) = self.selected_playlist_id.clone() {
                self.load_playlist_items(playlist_id);
            }
        }
    }

    fn color_bg() -> Color32 {
        Color32::from_rgb(11, 15, 20)
    }

    fn color_panel() -> Color32 {
        Color32::from_rgb(16, 22, 30)
    }

    fn color_surface() -> Color32 {
        Color32::from_rgb(22, 30, 40)
    }

    fn color_surface_alt() -> Color32 {
        Color32::from_rgb(26, 35, 46)
    }

    fn color_border() -> Color32 {
        Color32::from_rgb(52, 70, 88)
    }

    fn color_text_muted() -> Color32 {
        Color32::from_rgb(166, 181, 196)
    }

    fn color_accent() -> Color32 {
        Color32::from_rgb(221, 134, 76)
    }

    fn color_accent_soft() -> Color32 {
        Color32::from_rgb(79, 62, 49)
    }

    fn color_info() -> Color32 {
        Color32::from_rgb(92, 176, 202)
    }

    fn color_success() -> Color32 {
        Color32::from_rgb(112, 196, 149)
    }

    fn radius_s() -> egui::CornerRadius {
        egui::CornerRadius::same(8)
    }

    fn radius_m() -> egui::CornerRadius {
        egui::CornerRadius::same(10)
    }

    fn radius_l() -> egui::CornerRadius {
        egui::CornerRadius::same(12)
    }

    fn space_xs() -> f32 {
        4.0
    }

    fn space_s() -> f32 {
        6.0
    }

    fn space_m() -> f32 {
        8.0
    }

    fn space_l() -> f32 {
        12.0
    }

    fn content_max_width() -> f32 {
        1640.0
    }

    fn draw_centered_content<R>(
        ui: &mut egui::Ui,
        body: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let available_width = ui.available_width();
        let content_width = available_width.min(Self::content_max_width());
        let side_space = ((available_width - content_width) * 0.5).max(0.0);

        ui.horizontal(|ui| {
            if side_space > 0.0 {
                ui.add_space(side_space);
            }

            let inner = ui.vertical(|ui| {
                ui.set_min_width(content_width);
                ui.set_max_width(content_width);
                body(ui)
            });

            if side_space > 0.0 {
                ui.add_space(side_space);
            }

            inner.inner
        })
        .inner
    }

    fn section_frame(ui: &egui::Ui) -> egui::Frame {
        egui::Frame::group(ui.style())
            .fill(Self::color_surface())
            .stroke(Stroke::new(1.0, Self::color_border()))
            .corner_radius(Self::radius_l())
            .inner_margin(egui::Margin::symmetric(12, 10))
    }

    fn panel_frame(ui: &egui::Ui) -> egui::Frame {
        egui::Frame::group(ui.style())
            .fill(Self::color_panel())
            .stroke(Stroke::new(1.0, Self::color_border()))
            .corner_radius(Self::radius_l())
            .inner_margin(egui::Margin::symmetric(12, 10))
    }

    fn primary_button(label: impl Into<egui::WidgetText>) -> egui::Button<'static> {
        egui::Button::new(label)
            .fill(Self::color_accent_soft())
            .stroke(Stroke::new(1.0, Self::color_accent()))
            .corner_radius(Self::radius_s())
    }

    fn danger_button(label: impl Into<egui::WidgetText>) -> egui::Button<'static> {
        egui::Button::new(label)
            .fill(Color32::from_rgb(92, 44, 49))
            .stroke(Stroke::new(1.0, Color32::from_rgb(208, 100, 115)))
            .corner_radius(Self::radius_s())
    }

    fn muted_text(text: impl Into<String>) -> RichText {
        RichText::new(text.into()).color(Self::color_text_muted())
    }

    fn count_text(count: usize, singular: &str, plural: &str) -> String {
        let noun = if count == 1 { singular } else { plural };
        format!("{count} {noun}")
    }

    fn draw_image_placeholder(ui: &mut egui::Ui, size: Vec2, label: &str) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let rounding = Self::radius_m();

        let top_strip = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, rect.min.y + rect.height() * 0.42),
        );

        ui.painter().rect(
            rect,
            rounding,
            Self::color_surface(),
            Stroke::new(1.0, Self::color_border()),
            egui::StrokeKind::Outside,
        );
        ui.painter().rect_filled(
            top_strip,
            rounding,
            Color32::from_rgba_unmultiplied(36, 50, 65, 65),
        );

        ui.painter().text(
            rect.center() + egui::vec2(0.0, 8.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::TextStyle::Small.resolve(ui.style()),
            Self::color_text_muted(),
        );

        ui.painter().text(
            rect.center() + egui::vec2(0.0, -10.0),
            egui::Align2::CENTER_CENTER,
            "Artwork unavailable",
            egui::TextStyle::Small.resolve(ui.style()),
            Self::color_text_muted(),
        );
        response
    }

    pub(super) fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Self::color_bg();
        visuals.window_fill = Self::color_panel();
        visuals.faint_bg_color = Self::color_surface();
        visuals.extreme_bg_color = Color32::from_rgb(8, 11, 15);
        visuals.override_text_color = Some(Color32::from_rgb(229, 236, 242));

        visuals.selection.bg_fill = Self::color_info();
        visuals.selection.stroke = Stroke::new(1.0, Self::color_info());
        visuals.hyperlink_color = Self::color_info();

        visuals.widgets.noninteractive.bg_fill = Self::color_surface();
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::color_text_muted());
        visuals.widgets.inactive.bg_fill = Self::color_surface_alt();
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(214, 225, 236));
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(34, 45, 58);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_rgb(235, 241, 246));
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::color_info());
        visuals.widgets.active.bg_fill = Color32::from_rgb(43, 56, 71);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_rgb(243, 247, 251));
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Self::color_info());
        visuals.widgets.open.bg_fill = Self::color_surface_alt();

        visuals.window_stroke = Stroke::new(1.0, Self::color_border());
        visuals.popup_shadow.color = Color32::from_rgba_unmultiplied(6, 10, 16, 130);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(11.0, 9.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.window_margin = egui::Margin::symmetric(14, 12);
        style.spacing.indent = 18.0;
        style.spacing.interact_size = egui::vec2(40.0, 30.0);

        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(30.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(16.5, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(15.5, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(13.5, egui::FontFamily::Proportional),
        );

        ctx.set_style(style);
    }

    pub(super) fn handle_messages(&mut self) {
        for msg in self.take_messages() {
            match msg {
                UiMessage::LoggedIn {
                    token,
                    user_id,
                    user_name,
                    is_admin,
                    server_id,
                } => match self.build_client_with_token(token.clone()) {
                    Ok(client) => {
                        self.client = std::sync::Arc::new(std::sync::Mutex::new(Some(client)));
                        self.session = Some(SessionView {
                            user_id: user_id.clone(),
                            user_name,
                            is_admin,
                        });
                        self.current_screen = Screen::Home;
                        self.detail_return_screen = Screen::Home;
                        self.status_line = "Login successful".to_string();

                        let _ = crate::secure_store::store_session(
                            &self.paths.session_file,
                            &crate::secure_store::SessionToken {
                                access_token: token,
                                user_id,
                                server_id,
                            },
                        );

                        self.load_post_auth_data();
                    }
                    Err(err) => {
                        self.status_line = format!("Failed to build API client: {err}");
                    }
                },
                UiMessage::LoginFailed(message) => {
                    self.status_line = format!("Login failed: {message}");
                }
                UiMessage::HealthResult => {
                    self.status_line = "Server health updated".to_string();
                }
                UiMessage::HealthFailed(message) => {
                    self.status_line = format!("Health check failed: {message}");
                }
                UiMessage::SessionValidated {
                    user_name,
                    is_admin,
                } => {
                    if let Some(session) = self.session.as_mut() {
                        session.user_name = user_name;
                        session.is_admin = is_admin;
                    }
                    self.status_line = "Session validated".to_string();
                }
                UiMessage::SessionValidationFailed(message) => {
                    self.status_line = format!("Session validation failed: {message}");
                }
                UiMessage::ViewsLoaded(result) => {
                    self.views = result.items.unwrap_or_default();
                    self.status_line = format!(
                        "Loaded {}",
                        Self::count_text(self.views.len(), "library", "libraries")
                    );
                }
                UiMessage::SearchHintsLoaded(hints) => {
                    self.search_hints = hints;
                }
                UiMessage::SearchLoaded(result) => {
                    self.items = result.items.unwrap_or_default();
                    self.status_line = format!(
                        "Loaded {}",
                        Self::count_text(self.items.len(), "search item", "search items")
                    );
                }
                UiMessage::SearchFailed(message) => {
                    self.status_line = format!("Search failed: {message}");
                }
                UiMessage::HomeContinueWatchingLoaded(items) => {
                    self.home_continue_watching = items;
                }
                UiMessage::HomeRecentMoviesLoaded(items) => {
                    self.home_recent_movies = items;
                }
                UiMessage::HomeRecentSeriesLoaded(items) => {
                    self.home_recent_series = items;
                }
                UiMessage::HomeLoadFailed(message) => {
                    self.status_line = format!("Home row load failed: {message}");
                }
                UiMessage::LibraryItemsLoaded { section, items } => {
                    if section == self.current_library_section {
                        self.library_items = items;
                        self.status_line = format!(
                            "Loaded {}",
                            Self::count_text(
                                self.library_items.len(),
                                "library item",
                                "library items"
                            )
                        );
                    }
                }
                UiMessage::LibraryItemsFailed(message) => {
                    self.status_line = format!("Library load failed: {message}");
                }
                UiMessage::CollectionItemsLoaded(items) => {
                    self.collection_items = items;
                    self.status_line = format!(
                        "Loaded {}",
                        Self::count_text(self.collection_items.len(), "collection", "collections")
                    );
                }
                UiMessage::CollectionItemsFailed(message) => {
                    self.status_line = format!("Collection load failed: {message}");
                }
                UiMessage::VirtualFoldersLoaded(folders) => {
                    self.admin_virtual_folders = folders;
                    let selected_still_exists = self
                        .admin_selected_virtual_folder_name
                        .as_deref()
                        .map(|selected| {
                            self.admin_virtual_folders
                                .iter()
                                .any(|folder| folder.name.as_deref() == Some(selected))
                        })
                        .unwrap_or(false);

                    if !selected_still_exists {
                        self.admin_selected_virtual_folder_name = self
                            .admin_virtual_folders
                            .first()
                            .and_then(|folder| folder.name.clone());
                    }
                }
                UiMessage::VirtualFoldersFailed(message) => {
                    self.status_line = format!("Virtual folder load failed: {message}");
                }
                UiMessage::DetailSeasonsLoaded(seasons) => {
                    self.detail_seasons = seasons;

                    if self.detail_seasons.is_empty() {
                        self.detail_selected_season_id = None;
                        self.detail_preferred_season_id = None;
                        self.detail_pending_next_season_id = None;
                        self.detail_episodes.clear();
                        continue;
                    }

                    let target_id = self
                        .detail_pending_next_season_id
                        .take()
                        .or_else(|| self.detail_preferred_season_id.clone())
                        .or_else(|| self.detail_selected_season_id.clone());

                    if let Some(target_id) = target_id {
                        let exists = self
                            .detail_seasons
                            .iter()
                            .any(|season| season.id.as_deref() == Some(target_id.as_str()));
                        if exists {
                            let should_reload = self.detail_selected_season_id.as_deref()
                                != Some(target_id.as_str())
                                || self.detail_episodes.is_empty();
                            self.detail_selected_season_id = Some(target_id.clone());
                            self.detail_preferred_season_id = Some(target_id.clone());
                            if should_reload {
                                self.detail_episodes.clear();
                                self.load_detail_episodes(target_id);
                            }
                            continue;
                        }
                    }

                    if let Some(first_id) = self.detail_seasons.first().and_then(|s| s.id.clone()) {
                        self.choose_detail_season(first_id);
                    }
                }
                UiMessage::DetailSeasonsFailed(message) => {
                    self.status_line = format!("Seasons load failed: {message}");
                }
                UiMessage::DetailEpisodesLoaded { season_id, items } => {
                    if self.detail_selected_season_id.as_deref() == Some(season_id.as_str()) {
                        self.detail_preferred_season_id = Some(season_id.clone());
                        self.detail_episodes = items;

                        let should_open_first_next_episode = self
                            .detail_pending_next_season_id
                            .as_deref()
                            .map(|pending| pending == season_id.as_str())
                            .unwrap_or(false);

                        if should_open_first_next_episode {
                            self.detail_pending_next_season_id = None;

                            if let Some(first_episode) =
                                Self::sorted_episode_items(&self.detail_episodes)
                                    .into_iter()
                                    .next()
                            {
                                self.open_item_details(first_episode, self.detail_return_screen);
                            } else {
                                self.status_line =
                                    "No episodes found in the next season".to_string();
                            }
                        }
                    }
                }
                UiMessage::DetailEpisodesFailed(message) => {
                    self.status_line = format!("Episodes load failed: {message}");
                }
                UiMessage::DetailRelatedLoaded(items) => {
                    self.detail_related = items;
                }
                UiMessage::DetailRelatedFailed(message) => {
                    self.status_line = format!("Related items load failed: {message}");
                }
                UiMessage::DetailTechLoaded(media) => {
                    self.detail_media_source = media;
                }
                UiMessage::DetailTechFailed(message) => {
                    self.status_line = format!("Tech info load failed: {message}");
                }
                UiMessage::PlaylistsLoaded(playlists) => {
                    self.playlists = playlists;
                    if self.selected_playlist_id.is_none() {
                        self.selected_playlist_id =
                            self.playlists.first().and_then(|p| p.id.clone());
                        if let Some(first_id) = self.selected_playlist_id.clone() {
                            self.load_playlist_items(first_id);
                        }
                    }
                }
                UiMessage::PlaylistsFailed(message) => {
                    self.status_line = format!("Playlist load failed: {message}");
                }
                UiMessage::PlaylistItemsLoaded { playlist_id, items } => {
                    if self.selected_playlist_id.as_deref() == Some(playlist_id.as_str()) {
                        self.playlist_items = items;
                    }
                }
                UiMessage::PlaylistItemsFailed(message) => {
                    self.status_line = format!("Playlist items failed: {message}");
                }
                UiMessage::LastPlayedLoaded(item) => {
                    self.last_played_item = item;
                }
                UiMessage::LastPlayedFailed(message) => {
                    self.status_line = format!("Last played load failed: {message}");
                }
                UiMessage::ThumbnailLoaded { key, bytes } => {
                    self.thumbnail_pending.remove(&key);

                    if let Ok(image) = image::load_from_memory(&bytes) {
                        let rgba = image.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        self.thumbnail_images.insert(key.clone(), color_image);
                    } else {
                        self.thumbnail_failed.insert(key);
                    }
                }
                UiMessage::ThumbnailFailed { key, reason } => {
                    self.thumbnail_pending.remove(&key);
                    log::debug!("thumbnail load failed for {key}: {reason}");
                    self.thumbnail_failed.insert(key);
                }
                UiMessage::ItemLoaded(item) => {
                    self.selected_item = Some(item);
                    if self.current_screen == Screen::Details {
                        self.load_detail_sections();
                    }
                }
                UiMessage::ItemFailed(message) => {
                    self.status_line = format!("Failed to load item: {message}");
                }
                UiMessage::PlaybackPrepared {
                    item_id,
                    run_time_ticks,
                    stream_url,
                    transcode_stream_url,
                    used_transcode,
                    media_source_id,
                    play_session_id,
                } => {
                    self.launch_external_player(
                        item_id,
                        run_time_ticks,
                        stream_url,
                        media_source_id,
                        play_session_id,
                        used_transcode,
                        transcode_stream_url,
                    );
                }
                UiMessage::PlaybackPrepareFailed(message) => {
                    self.status_line = format!("Playback prepare failed: {message}");
                }
                UiMessage::PlayerExited {
                    generation,
                    elapsed_seconds,
                } => {
                    if let Some(playback) = self.playback.clone() {
                        if playback.generation == generation {
                            self.progress_generation.fetch_add(1, Ordering::SeqCst);

                            if let Some(path) = playback.mpv_socket_path.as_deref() {
                                let _ = std::fs::remove_file(path);
                            }

                            let fallback_url = if playback.used_transcode {
                                None
                            } else {
                                playback.transcode_stream_url.clone()
                            };

                            let fallback_allowed =
                                SlimJellyApp::should_retry_transcode(&playback, elapsed_seconds);

                            if fallback_allowed {
                                self.status_line =
                                    "Direct playback exited quickly; retrying transcode..."
                                        .to_string();
                                self.playback = None;

                                self.launch_external_player(
                                    playback.item_id.clone(),
                                    playback.run_time_ticks,
                                    fallback_url.unwrap_or_default(),
                                    playback.media_source_id.clone(),
                                    playback.play_session_id.clone(),
                                    true,
                                    None,
                                );
                            } else {
                                self.playback = None;
                                let stop_ticks =
                                    SlimJellyApp::stop_ticks_for_exit(&playback, elapsed_seconds);

                                self.cleanup_subtitle_temp();
                                self.status_line = "Playback ended".to_string();
                                self.report_stop_for_playback(playback, stop_ticks);
                            }
                        }
                    }
                }
                UiMessage::ProgressTick {
                    position_ticks,
                    is_paused,
                } => {
                    if let Some(playback) = self.playback.as_mut() {
                        playback.position_ticks = position_ticks;
                        playback.is_paused = is_paused;
                        playback.status_text = Self::status_text_for_playback(playback);
                    }
                }
                UiMessage::ProgressFailed(message) => {
                    self.status_line = format!("Progress sync error: {message}");
                }
                UiMessage::PlaybackStopped { item_id } => {
                    if self.playback.is_some() {
                        self.playback = None;
                        self.cleanup_subtitle_temp();
                    }
                    self.refresh_media_views_after_item_state_change(
                        Some(item_id),
                        true,
                        true,
                        true,
                    );
                }
                UiMessage::TasksLoaded(tasks) => {
                    self.tasks = tasks;
                    self.status_line = format!(
                        "Loaded {}",
                        Self::count_text(self.tasks.len(), "task", "tasks")
                    );
                }
                UiMessage::TasksFailed(message) => {
                    self.status_line = format!("Task load failed: {message}");
                }
                UiMessage::ActionDone(message) => {
                    self.status_line = message;
                    self.refresh_tasks();
                }
                UiMessage::ActionFailed(message) => {
                    self.status_line = format!("Action failed: {message}");
                }
                UiMessage::ShuffleItemReady(item) => {
                    self.selected_item = Some(item);
                    self.start_playback();
                }
                UiMessage::ShuffleItemFailed(message) => {
                    self.status_line = format!("Shuffle failed: {message}");
                }
                UiMessage::PlaylistAddDone {
                    playlist_id,
                    item_id,
                } => {
                    self.status_line = "Added item to playlist".to_string();
                    self.selected_playlist_id = Some(playlist_id.clone());
                    self.load_playlist_items(playlist_id);

                    if self.current_screen == Screen::Details {
                        self.load_item_detail(item_id);
                    }
                }
                UiMessage::PlaylistAddFailed(message) => {
                    self.status_line = format!("Add to playlist failed: {message}");
                }
                UiMessage::MarkPlayedDone { item_id } => {
                    self.status_line = "Marked played".to_string();
                    self.refresh_media_views_after_item_state_change(
                        Some(item_id),
                        false,
                        false,
                        false,
                    );
                }
                UiMessage::MarkPlayedFailed(message) => {
                    self.status_line = format!("Mark played failed: {message}");
                }
                UiMessage::MarkUnplayedDone { item_id } => {
                    self.status_line = "Marked unplayed".to_string();
                    self.refresh_media_views_after_item_state_change(
                        Some(item_id),
                        false,
                        false,
                        false,
                    );
                }
                UiMessage::MarkUnplayedFailed(message) => {
                    self.status_line = format!("Mark unplayed failed: {message}");
                }
                UiMessage::DeleteItemDone { item_id } => {
                    self.status_line = "Item deleted".to_string();
                    self.admin_delete_item_confirm.clear();
                    self.home_continue_watching
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.home_recent_movies
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.home_recent_series
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.library_items
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.collection_items
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.items
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.playlist_items
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.detail_related
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));
                    self.detail_episodes
                        .retain(|item| item.id.as_deref() != Some(item_id.as_str()));

                    if self
                        .selected_item
                        .as_ref()
                        .and_then(|item| item.id.as_ref())
                        == Some(&item_id)
                    {
                        self.selected_item = None;
                        self.current_screen = Screen::Admin;
                    }

                    self.load_home_sections();
                    self.load_library_items(self.current_library_section);
                    self.load_collections();
                    self.search_items();
                    self.load_playlists();
                    self.load_last_played();
                }
                UiMessage::DeleteItemFailed(message) => {
                    self.status_line = format!("Delete item failed: {message}");
                }
                UiMessage::DeleteLibraryDone { name } => {
                    self.status_line = format!("Library '{name}' deleted");
                    self.admin_delete_library_confirm.clear();
                    self.admin_virtual_folders
                        .retain(|folder| folder.name.as_deref() != Some(name.as_str()));
                    if self.admin_selected_virtual_folder_name.as_deref() == Some(name.as_str()) {
                        self.admin_selected_virtual_folder_name = self
                            .admin_virtual_folders
                            .first()
                            .and_then(|folder| folder.name.clone());
                    }

                    self.load_virtual_folders();
                    self.load_views();
                    self.load_library_items(self.current_library_section);
                    self.load_collections();
                    self.search_items();
                }
                UiMessage::DeleteLibraryFailed(message) => {
                    self.status_line = format!("Delete library failed: {message}");
                }
                UiMessage::SubtitleSearchResults(results) => {
                    self.subtitle_search_results = results;
                    self.subtitle_search_loading = false;
                    self.status_line = format!(
                        "Found {}",
                        Self::count_text(
                            self.subtitle_search_results.len(),
                            "subtitle result",
                            "subtitle results"
                        )
                    );
                }
                UiMessage::SubtitleSearchFailed(message) => {
                    self.subtitle_search_loading = false;
                    self.status_line = format!("Subtitle search failed: {message}");
                }
                UiMessage::SubtitleDownloaded { file_name, path } => {
                    self.subtitle_temp_path = Some(path);
                    self.status_line = format!("Subtitle downloaded: {file_name}");
                }
                UiMessage::SubtitleDownloadFailed(message) => {
                    self.status_line = format!("Subtitle download failed: {message}");
                }
            }
        }
    }

    pub(super) fn draw_app_shell(&mut self, ctx: &egui::Context) {
        debug_assert!(self.current_screen != Screen::Login);
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.current_screen != Screen::Login {
                Self::draw_centered_content(ui, |ui| self.draw_top_bar(ui));
                ui.add_space(8.0);
                Self::draw_centered_content(ui, |ui| self.draw_now_playing_strip(ui));
                ui.add_space(6.0);
            }

            egui::ScrollArea::vertical()
                .id_salt("main_screen_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    Self::draw_centered_content(ui, |ui| match self.current_screen {
                        Screen::Home => self.draw_home(ui),
                        Screen::Search => self.draw_search(ui),
                        Screen::Libraries => self.draw_libraries(ui),
                        Screen::Collections => self.draw_collections(ui),
                        Screen::Playlists => self.draw_playlists_screen(ui),
                        Screen::Admin => self.draw_admin(ui),
                        Screen::Settings => self.draw_settings(ui),
                        Screen::Details => self.draw_details(ui),
                        Screen::Login => unreachable!("Login screen is rendered in update()"),
                    });
                });
        });
    }

    fn top_nav_button(&mut self, ui: &mut egui::Ui, label: &str, screen: Screen) {
        let selected = self.current_screen == screen;
        let text = if selected {
            RichText::new(label).color(Color32::from_rgb(236, 241, 247)).strong()
        } else {
            Self::muted_text(label)
        };

        let button = egui::Button::new(text)
            .fill(if selected {
                Self::color_accent_soft()
            } else {
                Color32::TRANSPARENT
            })
            .stroke(Stroke::new(
                1.0,
                if selected {
                    Self::color_accent()
                } else {
                    Self::color_border()
                },
            ))
            .corner_radius(Self::radius_m())
            .min_size(egui::vec2(92.0, 32.0));
        if ui.add(button).clicked() {
            self.navigate_to(screen);
        }
    }

    fn draw_top_bar(&mut self, ui: &mut egui::Ui) {
        let compact = Self::is_compact_layout(ui);
        let user_label = self
            .session
            .as_ref()
            .map(|session| format!("User: {}", session.user_name))
            .unwrap_or_else(|| "Disconnected".to_string());

        Self::panel_frame(ui).show(ui, |ui| {
            if compact {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("slimjelly")
                                .strong()
                                .size(20.0)
                                .color(Color32::from_rgb(240, 245, 250)),
                        );
                        ui.separator();
                        ui.label(Self::muted_text(user_label.clone()).small());
                    });

                    ui.horizontal_wrapped(|ui| {
                        self.top_nav_button(ui, "Home", Screen::Home);
                        self.top_nav_button(ui, "Search", Screen::Search);
                        self.top_nav_button(ui, "Libraries", Screen::Libraries);
                        self.top_nav_button(ui, "Collections", Screen::Collections);
                        self.top_nav_button(ui, "Playlists", Screen::Playlists);
                        self.top_nav_button(ui, "Settings", Screen::Settings);
                        if self.session.as_ref().map(|s| s.is_admin).unwrap_or(false) {
                            self.top_nav_button(ui, "Admin", Screen::Admin);
                        }
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(Self::muted_text(self.status_line.clone()).small());
                        ui.separator();
                        if ui.button("Logout").clicked() {
                            self.do_logout();
                        }
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("slimjelly")
                            .strong()
                            .size(21.0)
                            .color(Color32::from_rgb(240, 245, 250)),
                    );
                    ui.separator();

                    self.top_nav_button(ui, "Home", Screen::Home);
                    self.top_nav_button(ui, "Search", Screen::Search);
                    self.top_nav_button(ui, "Libraries", Screen::Libraries);
                    self.top_nav_button(ui, "Collections", Screen::Collections);
                    self.top_nav_button(ui, "Playlists", Screen::Playlists);
                    self.top_nav_button(ui, "Settings", Screen::Settings);
                    if self.session.as_ref().map(|s| s.is_admin).unwrap_or(false) {
                        self.top_nav_button(ui, "Admin", Screen::Admin);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Logout").clicked() {
                            self.do_logout();
                        }

                        ui.label(Self::muted_text(user_label.clone()).small());

                        ui.separator();
                        ui.label(Self::muted_text(self.status_line.clone()).small());
                    });
                });
            }
        });
    }

    fn navigate_to(&mut self, screen: Screen) {
        self.current_screen = screen;
        match screen {
            Screen::Home => {
                if self.home_continue_watching.is_empty() && self.home_recent_movies.is_empty() {
                    self.load_home_sections();
                }
            }
            Screen::Search => {
                if self.items.is_empty() {
                    self.search_items();
                }
            }
            Screen::Libraries => {
                if self.library_items.is_empty() {
                    self.load_library_items(self.current_library_section);
                }
            }
            Screen::Collections => {
                if self.collection_items.is_empty() {
                    self.load_collections();
                }
            }
            Screen::Playlists => {
                if self.playlists.is_empty() {
                    self.load_playlists();
                }
            }
            Screen::Admin => {
                self.refresh_tasks();
            }
            Screen::Settings | Screen::Details | Screen::Login => {}
        }
    }

    pub(super) fn draw_login(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.1).max(16.0));

            let card_width = ui.available_width().min(620.0);
            Self::panel_frame(ui).show(ui, |ui| {
                    ui.set_width(card_width - 36.0);
                    ui.heading(RichText::new("Connect to Jellyfin").color(Color32::from_rgb(237, 243, 248)));
                    ui.label(
                        Self::muted_text("Clean cinematic desktop player for your media server"),
                    );
                    ui.add_space(Self::space_m());

                    ui.label(RichText::new("Server URL").small().strong());
                    let url_resp = ui.text_edit_singleline(&mut self.config.server.base_url);

                    ui.label(RichText::new("Username").small().strong());
                    let user_resp = ui.text_edit_singleline(&mut self.config.server.username);

                    ui.label(RichText::new("Password").small().strong());
                    let pass_resp =
                        ui.add(egui::TextEdit::singleline(&mut self.login_password).password(true));

                    ui.checkbox(
                        &mut self.config.server.allow_self_signed,
                        "Allow self-signed certificates",
                    );

                    ui.add_space(Self::space_m());
                    ui.horizontal(|ui| {
                        if ui.button("Save Settings").clicked() {
                            self.save_settings();
                        }
                        if ui
                            .add(Self::primary_button("Login"))
                            .clicked()
                        {
                            self.do_login();
                        }
                    });

                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let input_focused =
                        url_resp.has_focus() || user_resp.has_focus() || pass_resp.has_focus();
                    if enter_pressed && input_focused {
                        self.do_login();
                    }
                });
        });
    }

    fn draw_now_playing_strip(&mut self, ui: &mut egui::Ui) {
        let Some(playback) = &self.playback else {
            return;
        };

        let item_id = playback.item_id.clone();
        let status_text = playback.status_text.clone();
        let player_name = Self::describe_player_kind(playback.player_kind);

        egui::Frame::group(ui.style())
            .fill(Self::color_surface_alt())
            .stroke(Stroke::new(1.0, Self::color_border()))
            .corner_radius(Self::radius_l())
            .inner_margin(egui::Margin::symmetric(12, 9))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Now Playing").strong().color(Self::color_info()));
                    ui.separator();
                    ui.label(Self::muted_text(format!("Item {item_id}")));
                    ui.label(Self::muted_text(format!("Player {player_name}")));
                    ui.label(Self::muted_text(status_text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(Self::primary_button("Stop Sync")).clicked() {
                            self.stop_playback();
                        }
                    });
                });
            });
    }

    fn draw_screen_header(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        let compact = Self::is_compact_layout(ui);
        ui.label(Self::muted_text("Media Library").small());
        ui.label(
            RichText::new(title)
                .size(if compact { 26.0 } else { 31.0 })
                .strong()
                .color(Color32::from_rgb(237, 243, 248)),
        );
        ui.label(Self::muted_text(subtitle).small());
        ui.add_space(if compact { 4.0 } else { 8.0 });
    }

    fn is_compact_layout(ui: &egui::Ui) -> bool {
        ui.available_width() < 1120.0
    }

    fn card_dimensions(compact: bool) -> (f32, Vec2) {
        if compact {
            (178.0, Vec2::new(162.0, 232.0))
        } else {
            (204.0, Vec2::new(186.0, 268.0))
        }
    }

    fn show_faded_section<R>(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
        Self::section_frame(ui).show(ui, |ui| body(ui)).inner
    }

    fn draw_home(&mut self, ui: &mut egui::Ui) {
        let compact = Self::is_compact_layout(ui);
        self.draw_screen_header(
            ui,
            "Home",
            "Continue watching and newly added media across your server.",
        );

        Self::section_frame(ui).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.add(Self::primary_button("Refresh Home")).clicked() {
                        self.load_home_sections();
                        self.load_last_played();
                    }

                    if let Some(last_played) = &self.last_played_item {
                        let label = last_played
                            .name
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string());
                        ui.label(Self::muted_text(format!("Last played: {label}")).small());
                    }
                });
            });
        ui.add_space(if compact {
            Self::space_s()
        } else {
            Self::space_m()
        });

        self.draw_home_hero(ui);
        ui.add_space(if compact { Self::space_s() } else { 10.0 });

        let continue_items = self.home_continue_watching.clone();
        let recent_movies = self.home_recent_movies.clone();
        let recent_series = self.home_recent_series.clone();

        self.draw_media_row(
            ui,
            "continue_row",
            "Continue Watching",
            &continue_items,
            true,
            Screen::Home,
        );
        ui.add_space(Self::space_s());
        self.draw_media_row(
            ui,
            "recent_movies_row",
            "Recently Added Movies",
            &recent_movies,
            false,
            Screen::Home,
        );
        ui.add_space(Self::space_s());
        self.draw_media_row(
            ui,
            "recent_series_row",
            "Recently Added TV Shows",
            &recent_series,
            false,
            Screen::Home,
        );
    }

    fn hero_items(&self) -> Vec<BaseItemDto> {
        let mut seen = HashSet::new();
        let mut merged = Vec::new();

        for row in [&self.home_continue_watching, &self.home_recent_movies] {
            for item in row.iter().take(12) {
                let Some(item_id) = item.id.clone() else {
                    continue;
                };
                if seen.insert(item_id) {
                    merged.push(item.clone());
                }
            }
        }

        if merged.is_empty() {
            merged.extend(self.home_recent_series.iter().take(12).cloned());
        }

        merged
    }

    fn draw_home_hero(&mut self, ui: &mut egui::Ui) {
        let hero_items = self.hero_items();
        if hero_items.is_empty() {
            Self::section_frame(ui).show(ui, |ui| {
                ui.label(RichText::new("Featured media").strong());
                ui.label(Self::muted_text("No hero items yet."));
            });
            return;
        }

        if self.hero_index >= hero_items.len() {
            self.hero_index = 0;
        }

        let compact = Self::is_compact_layout(ui);

        let item = hero_items[self.hero_index].clone();
        let title = item.name.clone().unwrap_or_else(|| "Untitled".to_string());
        let subtitle = self.item_subtitle(&item);
        let is_episode = item
            .r#type
            .as_deref()
            .map(|kind| kind.eq_ignore_ascii_case("Episode"))
            .unwrap_or(false);
        let synopsis = item
            .overview
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_default();
        let synopsis = if synopsis.is_empty() {
            String::new()
        } else {
            Self::truncate_text(&synopsis, if compact { 220 } else { 360 })
        };

        let image_width = if compact {
            (ui.available_width() - 16.0).max(260.0)
        } else {
            420.0
        };
        let image_height = if compact {
            (image_width * 0.53).clamp(146.0, 256.0)
        } else {
            236.0
        };
        let hero_image_type = if !is_episode
            && item
                .image_tags
                .as_ref()
                .and_then(|tags| tags.backdrop.as_deref())
                .is_some()
        {
            "Backdrop"
        } else {
            "Primary"
        };

        Self::section_frame(ui).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Featured")
                            .small()
                            .color(Self::color_info())
                            .strong(),
                    );
                    ui.separator();
                    ui.spacing_mut().button_padding = egui::vec2(12.0, 6.0);
                    if ui.button("<").clicked() {
                        if self.hero_index == 0 {
                            self.hero_index = hero_items.len().saturating_sub(1);
                        } else {
                            self.hero_index = self.hero_index.saturating_sub(1);
                        }
                    }

                    ui.label(
                        RichText::new(format!(
                            "Featured {} / {}",
                            self.hero_index + 1,
                            hero_items.len()
                        ))
                        .color(Self::color_text_muted())
                        .small(),
                    );

                    if ui.button(">").clicked() {
                        self.hero_index = (self.hero_index + 1) % hero_items.len();
                    }
                });

                ui.add_space(if compact {
                    Self::space_xs()
                } else {
                    Self::space_s()
                });

                if compact {
                    if self
                        .draw_item_image_with_size(
                            ui,
                            &item,
                            Vec2::new(image_width, image_height),
                            hero_image_type,
                        )
                        .clicked()
                    {
                        self.open_item_details(item.clone(), Screen::Home);
                    }

                    ui.add_space(Self::space_s());
                    ui.label(
                        RichText::new(title)
                            .strong()
                            .size(24.0)
                            .color(Color32::from_rgb(236, 243, 248)),
                    );
                    ui.label(
                        RichText::new(subtitle)
                            .small()
                            .color(Self::color_text_muted()),
                    );
                    if !synopsis.is_empty() {
                        ui.add(
                            egui::Label::new(RichText::new(synopsis).small())
                                .wrap()
                                .selectable(false),
                        );
                    }
                    ui.add_space(Self::space_m());
                    ui.horizontal_wrapped(|ui| {
                        if ui.add(Self::primary_button("Play")).clicked() {
                            self.selected_item = Some(item.clone());
                            self.start_playback();
                        }

                        if ui.button("Open Details").clicked() {
                            self.open_item_details(item.clone(), Screen::Home);
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() * 0.52).max(280.0));
                            ui.label(
                                RichText::new(title)
                                    .strong()
                                    .size(30.0)
                                    .color(Color32::from_rgb(236, 243, 248)),
                            );
                            ui.label(RichText::new(subtitle).color(Self::color_text_muted()));
                            ui.add_space(Self::space_xs());
                            if !synopsis.is_empty() {
                                ui.add(
                                    egui::Label::new(RichText::new(synopsis).small())
                                        .wrap()
                                        .selectable(false),
                                );
                                ui.add_space(10.0);
                            } else {
                                ui.add_space(Self::space_s());
                            }
                            ui.horizontal(|ui| {
                                if ui.add(Self::primary_button("Play")).clicked() {
                                    self.selected_item = Some(item.clone());
                                    self.start_playback();
                                }

                                if ui.button("Open Details").clicked() {
                                    self.open_item_details(item.clone(), Screen::Home);
                                }
                            });
                        });

                        ui.add_space(10.0);
                        if self
                            .draw_item_image_with_size(
                                ui,
                                &item,
                                Vec2::new(image_width, image_height),
                                hero_image_type,
                            )
                            .clicked()
                        {
                            self.open_item_details(item.clone(), Screen::Home);
                        }
                    });
                }
            });
    }

    fn draw_media_row(
        &mut self,
        ui: &mut egui::Ui,
        row_id: &str,
        title: &str,
        items: &[BaseItemDto],
        show_progress: bool,
        return_screen: Screen,
    ) {
        Self::section_frame(ui).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .strong()
                        .size(20.0)
                        .color(Color32::from_rgb(236, 243, 248)),
                );
                ui.separator();
                ui.label(Self::muted_text(Self::count_text(items.len(), "item", "items")).small());
            });

            if items.is_empty() {
                ui.label(Self::muted_text("Nothing to show yet."));
                return;
            }

            let compact = Self::is_compact_layout(ui);
            egui::ScrollArea::horizontal()
                .id_salt(row_id)
                .max_height(if compact { 372.0 } else { 426.0 })
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        for item in items {
                            let clicked = self.draw_media_card(ui, item, show_progress);
                            if clicked {
                                self.open_item_details(item.clone(), return_screen);
                            }
                            ui.add_space(Self::space_m());
                        }
                    });
                });
        });
    }

    fn draw_media_card(
        &mut self,
        ui: &mut egui::Ui,
        item: &BaseItemDto,
        show_progress: bool,
    ) -> bool {
        let compact = Self::is_compact_layout(ui);
        let (card_width, image_size) = Self::card_dimensions(compact);
        let mut clicked = false;
        let watched = Self::is_item_watched(item);
        let content_width = (card_width - 20.0).max(140.0);
        let frame_fill = if watched {
            Color32::from_rgb(21, 38, 35)
        } else {
            Self::color_surface_alt()
        };
        let frame_stroke = if watched {
            Color32::from_rgb(88, 148, 123)
        } else {
            Self::color_border()
        };

        let frame = egui::Frame::group(ui.style())
            .fill(frame_fill)
            .stroke(Stroke::new(1.0, frame_stroke))
            .corner_radius(Self::radius_m())
            .inner_margin(egui::Margin::symmetric(10, 11))
            .show(ui, |ui| {
                ui.set_width(card_width);
                ui.vertical(|ui| {
                    ui.set_width(content_width);

                    if self
                        .draw_item_image_with_size(ui, item, image_size, "Primary")
                        .clicked()
                    {
                        clicked = true;
                    }

                    ui.add_space(Self::space_s());

                    let title = item.name.clone().unwrap_or_else(|| "Untitled".to_string());
                    let title_text = Self::truncate_text(&title, if compact { 72 } else { 90 });

                    if watched {
                        ui.label(RichText::new("Watched").small().color(Self::color_success()));
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new(title_text)
                                        .color(Self::color_success())
                                        .strong(),
                                )
                                .sense(egui::Sense::click())
                                .wrap(),
                            )
                            .clicked()
                        {
                            clicked = true;
                        }
                    } else if ui
                        .add(
                            egui::Label::new(RichText::new(title_text).strong())
                                .sense(egui::Sense::click())
                                .wrap(),
                        )
                        .clicked()
                    {
                        clicked = true;
                    }

                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(self.item_subtitle(item))
                            .small()
                            .color(Self::color_text_muted()),
                    );

                    if show_progress {
                        if let Some(progress) = Self::item_progress(item) {
                            ui.add_space(Self::space_s());
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .desired_width(content_width)
                                    .fill(Self::color_info()),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                Self::muted_text(format!(
                                    "Progress {}%",
                                    (progress * 100.0).round() as i32
                                ))
                                .small(),
                            );
                        }
                    }
                });
            });

        if frame.response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let hover_t = if frame.response.hovered() || frame.response.has_focus() {
            1.0
        } else {
            0.0
        };
        if hover_t > 0.0 {
            ui.painter().rect_stroke(
                frame.response.rect.expand(1.5),
                Self::radius_s(),
                Stroke::new(2.0, Self::color_info()),
                egui::StrokeKind::Outside,
            );
        }

        clicked || frame.response.clicked()
    }

    fn draw_media_grid(
        &mut self,
        ui: &mut egui::Ui,
        grid_id: &str,
        items: &[BaseItemDto],
        return_screen: Screen,
    ) {
        if items.is_empty() {
            Self::section_frame(ui).show(ui, |ui| {
                ui.label(Self::muted_text("No items found."));
            });
            return;
        }

        ui.push_id(grid_id, |ui| {
            ui.horizontal_wrapped(|ui| {
                for item in items {
                    if self.draw_media_card(ui, item, false) {
                        self.open_item_details(item.clone(), return_screen);
                    }
                    ui.add_space(Self::space_m());
                }
            });
        });
    }

    fn draw_item_image_with_size(
        &mut self,
        ui: &mut egui::Ui,
        item: &BaseItemDto,
        size: Vec2,
        image_type: &str,
    ) -> egui::Response {
        let Some(item_id) = item.id.as_deref() else {
            return Self::draw_image_placeholder(ui, size, "No artwork");
        };

        let width = size.x.max(16.0) as u32;
        let height = size.y.max(16.0) as u32;
        let tag = SlimJellyApp::image_tag_for_item(item, image_type).map(str::to_owned);
        let key = SlimJellyApp::thumbnail_key(item_id, width, height, image_type, tag.as_deref());

        if !self.thumbnail_textures.contains_key(&key) {
            if let Some(color_image) = self.thumbnail_images.remove(&key) {
                let texture = ui.ctx().load_texture(
                    format!("thumb-{key}"),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.thumbnail_textures.insert(key.clone(), texture);
            }
        }

        if !self.thumbnail_textures.contains_key(&key)
            && !self.thumbnail_pending.contains(&key)
            && !self.thumbnail_failed.contains(&key)
        {
            self.request_thumbnail(
                item_id.to_string(),
                width,
                height,
                image_type.to_string(),
                tag,
            );
        }

        if let Some(texture) = self.thumbnail_textures.get(&key) {
            ui.add(
                egui::Image::from_texture(texture)
                    .fit_to_exact_size(size)
                    .sense(egui::Sense::click()),
            )
        } else if self.thumbnail_pending.contains(&key) {
            ui.add_sized(size, egui::Spinner::new())
        } else {
            Self::draw_image_placeholder(ui, size, "No artwork")
        }
    }

    fn open_item_details(&mut self, item: BaseItemDto, return_screen: Screen) {
        self.detail_return_screen = return_screen;
        self.current_screen = Screen::Details;
        self.selected_item = Some(item.clone());

        if let Some(item_id) = item.id.clone() {
            self.load_item_detail(item_id);
        }
        self.load_detail_sections();
    }

    fn item_progress(item: &BaseItemDto) -> Option<f32> {
        let position = item
            .user_data
            .as_ref()
            .and_then(|data| data.playback_position_ticks)?;
        let total = item.run_time_ticks?;
        if total <= 0 {
            return None;
        }

        Some((position as f32 / total as f32).clamp(0.0, 1.0))
    }

    fn is_item_watched(item: &BaseItemDto) -> bool {
        item.user_data
            .as_ref()
            .and_then(|data| data.played)
            .unwrap_or(false)
    }

    fn item_subtitle(&self, item: &BaseItemDto) -> String {
        let kind = item.r#type.clone().unwrap_or_else(|| "Unknown".to_string());
        let year = item
            .production_year
            .map(|value| value.to_string())
            .unwrap_or_else(|| "--".to_string());
        format!("{kind} | {year}")
    }

    fn format_runtime(run_time_ticks: Option<i64>) -> String {
        let Some(ticks) = run_time_ticks else {
            return "--".to_string();
        };
        if ticks <= 0 {
            return "--".to_string();
        }

        let total_seconds = ticks / 10_000_000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if hours > 0 {
            format!("{hours}h {minutes:02}m")
        } else {
            format!("{minutes}m")
        }
    }

    fn truncate_text(value: &str, max_chars: usize) -> String {
        if value.chars().count() <= max_chars {
            return value.to_string();
        }

        let mut out = value.chars().take(max_chars).collect::<String>();
        out.push_str("...");
        out
    }

    fn draw_search(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(ui, "Search", "Search across libraries and metadata.");

        Self::section_frame(ui).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(Self::muted_text("Library"));
                    egui::ComboBox::from_id_salt("search_view_select")
                        .selected_text(
                            self.selected_view_id
                                .clone()
                                .unwrap_or_else(|| "All".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.selected_view_id, None, "All");
                            for view in &self.views {
                                if let Some(id) = &view.id {
                                    let label = view.name.clone().unwrap_or_else(|| id.clone());
                                    ui.selectable_value(
                                        &mut self.selected_view_id,
                                        Some(id.clone()),
                                        label,
                                    );
                                }
                            }
                        });

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.search_term)
                            .hint_text("Search movies, shows, audio, collections...")
                            .desired_width((ui.available_width() * 0.58).max(220.0)),
                    );
                    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.search_items();
                    }

                    if ui
                        .add(Self::primary_button("Search"))
                        .clicked()
                    {
                        self.search_items();
                    }
                });
            });

        if !self.search_hints.is_empty() {
            let hints = self.search_hints.clone();
            ui.horizontal_wrapped(|ui| {
                ui.label(Self::muted_text("Hints:"));
                for hint in hints.iter().take(8) {
                    let label = hint
                        .name
                        .clone()
                        .or_else(|| hint.item_id.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    if ui.button(label.clone()).clicked() {
                        self.search_term = label;
                        self.search_items();
                    }
                }
            });
        }

        ui.add_space(Self::space_s());
        let items = self.items.clone();
        self.draw_media_grid(ui, "search_grid", &items, Screen::Search);
    }

    fn draw_libraries(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(
            ui,
            "Libraries",
            "Browse all libraries by media type across your server.",
        );

        Self::section_frame(ui).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    self.library_tab_button(ui, LibrarySection::Movies, "Movies");
                    self.library_tab_button(ui, LibrarySection::TvShows, "TV Shows");
                    self.library_tab_button(ui, LibrarySection::Music, "Music");
                    self.library_tab_button(ui, LibrarySection::Audiobooks, "Audiobooks");
                });
            });

        ui.add_space(Self::space_s());
        let items = self.library_items.clone();
        self.draw_media_grid(ui, "library_grid", &items, Screen::Libraries);
    }

    fn library_tab_button(&mut self, ui: &mut egui::Ui, section: LibrarySection, label: &str) {
        let selected = self.current_library_section == section;
        let response = ui.add(
            egui::Button::new(label)
                .fill(if selected {
                    Self::color_accent_soft()
                } else {
                    Self::color_surface_alt()
                })
                .stroke(Stroke::new(
                    1.0,
                    if selected {
                        Self::color_accent()
                    } else {
                        Self::color_border()
                    },
                ))
                .corner_radius(Self::radius_s()),
        );
        if response.clicked() && !selected {
            self.current_library_section = section;
            self.load_library_items(section);
        }
    }

    fn draw_collections(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(ui, "Collections", "Grouped media sets and curated bundles.");
        Self::section_frame(ui).show(ui, |ui| {
                if ui.add(Self::primary_button("Refresh Collections")).clicked() {
                    self.load_collections();
                }
            });
        ui.add_space(Self::space_s());

        let items = self.collection_items.clone();
        self.draw_media_grid(ui, "collections_grid", &items, Screen::Collections);
    }

    fn draw_playlists_screen(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(
            ui,
            "Playlists",
            "Open playlists and browse their media items.",
        );

        Self::section_frame(ui).show(ui, |ui| {
                if ui.add(Self::primary_button("Refresh Playlists")).clicked() {
                    self.load_playlists();
                }
            });

        let playlists = self.playlists.clone();
        if playlists.is_empty() {
            ui.label(Self::muted_text("No playlists found."));
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("playlist_strip")
            .max_height(340.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for playlist in playlists {
                        let Some(playlist_id) = playlist.id.clone() else {
                            continue;
                        };

                        let selected =
                            self.selected_playlist_id.as_deref() == Some(playlist_id.as_str());
                        let stroke = if selected {
                            Stroke::new(1.4, Self::color_accent())
                        } else {
                            Stroke::new(1.0, Self::color_border())
                        };

                        egui::Frame::group(ui.style())
                            .fill(if selected {
                                Self::color_accent_soft()
                            } else {
                                Self::color_surface_alt()
                            })
                            .stroke(stroke)
                            .corner_radius(Self::radius_m())
                            .inner_margin(egui::Margin::symmetric(8, 8))
                            .show(ui, |ui| {
                                ui.set_width(188.0);
                                if self
                                    .draw_item_image_with_size(
                                        ui,
                                        &playlist,
                                        Vec2::new(172.0, 236.0),
                                        "Primary",
                                    )
                                    .clicked()
                                {
                                    self.choose_playlist(playlist_id.clone());
                                }
                                let name = playlist
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| "Untitled Playlist".to_string());
                                if ui.link(name).clicked() {
                                    self.choose_playlist(playlist_id.clone());
                                }
                                ui.label(Self::muted_text("Playlist"));
                            });

                        ui.add_space(Self::space_s());
                    }
                });
            });

        ui.add_space(Self::space_l());
        ui.label(RichText::new("Playlist Items").strong());
        let items = self.playlist_items.clone();
        self.draw_media_grid(ui, "playlist_items_grid", &items, Screen::Playlists);
    }

    fn draw_details(&mut self, ui: &mut egui::Ui) {
        let Some(item) = self.selected_item.clone() else {
            ui.label("No item selected");
            if ui.button("Back").clicked() {
                self.current_screen = self.detail_return_screen;
            }
            return;
        };

        Self::section_frame(ui).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.current_screen = self.detail_return_screen;
                    }

                    ui.separator();
                    ui.label(RichText::new("Details").strong().color(Self::color_info()));
                });
            });
        ui.add_space(Self::space_s());

        let compact = Self::is_compact_layout(ui);
        Self::show_faded_section(ui, |ui| {
            egui::Frame::group(ui.style())
                    .fill(Self::color_surface())
                    .stroke(Stroke::new(1.0, Self::color_border()))
                    .corner_radius(Self::radius_l())
                    .inner_margin(egui::Margin::symmetric(12, if compact { 8 } else { 10 }))
                    .show(ui, |ui| {
                        if compact {
                            if self
                                .draw_item_image_with_size(
                                    ui,
                                    &item,
                                    Vec2::new(220.0, 308.0),
                                    "Primary",
                                )
                                .clicked()
                            {
                                self.selected_item = Some(item.clone());
                            }
                            ui.add_space(Self::space_m());
                            self.draw_details_header_text(ui, &item, true);
                        } else {
                            ui.horizontal(|ui| {
                                if self
                                    .draw_item_image_with_size(
                                        ui,
                                        &item,
                                        Vec2::new(240.0, 340.0),
                                        "Primary",
                                    )
                                    .clicked()
                                {
                                    self.selected_item = Some(item.clone());
                                }

                                ui.vertical(|ui| {
                                    self.draw_details_header_text(ui, &item, false);
                                });
                            });
                        }
                    });
        });

        ui.add_space(if compact {
            Self::space_s()
        } else {
            Self::space_m()
        });
        Self::show_faded_section(ui, |ui| {
            ui.label(RichText::new("Synopsis").strong().color(Self::color_info()));
            let synopsis = item
                .overview
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "--".to_string());
            ui.label(synopsis);
        });

        ui.add_space(if compact {
            Self::space_xs()
        } else {
            Self::space_s()
        });
        self.draw_subtitle_panel(ui);
        ui.add_space(if compact {
            Self::space_xs()
        } else {
            Self::space_s()
        });
        self.draw_detail_seasons_section(ui, &item);
        ui.add_space(if compact {
            Self::space_xs()
        } else {
            Self::space_s()
        });
        self.draw_detail_cast_section(ui, &item);
        ui.add_space(if compact {
            Self::space_xs()
        } else {
            Self::space_s()
        });
        self.draw_detail_related_section(ui);
    }

    fn draw_details_header_text(&mut self, ui: &mut egui::Ui, item: &BaseItemDto, compact: bool) {
        let title = item.name.clone().unwrap_or_else(|| "Untitled".to_string());
        ui.label(
            RichText::new(title)
                .size(if compact { 24.0 } else { 30.0 })
                .strong()
                .color(Color32::from_rgb(237, 243, 248)),
        );

        let year = item
            .production_year
            .map(|v| v.to_string())
            .unwrap_or_else(|| "--".to_string());
        let rating = item
            .community_rating
            .map(|v| format!("{v:.1}/10"))
            .unwrap_or_else(|| "--".to_string());
        let duration = Self::format_runtime(item.run_time_ticks);
        let age = item
            .official_rating
            .clone()
            .unwrap_or_else(|| "--".to_string());

        if compact {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    Self::muted_text(format!("Year {year}")),
                );
                ui.separator();
                ui.label(
                    Self::muted_text(format!("Rating {rating}")),
                );
                ui.separator();
                ui.label(
                    Self::muted_text(format!("Duration {duration}")),
                );
                ui.separator();
                ui.label(
                    Self::muted_text(format!("Age {age}")),
                );
            });
        } else {
            ui.label(
                Self::muted_text(format!(
                    "Year: {year} | Rating: {rating} | Duration: {duration} | Age: {age}"
                )),
            );
        }

        ui.label(Self::muted_text(self.detail_tech_summary()));

        ui.add_space(if compact {
            Self::space_s()
        } else {
            Self::space_m()
        });
        if compact {
            ui.horizontal_wrapped(|ui| {
                self.draw_details_action_buttons(ui, item);
                self.draw_episode_navigation_buttons(ui, item);
            });
        } else {
            ui.horizontal(|ui| {
                self.draw_details_action_buttons(ui, item);
                self.draw_episode_navigation_buttons(ui, item);
            });
        }
    }

    fn draw_details_action_buttons(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        let can_resume = Self::item_progress(item).map(|v| v > 0.0).unwrap_or(false);
        let watched = Self::is_item_watched(item);
        let play_label = if can_resume { "Resume" } else { "Play" };
        if ui.add(Self::primary_button(play_label)).clicked() {
            self.selected_item = Some(item.clone());
            self.start_playback();
        }

        if !watched && ui.button("Mark Played").clicked() {
            self.selected_item = Some(item.clone());
            self.mark_selected_item_played();
        }

        if watched {
            ui.label(RichText::new("Watched").color(Self::color_success()));
        }

        if ui.button("Shuffle").clicked() {
            self.selected_item = Some(item.clone());
            self.shuffle_play_selected_context();
        }

        ui.menu_button("Add to Playlist", |ui| {
            if self.playlists.is_empty() {
                ui.label(Self::muted_text("No playlists loaded"));
                if ui.button("Refresh Playlists").clicked() {
                    self.load_playlists();
                    ui.close_menu();
                }
                return;
            }

            let playlists = self.playlists.clone();
            for playlist in playlists {
                let Some(playlist_id) = playlist.id.clone() else {
                    continue;
                };

                let label = playlist
                    .name
                    .clone()
                    .unwrap_or_else(|| "Untitled Playlist".to_string());
                if ui.button(label).clicked() {
                    self.selected_item = Some(item.clone());
                    self.add_selected_item_to_playlist(playlist_id);
                    ui.close_menu();
                }
            }
        });

        ui.menu_button("More", |ui| {
            if watched && ui.button("Mark Unplayed").clicked() {
                self.selected_item = Some(item.clone());
                self.mark_selected_item_unplayed();
                ui.close_menu();
            }

            if ui.button("Refresh Metadata").clicked() {
                if let Some(item_id) = item.id.clone() {
                    self.refresh_item_by_id(item_id);
                } else {
                    self.status_line = "Selected item has no id".to_string();
                }
                ui.close_menu();
            }

            if ui.button("Download Subtitles").clicked() {
                self.subtitle_panel_open = !self.subtitle_panel_open;
                if self.subtitle_panel_open {
                    self.search_subtitles();
                }
                ui.close_menu();
            }

            ui.separator();
            ui.label(
                Self::muted_text("Delete is available in Admin only").small(),
            );
        });
    }

    fn draw_episode_navigation_buttons(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        let is_episode = item
            .r#type
            .as_deref()
            .map(|item_type| item_type.eq_ignore_ascii_case("Episode"))
            .unwrap_or(false);
        if !is_episode {
            return;
        }

        let Some(item_id) = item.id.as_deref() else {
            ui.add_enabled(false, egui::Button::new("Previous"))
                .on_hover_text("Episode metadata unavailable");
            ui.add_enabled(false, egui::Button::new("Next"))
                .on_hover_text("Episode metadata unavailable");
            return;
        };

        let ordered_episodes = Self::sorted_episode_items(&self.detail_episodes);
        let current_index = ordered_episodes
            .iter()
            .position(|episode| episode.id.as_deref() == Some(item_id));

        let previous_episode = current_index
            .and_then(|idx| idx.checked_sub(1))
            .and_then(|idx| ordered_episodes.get(idx))
            .cloned();

        let next_episode = current_index
            .and_then(|idx| ordered_episodes.get(idx.saturating_add(1)))
            .cloned();

        let on_last_in_season = current_index
            .map(|idx| idx.saturating_add(1) == ordered_episodes.len())
            .unwrap_or(false);

        let next_season_id = if on_last_in_season && next_episode.is_none() {
            self.next_numbered_season_id_for_episode(item)
        } else {
            None
        };

        let previous_clicked = if previous_episode.is_some() {
            ui.add_enabled(true, egui::Button::new("Previous")).clicked()
        } else {
            ui.add_enabled(false, egui::Button::new("Previous"))
                .on_hover_text("First episode")
                .clicked()
        };
        if previous_clicked {
            if let Some(previous_episode) = previous_episode {
                self.open_item_details(previous_episode, self.detail_return_screen);
            }
        }

        if next_season_id.is_some() {
            ui.label(
                RichText::new("Season finale")
                    .small()
                    .color(Self::color_accent()),
            );
        }

        let next_label = if next_season_id.is_some() {
            "Next Season"
        } else {
            "Next"
        };
        let next_enabled = next_episode.is_some() || next_season_id.is_some();
        let next_clicked = if next_enabled {
            ui.add_enabled(true, egui::Button::new(next_label)).clicked()
        } else {
            let disabled_reason = if current_index.is_none() {
                "Loading episode order"
            } else {
                "Series finale"
            };
            ui.add_enabled(false, egui::Button::new(next_label))
                .on_hover_text(disabled_reason)
                .clicked()
        };
        if next_clicked {
            if let Some(next_episode) = next_episode {
                self.open_item_details(next_episode, self.detail_return_screen);
            } else if let Some(next_season_id) = next_season_id {
                self.detail_selected_season_id = Some(next_season_id.clone());
                self.detail_preferred_season_id = Some(next_season_id.clone());
                self.detail_pending_next_season_id = Some(next_season_id.clone());
                self.detail_episodes.clear();
                self.load_detail_episodes(next_season_id);
            }
        }
    }

    fn next_numbered_season_id_for_episode(&self, episode: &BaseItemDto) -> Option<String> {
        let mut numbered_seasons = self
            .detail_seasons
            .iter()
            .filter_map(|season| {
                let season_id = season.id.clone()?;
                let season_number = season.index_number?;
                (season_number > 0).then_some((season_number, season_id))
            })
            .collect::<Vec<_>>();

        numbered_seasons.sort_by_key(|(season_number, _)| *season_number);

        let current_season_id = episode
            .season_id
            .clone()
            .or_else(|| episode.parent_id.clone())
            .or_else(|| self.detail_selected_season_id.clone());

        if let Some(current_season_id) = current_season_id {
            if let Some(position) = numbered_seasons
                .iter()
                .position(|(_, season_id)| season_id == &current_season_id)
            {
                return numbered_seasons.get(position + 1).map(|(_, id)| id.clone());
            }
        }

        let current_season_number = episode
            .parent_index_number
            .filter(|number| *number > 0)
            .or_else(|| {
                self.detail_selected_season_id.as_deref().and_then(|selected_id| {
                    self.detail_seasons
                        .iter()
                        .find(|season| season.id.as_deref() == Some(selected_id))
                        .and_then(|season| season.index_number)
                        .filter(|number| *number > 0)
                })
            });

        current_season_number.and_then(|current_number| {
            numbered_seasons
                .into_iter()
                .find(|(season_number, _)| *season_number > current_number)
                .map(|(_, season_id)| season_id)
        })
    }

    fn sorted_episode_items(items: &[BaseItemDto]) -> Vec<BaseItemDto> {
        let mut episodes = items.to_vec();
        episodes.sort_by(|a, b| {
            a.index_number
                .unwrap_or(i32::MAX)
                .cmp(&b.index_number.unwrap_or(i32::MAX))
                .then_with(|| a.name.as_deref().unwrap_or("").cmp(b.name.as_deref().unwrap_or("")))
        });
        episodes
    }

    fn draw_detail_seasons_section(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        Self::show_faded_section(ui, |ui| {
                ui.label(
                    RichText::new("Seasons & Episodes")
                        .strong()
                        .color(Self::color_info()),
                );

                let is_series = item
                    .r#type
                    .as_deref()
                    .map(|item_type| item_type.eq_ignore_ascii_case("Series"))
                    .unwrap_or(false);

                if !is_series {
                    ui.label(Self::muted_text("--"));
                    return;
                }

                if self.detail_seasons.is_empty() {
                    ui.label(Self::muted_text("No seasons found"));
                    return;
                }

                let seasons = self.detail_seasons.clone();
                ui.horizontal(|ui| {
                    ui.label("Season");

                    let selected_text = self
                        .detail_selected_season_id
                        .as_deref()
                        .and_then(|selected| {
                            seasons
                                .iter()
                                .find(|season| season.id.as_deref() == Some(selected))
                        })
                        .and_then(|season| season.name.clone())
                        .unwrap_or_else(|| "--".to_string());

                    egui::ComboBox::from_id_salt("detail_season_select")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for season in &seasons {
                                let Some(season_id) = season.id.clone() else {
                                    continue;
                                };
                                let label = season
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| "Untitled season".to_string());
                                let selected = self.detail_selected_season_id.as_deref()
                                    == Some(season_id.as_str());

                                if ui.selectable_label(selected, label).clicked() {
                                    self.choose_detail_season(season_id);
                                }
                            }
                        });
                });

                if self.detail_episodes.is_empty() {
                    ui.add_space(Self::space_xs());
                    ui.label(Self::muted_text("No episodes found"));
                    return;
                }

                let episodes = Self::sorted_episode_items(&self.detail_episodes);
                egui::ScrollArea::vertical()
                    .id_salt("detail_episodes_list")
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for episode in &episodes {
                            let watched = Self::is_item_watched(episode);
                            egui::Frame::group(ui.style())
                                .fill(if watched {
                                    Color32::from_rgb(20, 39, 34)
                                } else {
                                    Self::color_surface_alt()
                                })
                                .stroke(Stroke::new(1.0, Self::color_border()))
                                .corner_radius(Self::radius_m())
                                .inner_margin(egui::Margin::symmetric(8, 8))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if self
                                            .draw_item_image_with_size(
                                                ui,
                                                episode,
                                                Vec2::new(148.0, 84.0),
                                                "Primary",
                                            )
                                            .clicked()
                                        {
                                            self.open_item_details(
                                                episode.clone(),
                                                self.detail_return_screen,
                                            );
                                        }

                                        ui.vertical(|ui| {
                                            let episode_title = Self::episode_title(episode);
                                            if watched {
                                                if ui
                                                    .add(
                                                        egui::Label::new(
                                                            RichText::new(format!(
                                                                "{}  Watched",
                                                                episode_title
                                                            ))
                                                            .color(Self::color_success()),
                                                        )
                                                        .sense(egui::Sense::click()),
                                                    )
                                                    .clicked()
                                                {
                                                    self.open_item_details(
                                                        episode.clone(),
                                                        self.detail_return_screen,
                                                    );
                                                }
                                            } else if ui.link(episode_title).clicked() {
                                                self.open_item_details(
                                                    episode.clone(),
                                                    self.detail_return_screen,
                                                );
                                            }

                                            ui.label(
                                                Self::muted_text(Self::episode_subtitle(episode)),
                                            );

                                            let overview = episode
                                                .overview
                                                .as_deref()
                                                .map(str::trim)
                                                .filter(|value| !value.is_empty())
                                                .map(|value| {
                                                    let mut text =
                                                        value.chars().take(170).collect::<String>();
                                                    if value.chars().count() > 170 {
                                                        text.push_str("...");
                                                    }
                                                    text
                                                })
                                                .unwrap_or_else(|| "--".to_string());
                                            ui.label(Self::muted_text(overview).small());
                                        });
                                    });
                                });
                            ui.add_space(Self::space_xs());
                        }
                    });
            });
    }

    fn draw_detail_cast_section(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        Self::show_faded_section(ui, |ui| {
            ui.label(RichText::new("Cast & Crew").strong().color(Self::color_info()));

            let Some(people) = item.people.as_ref() else {
                ui.label(Self::muted_text("--"));
                return;
            };
            if people.is_empty() {
                ui.label(Self::muted_text("--"));
                return;
            }

            egui::ScrollArea::horizontal()
                .id_salt("detail_people_row")
                .max_height(118.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for person in people.iter().take(20) {
                            egui::Frame::group(ui.style())
                                .fill(Self::color_surface_alt())
                                .stroke(Stroke::new(1.0, Self::color_border()))
                                .corner_radius(Self::radius_m())
                                .inner_margin(egui::Margin::symmetric(8, 8))
                                .show(ui, |ui| {
                                    ui.set_width(152.0);

                                    let name = person
                                        .name
                                        .clone()
                                        .unwrap_or_else(|| "Unknown".to_string());
                                    ui.label(RichText::new(name.clone()).strong());

                                    let role = person
                                        .role
                                        .clone()
                                        .or_else(|| person.r#type.clone())
                                        .unwrap_or_else(|| "--".to_string());
                                    ui.label(Self::muted_text(role));

                                    if ui.button("Search").clicked() {
                                        self.search_term = name;
                                        self.navigate_to(Screen::Search);
                                        self.search_items();
                                    }
                                });

                            ui.add_space(Self::space_s());
                        }
                    });
                });
        });
    }

    fn draw_detail_related_section(&mut self, ui: &mut egui::Ui) {
        Self::show_faded_section(ui, |ui| {
            ui.label(RichText::new("More Like This").strong().color(Self::color_info()));

            if self.detail_related.is_empty() {
                ui.label(Self::muted_text("--"));
                return;
            }

            let related_items = self.detail_related.clone();
            let return_screen = self.detail_return_screen;
            egui::ScrollArea::horizontal()
                .id_salt("detail_related_row")
                .max_height(320.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for related in &related_items {
                            if self.draw_media_card(ui, related, false) {
                                self.open_item_details(related.clone(), return_screen);
                            }
                            ui.add_space(Self::space_s());
                        }
                    });
                });
        });
    }

    fn episode_title(item: &BaseItemDto) -> String {
        let name = item.name.clone().unwrap_or_else(|| "Untitled".to_string());
        let episode_number = item
            .index_number
            .map(|value| value.to_string())
            .unwrap_or_else(|| "--".to_string());
        format!("{episode_number}. {name}")
    }

    fn episode_subtitle(item: &BaseItemDto) -> String {
        let season = item
            .parent_index_number
            .map(|value| value.to_string())
            .unwrap_or_else(|| "--".to_string());
        let episode = item
            .index_number
            .map(|value| value.to_string())
            .unwrap_or_else(|| "--".to_string());
        let runtime = Self::format_runtime(item.run_time_ticks);
        format!("S{season}:E{episode} | {runtime}")
    }

    fn detail_tech_summary(&self) -> String {
        let Some(media) = &self.detail_media_source else {
            return "Tech: --".to_string();
        };

        let container = media.container.clone().unwrap_or_else(|| "--".to_string());
        let direct = if media.supports_direct_play.unwrap_or(false) {
            "Direct"
        } else {
            "No Direct"
        };
        let transcode = if media.supports_transcoding.unwrap_or(false) {
            "Transcode"
        } else {
            "No Transcode"
        };

        format!("Tech: {container} | {direct} | {transcode}")
    }

    fn draw_admin(&mut self, ui: &mut egui::Ui) {
        let Some(session) = &self.session else {
            return;
        };
        if !session.is_admin {
            ui.label(Self::muted_text("Admin panel hidden for non-admin account."));
            return;
        }

        self.draw_screen_header(
            ui,
            "Dashboard",
            "Server admin controls and scheduled tasks.",
        );

        Self::section_frame(ui).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.add(Self::primary_button("Scan All Libraries")).clicked() {
                        self.trigger_scan_all();
                    }
                    if ui.button("Refresh Tasks").clicked() {
                        self.refresh_tasks();
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Library or Item ID");
                    let id_resp =
                        ui.add(egui::TextEdit::singleline(&mut self.selected_library_id).desired_width(340.0));
                    if ui.button("Refresh Item").clicked() {
                        self.trigger_refresh_item();
                    }

                    if id_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.trigger_refresh_item();
                    }
                });
            });

        ui.separator();
        Self::section_frame(ui).show(ui, |ui| {
                ui.label(RichText::new("Scheduled Tasks").strong());
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for task in &self.tasks {
                            let name = task
                                .name
                                .clone()
                                .unwrap_or_else(|| "Unnamed task".to_string());
                            let state = task.state.clone().unwrap_or_else(|| "unknown".to_string());
                            let progress = task
                                .current_progress_percentage
                                .map(|v| format!("{v:.0}%"))
                                .unwrap_or_else(|| "n/a".to_string());
                            ui.label(Self::muted_text(format!("{name} | {state} | {progress}")));
                        }
                    });
            });

        ui.add_space(Self::space_m());
        ui.separator();
        ui.label(
            RichText::new("Danger Zone")
                .strong()
                .color(Color32::from_rgb(210, 78, 95)),
        );
        ui.label(
            Self::muted_text(
                "Deletes are permanent and can remove files from disk depending on server configuration.",
            )
            .small(),
        );

        ui.add_space(Self::space_s());
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(40, 24, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(95, 36, 47)))
            .corner_radius(Self::radius_m())
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(RichText::new("Delete Item by ID").strong());
                ui.label(Self::muted_text("Type DELETE to enable item deletion.").small());

                ui.horizontal(|ui| {
                    ui.label("Confirm");
                    ui.text_edit_singleline(&mut self.admin_delete_item_confirm);
                });

                let can_delete_item = !self.selected_library_id.trim().is_empty()
                    && self.admin_delete_item_confirm.trim() == "DELETE";
                if ui
                    .add_enabled(can_delete_item, Self::danger_button("Delete Item"))
                    .clicked()
                {
                    self.delete_admin_item_by_id(self.selected_library_id.trim().to_string());
                }
            });

        ui.add_space(Self::space_s());
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(40, 24, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(95, 36, 47)))
            .corner_radius(Self::radius_m())
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Delete Library").strong());
                    if ui.button("Refresh Libraries").clicked() {
                        self.load_virtual_folders();
                    }
                });

                if self.admin_virtual_folders.is_empty() {
                    ui.label(Self::muted_text("No virtual folders loaded."));
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label("Library");
                    let selected_name = self
                        .admin_selected_virtual_folder_name
                        .clone()
                        .unwrap_or_else(|| "Select library".to_string());
                    egui::ComboBox::from_id_salt("admin_virtual_folder_select")
                        .selected_text(selected_name)
                        .show_ui(ui, |ui| {
                            for folder in self.admin_virtual_folders.clone() {
                                let Some(name) = folder.name.clone() else {
                                    continue;
                                };
                                let selected = self.admin_selected_virtual_folder_name.as_deref()
                                    == Some(name.as_str());
                                if ui.selectable_label(selected, &name).clicked() {
                                    self.admin_selected_virtual_folder_name = Some(name);
                                }
                            }
                        });
                });

                let selected_library_name = self
                    .admin_selected_virtual_folder_name
                    .clone()
                    .unwrap_or_default();
                ui.label(
                    Self::muted_text(format!(
                        "Type library name exactly to confirm: {}",
                        if selected_library_name.is_empty() {
                            "(none selected)"
                        } else {
                            selected_library_name.as_str()
                        }
                    ))
                    .small(),
                );
                ui.horizontal(|ui| {
                    ui.label("Confirm");
                    ui.text_edit_singleline(&mut self.admin_delete_library_confirm);
                });

                let can_delete_library = !selected_library_name.is_empty()
                    && self.admin_delete_library_confirm.trim() == selected_library_name;
                if ui
                    .add_enabled(can_delete_library, Self::danger_button("Delete Library"))
                    .clicked()
                {
                    self.delete_admin_virtual_folder(selected_library_name);
                }
            });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(ui, "Settings", "Playback and client behavior.");

        Self::section_frame(ui).show(ui, |ui| {
            ui.label(RichText::new("Playback").strong().color(Self::color_info()));

            ui.horizontal_wrapped(|ui| {
                    ui.label("Preferred Player");
                    egui::ComboBox::from_id_salt("preferred_player")
                        .selected_text(match self.config.player.preferred {
                            PreferredPlayer::Mpv => "mpv",
                            PreferredPlayer::Vlc => "VLC",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.config.player.preferred,
                                PreferredPlayer::Mpv,
                                "mpv",
                            );
                            ui.selectable_value(
                                &mut self.config.player.preferred,
                                PreferredPlayer::Vlc,
                                "VLC",
                            );
                        });
            });

            ui.horizontal_wrapped(|ui| {
                    ui.label("mpv path (optional)");
                    let mut value = self.config.player.mpv_path.clone().unwrap_or_default();
                    let response =
                        ui.add(egui::TextEdit::singleline(&mut value).desired_width(320.0));
                    if response.changed() {
                        let trimmed = value.trim().to_string();
                        self.config.player.mpv_path = if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed)
                        };
                    }

                    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.save_settings();
                    }
            });

            ui.horizontal_wrapped(|ui| {
                    ui.label("VLC path (optional)");
                    let mut value = self.config.player.vlc_path.clone().unwrap_or_default();
                    let response =
                        ui.add(egui::TextEdit::singleline(&mut value).desired_width(320.0));
                    if response.changed() {
                        let trimmed = value.trim().to_string();
                        self.config.player.vlc_path = if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed)
                        };
                    }

                    if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.save_settings();
                    }
            });

            ui.horizontal_wrapped(|ui| {
                    ui.label("Base Sync Interval (s)");
                    let mut interval = i64::try_from(self.config.playback.base_sync_interval_seconds)
                        .unwrap_or(15)
                        .clamp(5, 120);
                    if ui
                        .add(egui::DragValue::new(&mut interval).range(5..=120))
                        .changed()
                    {
                        self.config.playback.base_sync_interval_seconds = interval as u64;
                    }
            });

            ui.horizontal_wrapped(|ui| {
                    ui.checkbox(
                        &mut self.config.server.allow_self_signed,
                        "Allow self-signed certs",
                    );
                    ui.checkbox(&mut self.config.playback.direct_first, "Direct play first");
                    ui.checkbox(
                        &mut self.config.playback.fallback_once,
                        "Transcode fallback once",
                    );
            });
        });

        ui.add_space(12.0);
        Self::section_frame(ui).show(ui, |ui| {
            ui.label(RichText::new("OpenSubtitles").strong().size(16.0).color(Self::color_info()));
            ui.label(Self::muted_text("Configure subtitle search and download.").small());

            ui.horizontal_wrapped(|ui| {
                    ui.label("API Key");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.config.subtitles.api_key)
                            .password(true)
                            .desired_width(300.0),
                    );
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.save_settings();
                    }
            });

            ui.horizontal_wrapped(|ui| {
                    ui.label("Username");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.config.subtitles.username)
                            .desired_width(250.0),
                    );
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.save_settings();
                    }
            });

            ui.horizontal_wrapped(|ui| {
                    ui.label("Password");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.config.subtitles.password)
                            .password(true)
                            .desired_width(250.0),
                    );
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.save_settings();
                    }
            });

            ui.horizontal_wrapped(|ui| {
                    ui.label("Default Language");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.config.subtitles.default_language)
                            .desired_width(80.0),
                    );
                    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.save_settings();
                    }
                    ui.label(
                        Self::muted_text("ISO 639-1 codes: en, ar, fr, es, de …").small(),
                    );
            });
        });

        ui.add_space(Self::space_m());
        if ui.add(Self::primary_button("Save Settings")).clicked() {
            self.save_settings();
        }
    }

    fn draw_subtitle_panel(&mut self, ui: &mut egui::Ui) {
        if !self.subtitle_panel_open {
            return;
        }

        Self::show_faded_section(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Subtitles").strong().color(Self::color_info()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            self.subtitle_panel_open = false;
                        }
                    });
                });

                if let Some(path) = &self.subtitle_temp_path {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✓ Downloaded:").small().strong());
                        ui.label(Self::muted_text(path.clone()).small());
                    });
                    ui.add_space(Self::space_xs());
                }

                ui.horizontal(|ui| {
                    ui.label("Language");
                    let lang_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.subtitle_search_language)
                            .desired_width(80.0),
                    );
                    if ui.button("Search").clicked()
                        || (lang_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.search_subtitles();
                    }
                    if self.subtitle_search_loading {
                        ui.spinner();
                    }
                });

                if self.subtitle_search_results.is_empty() && !self.subtitle_search_loading {
                    ui.label(Self::muted_text("No results"));
                    return;
                }

                let results = self.subtitle_search_results.clone();
                egui::ScrollArea::vertical()
                    .id_salt("subtitle_results_scroll")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for result in &results {
                            let Some(attrs) = &result.attributes else {
                                continue;
                            };

                            let release = attrs
                                .release
                                .clone()
                                .unwrap_or_else(|| "Unknown release".to_string());
                            let lang = attrs.language.clone().unwrap_or_else(|| "--".to_string());
                            let downloads = attrs.download_count.unwrap_or(0);
                            let trusted = attrs.from_trusted.unwrap_or(false);

                            egui::Frame::group(ui.style())
                                .fill(Self::color_surface_alt())
                                .stroke(Stroke::new(1.0, Self::color_border()))
                                .corner_radius(Self::radius_m())
                                .inner_margin(egui::Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&release).small().strong());
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    Self::muted_text(format!("Lang: {lang}")).small(),
                                                );
                                                ui.label(
                                                    Self::muted_text(format!("DL: {downloads}")).small(),
                                                );
                                                if trusted {
                                                    ui.label(
                                                        RichText::new("✓ Trusted")
                                                            .small()
                                                            .color(Self::color_success()),
                                                    );
                                                }
                                            });
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if let Some(files) = &attrs.files {
                                                    if let Some(file) = files.first() {
                                                        if let Some(file_id) = file.file_id {
                                                            let fname = file
                                                                .file_name
                                                                .clone()
                                                                .unwrap_or_else(|| {
                                                                    format!("{file_id}.srt")
                                                                });
                                                            if ui.button("Download").clicked() {
                                                                self.download_subtitle(
                                                                    file_id, fname,
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                        }
                    });
            });
    }
}
