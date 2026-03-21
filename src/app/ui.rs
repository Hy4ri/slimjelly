use std::{collections::HashSet, sync::atomic::Ordering};

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::{
    config::PreferredPlayer,
    jellyfin::models::BaseItemDto,
};

use super::{LibrarySection, Screen, SessionView, SlimJellyApp, UiMessage};

impl SlimJellyApp {
    pub(super) fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        let accent = Color32::from_rgb(172, 36, 54);
        let accent_soft = Color32::from_rgb(118, 34, 46);

        visuals.panel_fill = Color32::from_rgb(13, 14, 18);
        visuals.faint_bg_color = Color32::from_rgb(25, 27, 33);
        visuals.extreme_bg_color = Color32::from_rgb(8, 9, 12);
        visuals.selection.bg_fill = accent_soft;
        visuals.selection.stroke = Stroke::new(1.0, accent);
        visuals.hyperlink_color = accent;
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(20, 22, 28);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 30, 37);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 28, 35);
        visuals.widgets.active.bg_fill = Color32::from_rgb(66, 30, 40);
        visuals.widgets.open.bg_fill = Color32::from_rgb(34, 36, 43);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.window_margin = egui::Margin::symmetric(14, 12);
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

                        self.refresh_health();
                        self.load_views();
                        self.load_home_sections();
                        self.load_library_items(self.current_library_section);
                        self.load_collections();
                        self.load_playlists();
                        self.load_last_played();
                        self.search_items();
                    }
                    Err(err) => {
                        self.status_line = format!("Failed to build API client: {err}");
                    }
                },
                UiMessage::LoginFailed(message) => {
                    self.status_line = format!("Login failed: {message}");
                }
                UiMessage::HealthResult { ping, info } => {
                    self.health_ping = Some(ping);
                    self.health_info = Some(info);
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
                    self.status_line = format!("Loaded {} views", self.views.len());
                }
                UiMessage::SearchHintsLoaded(hints) => {
                    self.search_hints = hints;
                }
                UiMessage::SearchLoaded(result) => {
                    self.items = result.items.unwrap_or_default();
                    self.status_line = format!("Loaded {} search items", self.items.len());
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
                        self.status_line =
                            format!("Loaded {} library items", self.library_items.len());
                    }
                }
                UiMessage::LibraryItemsFailed(message) => {
                    self.status_line = format!("Library load failed: {message}");
                }
                UiMessage::CollectionItemsLoaded(items) => {
                    self.collection_items = items;
                    self.status_line =
                        format!("Loaded {} collections", self.collection_items.len());
                }
                UiMessage::CollectionItemsFailed(message) => {
                    self.status_line = format!("Collection load failed: {message}");
                }
                UiMessage::DetailSeasonsLoaded(seasons) => {
                    self.detail_seasons = seasons;

                    if self.detail_seasons.is_empty() {
                        self.detail_selected_season_id = None;
                        self.detail_episodes.clear();
                        continue;
                    }

                    let selected_exists = self
                        .detail_selected_season_id
                        .as_deref()
                        .map(|selected| {
                            self.detail_seasons
                                .iter()
                                .any(|season| season.id.as_deref() == Some(selected))
                        })
                        .unwrap_or(false);

                    if !selected_exists {
                        if let Some(first_id) = self.detail_seasons.first().and_then(|s| s.id.clone()) {
                            self.choose_detail_season(first_id);
                        }
                    } else if self.detail_episodes.is_empty() {
                        if let Some(selected_id) = self.detail_selected_season_id.clone() {
                            self.choose_detail_season(selected_id);
                        }
                    }
                }
                UiMessage::DetailSeasonsFailed(message) => {
                    self.status_line = format!("Seasons load failed: {message}");
                }
                UiMessage::DetailEpisodesLoaded { season_id, items } => {
                    if self.detail_selected_season_id.as_deref() == Some(season_id.as_str()) {
                        self.detail_episodes = items;
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
                        self.selected_playlist_id = self.playlists.first().and_then(|p| p.id.clone());
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
                UiMessage::ThumbnailLoaded {
                    key,
                    bytes,
                } => {
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
                    stream_url,
                    transcode_stream_url,
                    used_transcode,
                    media_source_id,
                    play_session_id,
                } => {
                    self.launch_external_player(
                        item_id,
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
                UiMessage::PlaybackStopped => {
                    if self.playback.is_some() {
                        self.playback = None;
                    }
                }
                UiMessage::TasksLoaded(tasks) => {
                    self.tasks = tasks;
                    self.status_line = format!("Loaded {} tasks", self.tasks.len());
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
                UiMessage::MarkPlayedDone { item_id } => {
                    self.status_line = "Marked played".to_string();
                    if self.current_screen == Screen::Details {
                        self.load_item_detail(item_id);
                    }
                    self.load_home_sections();
                    self.load_last_played();
                    if self.current_screen == Screen::Libraries {
                        self.load_library_items(self.current_library_section);
                    }
                }
                UiMessage::MarkPlayedFailed(message) => {
                    self.status_line = format!("Mark played failed: {message}");
                }
            }
        }
    }

    pub(super) fn draw_app_shell(&mut self, ctx: &egui::Context) {
        let sidebar_response = egui::SidePanel::left("main_sidebar")
            .resizable(false)
            .exact_width(self.sidebar_width)
            .show(ctx, |ui| {
                self.draw_sidebar(ui);
            });

        let pointer_near_sidebar = ctx
            .input(|i| i.pointer.latest_pos())
            .map(|pos| pos.x <= self.sidebar_width + 6.0)
            .unwrap_or(false);

        self.sidebar_expanded = sidebar_response.response.hovered() || pointer_near_sidebar;
        self.animate_sidebar_width(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_now_playing_strip(ui);
            ui.add_space(6.0);

            match self.current_screen {
                Screen::Login => self.draw_login(ui),
                Screen::Home => self.draw_home(ui),
                Screen::Search => self.draw_search(ui),
                Screen::Libraries => self.draw_libraries(ui),
                Screen::Collections => self.draw_collections(ui),
                Screen::Playlists => self.draw_playlists_screen(ui),
                Screen::Admin => self.draw_admin(ui),
                Screen::Settings => self.draw_settings(ui),
                Screen::Details => self.draw_details(ui),
            }
        });
    }

    fn animate_sidebar_width(&mut self, ctx: &egui::Context) {
        let target = if self.sidebar_expanded { 236.0 } else { 68.0 };
        let dt = ctx.input(|i| i.stable_dt).clamp(1.0 / 240.0, 1.0 / 24.0);
        let speed = if self.sidebar_expanded { 16.0 } else { 11.0 };
        let blend = 1.0 - (-speed * dt).exp();

        self.sidebar_width = egui::lerp(self.sidebar_width..=target, blend).clamp(68.0, 236.0);
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            if self.sidebar_expanded {
                ui.label(RichText::new("slimjelly").strong().size(18.0));
                if let Some(session) = &self.session {
                    ui.label(RichText::new(&session.user_name).weak());
                }
            } else {
                ui.label(RichText::new("sj").strong());
            }
        });

        ui.add_space(8.0);
        self.sidebar_nav_button(ui, "HM", "Home", Screen::Home);
        self.sidebar_nav_button(ui, "SR", "Search", Screen::Search);

        ui.add_space(4.0);
        if self.sidebar_expanded {
            ui.label(RichText::new("Libraries").weak());
        }

        self.sidebar_library_button(ui, "MV", "Movies", LibrarySection::Movies);
        self.sidebar_library_button(ui, "TV", "TV Shows", LibrarySection::TvShows);
        self.sidebar_library_button(ui, "MU", "Music", LibrarySection::Music);
        self.sidebar_library_button(ui, "AB", "Audiobooks", LibrarySection::Audiobooks);

        ui.add_space(4.0);
        self.sidebar_nav_button(ui, "CL", "Collections", Screen::Collections);
        self.sidebar_nav_button(ui, "PL", "Playlists", Screen::Playlists);

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if ui.button(self.sidebar_label("LO", "Logout")).clicked() {
                self.do_logout();
            }

            self.sidebar_nav_button(ui, "ST", "Settings", Screen::Settings);
            if self.session.as_ref().map(|s| s.is_admin).unwrap_or(false) {
                self.sidebar_nav_button(ui, "DB", "Dashboard", Screen::Admin);
            }
        });
    }

    fn sidebar_label(&self, compact: &str, expanded: &str) -> String {
        if self.sidebar_expanded {
            expanded.to_string()
        } else {
            compact.to_string()
        }
    }

    fn sidebar_nav_button(&mut self, ui: &mut egui::Ui, compact: &str, expanded: &str, screen: Screen) {
        let text = self.sidebar_label(compact, expanded);
        let selected = self.current_screen == screen;
        let fill = if selected {
            Color32::from_rgb(78, 28, 38)
        } else {
            Color32::TRANSPARENT
        };

        let button = egui::Button::new(text)
            .fill(fill)
            .min_size(egui::vec2(ui.available_width(), 28.0));
        if ui.add(button).clicked() {
            self.navigate_to(screen);
        }
    }

    fn sidebar_library_button(
        &mut self,
        ui: &mut egui::Ui,
        compact: &str,
        expanded: &str,
        section: LibrarySection,
    ) {
        let text = self.sidebar_label(compact, expanded);
        let selected =
            self.current_screen == Screen::Libraries && self.current_library_section == section;
        let fill = if selected {
            Color32::from_rgb(78, 28, 38)
        } else {
            Color32::TRANSPARENT
        };

        let button = egui::Button::new(text)
            .fill(fill)
            .min_size(egui::vec2(ui.available_width(), 28.0));
        if ui.add(button).clicked() {
            self.current_library_section = section;
            self.current_screen = Screen::Libraries;
            self.load_library_items(section);
        }
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
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(18, 20, 26))
                .stroke(Stroke::new(1.0, Color32::from_rgb(82, 32, 42)))
                .inner_margin(egui::Margin::symmetric(18, 16))
                .show(ui, |ui| {
                    ui.set_width(card_width - 36.0);
                    ui.heading("Connect to Jellyfin");
                    ui.label(RichText::new("Modern media shell with external playback").weak());
                    ui.add_space(10.0);

                    ui.label(RichText::new("Server URL").small().strong());
                    ui.text_edit_singleline(&mut self.config.server.base_url);

                    ui.label(RichText::new("Username").small().strong());
                    ui.text_edit_singleline(&mut self.config.server.username);

                    ui.label(RichText::new("Password").small().strong());
                    ui.add(egui::TextEdit::singleline(&mut self.login_password).password(true));

                    ui.checkbox(
                        &mut self.config.server.allow_self_signed,
                        "Allow self-signed certificates",
                    );

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save Settings").clicked() {
                            self.save_settings();
                        }
                        if ui
                            .add(
                                egui::Button::new("Login")
                                    .fill(Color32::from_rgb(122, 36, 50))
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(175, 56, 74))),
                            )
                            .clicked()
                        {
                            self.do_login();
                        }
                    });
                });
        });
    }

    fn draw_now_playing_strip(&mut self, ui: &mut egui::Ui) {
        let Some(playback) = &self.playback else {
            return;
        };

        let item_id = playback.item_id.clone();
        let status_text = playback.status_text.clone();
        let player_name = match playback.player_kind {
            super::PlayerKind::Mpv => "mpv",
            super::PlayerKind::Vlc => "VLC",
        };

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(20, 23, 30))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Now Playing").strong());
                    ui.separator();
                    ui.label(format!("Item {item_id}"));
                    ui.label(RichText::new(format!("Player {player_name}")).weak());
                    ui.label(RichText::new(status_text).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Stop Sync").clicked() {
                            self.stop_playback();
                        }
                    });
                });
            });
    }

    fn draw_screen_header(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        let compact = Self::is_compact_layout(ui);
        ui.label(RichText::new("Media Library").small().weak());
        ui.label(
            RichText::new(title)
                .size(if compact { 26.0 } else { 31.0 })
                .strong(),
        );
        ui.label(RichText::new(subtitle).small().weak());
        ui.add_space(if compact { 4.0 } else { 8.0 });
    }

    fn is_compact_layout(ui: &egui::Ui) -> bool {
        ui.available_width() < 1020.0
    }

    fn card_dimensions(compact: bool) -> (f32, Vec2) {
        if compact {
            (156.0, Vec2::new(140.0, 204.0))
        } else {
            (178.0, Vec2::new(162.0, 236.0))
        }
    }

    fn section_fade_t(ui: &egui::Ui, id: &'static str) -> f32 {
        ui.ctx()
            .animate_bool(ui.make_persistent_id(id), true)
            .clamp(0.0, 1.0)
    }

    fn show_faded_section<R>(
        ui: &mut egui::Ui,
        id: &'static str,
        offset: f32,
        alpha_max: f32,
        body: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let fade_t = Self::section_fade_t(ui, id);
        egui::Frame::group(ui.style())
            .fill(Color32::from_rgba_unmultiplied(
                18,
                20,
                26,
                (alpha_max * fade_t) as u8,
            ))
            .show(ui, |ui| {
                if fade_t < 1.0 {
                    ui.add_space((1.0 - fade_t) * offset);
                }
                body(ui)
            })
            .inner
    }

    fn draw_home(&mut self, ui: &mut egui::Ui) {
        let compact = Self::is_compact_layout(ui);
        self.draw_screen_header(
            ui,
            "Home",
            "Continue watching and newly added media across your server.",
        );

        ui.horizontal(|ui| {
            if ui.button("Refresh Home").clicked() {
                self.load_home_sections();
                self.load_last_played();
            }
        });
        ui.add_space(if compact { 3.0 } else { 6.0 });

        self.draw_home_hero(ui);
        ui.add_space(if compact { 6.0 } else { 10.0 });

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
        ui.add_space(6.0);
        self.draw_media_row(
            ui,
            "recent_movies_row",
            "Recently Added Movies",
            &recent_movies,
            false,
            Screen::Home,
        );
        ui.add_space(6.0);
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
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(RichText::new("Featured media").strong());
                ui.label(RichText::new("No hero items yet.").weak());
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
        let synopsis = item
            .overview
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "--".to_string());
        let synopsis = Self::truncate_text(&synopsis, if compact { 220 } else { 360 });

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

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(20, 22, 29))
            .stroke(Stroke::new(1.0, Color32::from_rgb(72, 28, 38)))
            .inner_margin(egui::Margin::symmetric(12, if compact { 9 } else { 11 }))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Featured").small().weak());
                    ui.separator();
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
                        .weak(),
                    );

                    if ui.button(">").clicked() {
                        self.hero_index = (self.hero_index + 1) % hero_items.len();
                    }
                });

                ui.add_space(if compact { 5.0 } else { 6.0 });

                if compact {
                    if self
                        .draw_item_image_with_size(
                            ui,
                            &item,
                            Vec2::new(image_width, image_height),
                            "Backdrop",
                        )
                        .clicked()
                    {
                        self.open_item_details(item.clone(), Screen::Home);
                    }

                    ui.add_space(6.0);
                    ui.label(RichText::new(title).strong().size(24.0));
                    ui.label(RichText::new(subtitle).small().weak());
                    ui.add(
                        egui::Label::new(RichText::new(synopsis).small())
                            .wrap()
                            .selectable(false),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add(
                                egui::Button::new("Play")
                                    .fill(Color32::from_rgb(122, 36, 50))
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(175, 56, 74))),
                            )
                            .clicked()
                        {
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
                            ui.label(RichText::new(title).strong().size(30.0));
                            ui.label(RichText::new(subtitle).weak());
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(RichText::new(synopsis).small())
                                    .wrap()
                                    .selectable(false),
                            );
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new("Play")
                                            .fill(Color32::from_rgb(122, 36, 50))
                                            .stroke(Stroke::new(1.0, Color32::from_rgb(175, 56, 74))),
                                    )
                                    .clicked()
                                {
                                    self.selected_item = Some(item.clone());
                                    self.start_playback();
                                }

                                if ui.button("Open Details").clicked() {
                                    self.open_item_details(item.clone(), Screen::Home);
                                }
                            });
                        });

                        ui.add_space(8.0);
                        if self
                            .draw_item_image_with_size(
                                ui,
                                &item,
                                Vec2::new(image_width, image_height),
                                "Backdrop",
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
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong());
            ui.label(RichText::new(format!("{} items", items.len())).weak());
        });

        if items.is_empty() {
            ui.label(RichText::new("Nothing to show yet.").weak());
            return;
        }

        let reveal = ui
            .ctx()
            .animate_bool(ui.make_persistent_id((row_id, "visible")), !items.is_empty());
        if reveal < 1.0 {
            ui.add_space((1.0 - reveal) * 8.0);
        }

        let compact = Self::is_compact_layout(ui);
        egui::ScrollArea::horizontal()
            .id_salt(row_id)
            .max_height(if compact { 286.0 } else { 332.0 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for item in items {
                        let clicked = self.draw_media_card(ui, item, show_progress);
                        if clicked {
                            self.open_item_details(item.clone(), return_screen);
                        }
                        ui.add_space(6.0);
                    }
                });
            });
    }

    fn draw_media_card(&mut self, ui: &mut egui::Ui, item: &BaseItemDto, show_progress: bool) -> bool {
        let compact = Self::is_compact_layout(ui);
        let (card_width, image_size) = Self::card_dimensions(compact);
        let mut clicked = false;

        let frame = egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(18, 21, 27))
            .stroke(Stroke::new(1.0, Color32::from_rgb(43, 48, 58)))
            .inner_margin(egui::Margin::symmetric(8, 8))
            .show(ui, |ui| {
                ui.set_width(card_width);

                if self
                    .draw_item_image_with_size(ui, item, image_size, "Primary")
                    .clicked()
                {
                    clicked = true;
                }

                let title = item.name.clone().unwrap_or_else(|| "Untitled".to_string());
                if ui.link(title).clicked() {
                    clicked = true;
                }
                ui.label(RichText::new(self.item_subtitle(item)).weak().small());

                if show_progress {
                    if let Some(progress) = Self::item_progress(item) {
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .desired_width(image_size.x)
                                .show_percentage(),
                        );
                    }
                }
            });

        if frame.response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let hover_t = ui.ctx().animate_bool(
            frame.response.id.with("card_hover"),
            frame.response.hovered() || frame.response.has_focus(),
        );
        if hover_t > 0.0 {
            let alpha = (hover_t * 90.0).round().clamp(0.0, 255.0) as u8;
            ui.painter().rect_stroke(
                frame.response.rect.expand(1.5),
                egui::CornerRadius::same(8),
                Stroke::new(1.0 + hover_t, Color32::from_rgba_unmultiplied(196, 62, 82, alpha)),
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
            ui.label(RichText::new("No items found.").weak());
            return;
        }

        egui::ScrollArea::vertical().id_salt(grid_id).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for item in items {
                    if self.draw_media_card(ui, item, false) {
                        self.open_item_details(item.clone(), return_screen);
                    }
                    ui.add_space(6.0);
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
            return ui.add_sized(size, egui::Label::new(RichText::new("No image").weak()));
        };

        let width = size.x.max(16.0) as u32;
        let height = size.y.max(16.0) as u32;
        let tag = SlimJellyApp::image_tag_for_item(item).map(str::to_owned);
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
            ui.add_sized(size, egui::Label::new(RichText::new("No image").weak()))
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

        ui.horizontal(|ui| {
            ui.label("View");
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
                            ui.selectable_value(&mut self.selected_view_id, Some(id.clone()), label);
                        }
                    }
                });
        });

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.search_term)
                    .hint_text("Search movies, shows, audio, collections..."),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.search_items();
            }

            if ui.button("Run").clicked() {
                self.search_items();
            }
        });

        if !self.search_hints.is_empty() {
            let hints = self.search_hints.clone();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Hints:").weak());
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

        ui.add_space(6.0);
        let items = self.items.clone();
        self.draw_media_grid(ui, "search_grid", &items, Screen::Search);
    }

    fn draw_libraries(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(
            ui,
            "Libraries",
            "Browse all libraries by media type across your server.",
        );

        ui.horizontal(|ui| {
            self.library_tab_button(ui, LibrarySection::Movies, "Movies");
            self.library_tab_button(ui, LibrarySection::TvShows, "TV Shows");
            self.library_tab_button(ui, LibrarySection::Music, "Music");
            self.library_tab_button(ui, LibrarySection::Audiobooks, "Audiobooks");
        });

        ui.add_space(6.0);
        let items = self.library_items.clone();
        self.draw_media_grid(ui, "library_grid", &items, Screen::Libraries);
    }

    fn library_tab_button(&mut self, ui: &mut egui::Ui, section: LibrarySection, label: &str) {
        let selected = self.current_library_section == section;
        let response = ui.selectable_label(selected, label);
        if response.clicked() && !selected {
            self.current_library_section = section;
            self.load_library_items(section);
        }
    }

    fn draw_collections(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(ui, "Collections", "Grouped media sets and curated bundles.");
        ui.horizontal(|ui| {
            if ui.button("Reload Collections").clicked() {
                self.load_collections();
            }
        });
        ui.add_space(6.0);

        let items = self.collection_items.clone();
        self.draw_media_grid(ui, "collections_grid", &items, Screen::Collections);
    }

    fn draw_playlists_screen(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(ui, "Playlists", "Open playlists and browse their media items.");

        ui.horizontal(|ui| {
            if ui.button("Reload Playlists").clicked() {
                self.load_playlists();
            }
        });

        let playlists = self.playlists.clone();
        if playlists.is_empty() {
            ui.label(RichText::new("No playlists found.").weak());
            return;
        }

        egui::ScrollArea::horizontal()
            .id_salt("playlist_strip")
            .max_height(300.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for playlist in playlists {
                        let Some(playlist_id) = playlist.id.clone() else {
                            continue;
                        };

                        let selected =
                            self.selected_playlist_id.as_deref() == Some(playlist_id.as_str());
                        let stroke = if selected {
                            Stroke::new(1.4, Color32::from_rgb(148, 45, 63))
                        } else {
                            Stroke::new(1.0, Color32::from_rgb(45, 50, 60))
                        };

                        egui::Frame::group(ui.style())
                            .stroke(stroke)
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
                                ui.label(RichText::new("Playlist").weak());
                            });

                        ui.add_space(6.0);
                    }
                });
            });

        ui.add_space(8.0);
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

        ui.horizontal(|ui| {
            if ui.button("< Back").clicked() {
                self.current_screen = self.detail_return_screen;
            }

            ui.separator();
            ui.label(RichText::new("Details").strong());
        });
        ui.add_space(6.0);

        let compact = Self::is_compact_layout(ui);
        Self::show_faded_section(
            ui,
            "details_header_section",
            if compact { 8.0 } else { 12.0 },
            235.0,
            |ui| {
                egui::Frame::group(ui.style())
                    .stroke(Stroke::new(1.0, Color32::from_rgb(70, 30, 40)))
                    .inner_margin(egui::Margin::symmetric(12, if compact { 8 } else { 10 }))
                    .show(ui, |ui| {
                        if compact {
                            if self
                                .draw_item_image_with_size(ui, &item, Vec2::new(220.0, 308.0), "Primary")
                                .clicked()
                            {
                                self.selected_item = Some(item.clone());
                            }
                            ui.add_space(8.0);
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
            },
        );

        ui.add_space(if compact { 6.0 } else { 8.0 });
        Self::show_faded_section(ui, "details_synopsis_section", 5.0, 220.0, |ui| {
            ui.label(RichText::new("Synopsis").strong());
            let synopsis = item
                .overview
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "--".to_string());
            ui.label(synopsis);
        });

        ui.add_space(if compact { 5.0 } else { 6.0 });
        self.draw_detail_seasons_section(ui, &item);
        ui.add_space(if compact { 5.0 } else { 6.0 });
        self.draw_detail_cast_section(ui, &item);
        ui.add_space(if compact { 5.0 } else { 6.0 });
        self.draw_detail_related_section(ui);
    }

    fn draw_details_header_text(&mut self, ui: &mut egui::Ui, item: &BaseItemDto, compact: bool) {
        let title = item.name.clone().unwrap_or_else(|| "Untitled".to_string());
        ui.label(
            RichText::new(title)
                .size(if compact { 24.0 } else { 30.0 })
                .strong(),
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
        let age = item.official_rating.clone().unwrap_or_else(|| "--".to_string());

        if compact {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("Year {year}")).weak());
                ui.separator();
                ui.label(RichText::new(format!("Rating {rating}")).weak());
                ui.separator();
                ui.label(RichText::new(format!("Duration {duration}")).weak());
                ui.separator();
                ui.label(RichText::new(format!("Age {age}")).weak());
            });
        } else {
            ui.label(format!("Year: {year} | Rating: {rating} | Duration: {duration} | Age: {age}"));
        }

        ui.label(RichText::new(self.detail_tech_summary()).weak());

        ui.add_space(if compact { 6.0 } else { 8.0 });
        if compact {
            ui.horizontal_wrapped(|ui| {
                self.draw_details_action_buttons(ui, item);
            });
        } else {
            ui.horizontal(|ui| {
                self.draw_details_action_buttons(ui, item);
            });
        }
    }

    fn draw_details_action_buttons(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        let can_resume = Self::item_progress(item).map(|v| v > 0.0).unwrap_or(false);
        let play_label = if can_resume { "Resume" } else { "Play" };
        if ui
            .add(
                egui::Button::new(play_label)
                    .fill(Color32::from_rgb(122, 36, 50))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(175, 56, 74))),
            )
            .clicked()
        {
            self.selected_item = Some(item.clone());
            self.start_playback();
        }

        if ui.button("Mark Played").clicked() {
            self.selected_item = Some(item.clone());
            self.mark_selected_item_played();
        }

        ui.add_enabled(false, egui::Button::new("Shuffle"));
        ui.add_enabled(false, egui::Button::new("Add to Playlist"));
        ui.add_enabled(false, egui::Button::new("More"));
    }

    fn draw_detail_seasons_section(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        let compact = Self::is_compact_layout(ui);
        Self::show_faded_section(
            ui,
            "details_seasons_section",
            if compact { 4.0 } else { 6.0 },
            215.0,
            |ui| {
            ui.label(RichText::new("Seasons & Episodes").strong());

            let is_series = item
                .r#type
                .as_deref()
                .map(|item_type| item_type.eq_ignore_ascii_case("Series"))
                .unwrap_or(false);

            if !is_series {
                ui.label(RichText::new("--").weak());
                return;
            }

            if self.detail_seasons.is_empty() {
                ui.label(RichText::new("No seasons found").weak());
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
                            let selected =
                                self.detail_selected_season_id.as_deref() == Some(season_id.as_str());

                            if ui.selectable_label(selected, label).clicked() {
                                self.choose_detail_season(season_id);
                            }
                        }
                    });
            });

            if self.detail_episodes.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new("No episodes found").weak());
                return;
            }

            let episodes = self.detail_episodes.clone();
            egui::ScrollArea::vertical()
                .id_salt("detail_episodes_list")
                .max_height(280.0)
                .show(ui, |ui| {
                    for episode in &episodes {
                        egui::Frame::group(ui.style())
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
                                        if ui.link(episode_title).clicked() {
                                            self.open_item_details(
                                                episode.clone(),
                                                self.detail_return_screen,
                                            );
                                        }

                                        ui.label(RichText::new(Self::episode_subtitle(episode)).weak());

                                        let overview = episode
                                            .overview
                                            .as_deref()
                                            .map(str::trim)
                                            .filter(|value| !value.is_empty())
                                            .map(|value| {
                                                let mut text = value.chars().take(170).collect::<String>();
                                                if value.chars().count() > 170 {
                                                    text.push_str("...");
                                                }
                                                text
                                            })
                                            .unwrap_or_else(|| "--".to_string());
                                        ui.label(RichText::new(overview).small().weak());
                                    });
                                });
                            });
                        ui.add_space(4.0);
                    }
                });
            },
        );
    }

    fn draw_detail_cast_section(&mut self, ui: &mut egui::Ui, item: &BaseItemDto) {
        Self::show_faded_section(ui, "details_cast_section", 5.0, 215.0, |ui| {
            ui.label(RichText::new("Cast & Crew").strong());

            let Some(people) = item.people.as_ref() else {
                ui.label(RichText::new("--").weak());
                return;
            };
            if people.is_empty() {
                ui.label(RichText::new("--").weak());
                return;
            }

            egui::ScrollArea::horizontal()
                .id_salt("detail_people_row")
                .max_height(118.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for person in people.iter().take(20) {
                            egui::Frame::group(ui.style())
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
                                    ui.label(RichText::new(role).weak());

                                    if ui.button("Search").clicked() {
                                        self.search_term = name;
                                        self.navigate_to(Screen::Search);
                                        self.search_items();
                                    }
                                });

                            ui.add_space(6.0);
                        }
                    });
                });
        });
    }

    fn draw_detail_related_section(&mut self, ui: &mut egui::Ui) {
        Self::show_faded_section(ui, "details_related_section", 5.0, 215.0, |ui| {
            ui.label(RichText::new("More Like This").strong());

            if self.detail_related.is_empty() {
                ui.label(RichText::new("--").weak());
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
                            ui.add_space(6.0);
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
            ui.label("Admin panel hidden for non-admin account.");
            return;
        }

        self.draw_screen_header(ui, "Dashboard", "Server admin controls and scheduled tasks.");

        ui.horizontal(|ui| {
            if ui.button("Scan All Libraries").clicked() {
                self.trigger_scan_all();
            }
            if ui.button("Reload Tasks").clicked() {
                self.refresh_tasks();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Library or Item ID");
            ui.text_edit_singleline(&mut self.selected_library_id);
            if ui.button("Refresh One").clicked() {
                self.trigger_refresh_item();
            }
        });

        ui.separator();
        ui.label(RichText::new("Scheduled Tasks").strong());
        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            for task in &self.tasks {
                let name = task.name.clone().unwrap_or_else(|| "Unnamed task".to_string());
                let state = task.state.clone().unwrap_or_else(|| "unknown".to_string());
                let progress = task
                    .current_progress_percentage
                    .map(|v| format!("{v:.0}%"))
                    .unwrap_or_else(|| "n/a".to_string());
                ui.label(format!("{name} | {state} | {progress}"));
            }
        });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        self.draw_screen_header(ui, "Settings", "Playback and client behavior.");

        ui.horizontal(|ui| {
            ui.label("Preferred Player");
            egui::ComboBox::from_id_salt("preferred_player")
                .selected_text(match self.config.player.preferred {
                    PreferredPlayer::Mpv => "mpv",
                    PreferredPlayer::Vlc => "VLC",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.player.preferred, PreferredPlayer::Mpv, "mpv");
                    ui.selectable_value(&mut self.config.player.preferred, PreferredPlayer::Vlc, "VLC");
                });
        });

        ui.horizontal(|ui| {
            ui.label("mpv path (optional)");
            let mut value = self.config.player.mpv_path.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut value).changed() {
                let trimmed = value.trim().to_string();
                self.config.player.mpv_path = if trimmed.is_empty() { None } else { Some(trimmed) };
            }
        });

        ui.horizontal(|ui| {
            ui.label("VLC path (optional)");
            let mut value = self.config.player.vlc_path.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut value).changed() {
                let trimmed = value.trim().to_string();
                self.config.player.vlc_path = if trimmed.is_empty() { None } else { Some(trimmed) };
            }
        });

        ui.horizontal(|ui| {
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

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.config.server.allow_self_signed, "Allow self-signed certs");
            ui.checkbox(&mut self.config.playback.direct_first, "Direct play first");
            ui.checkbox(&mut self.config.playback.fallback_once, "Transcode fallback once");
        });

        if ui.button("Save Settings").clicked() {
            self.save_settings();
        }
    }
}
