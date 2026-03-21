mod actions;
mod playback;
mod ui;

use std::sync::{
    Arc, Mutex,
    atomic::AtomicU64,
};

use eframe::egui;
use tokio::runtime::Runtime;

use crate::{
    config::{AppConfig, AppPaths},
    error::AppError,
    jellyfin::{
        JellyfinClient,
        models::{
            BaseItemDto, BaseItemDtoQueryResult, MediaSourceInfo, PublicSystemInfo, SearchHint,
            TaskInfo,
        },
    },
    secure_store::load_session,
};

const MPV_IPC_SOCKET_PATH: &str = "/tmp/slimjelly-mpv.sock";
const QUICK_EXIT_FALLBACK_SECONDS: u64 = 5;
const MPV_IPC_TIMEOUT_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
    Mpv,
    Vlc,
}

#[derive(Debug, Clone, Copy)]
struct MpvPlaybackSnapshot {
    position_secs: f64,
    is_paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Login,
    Home,
    Search,
    Libraries,
    Collections,
    Playlists,
    Admin,
    Settings,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibrarySection {
    Movies,
    TvShows,
    Music,
    Audiobooks,
}

#[derive(Debug, Clone)]
enum UiMessage {
    SessionValidated {
        user_name: String,
        is_admin: bool,
    },
    SessionValidationFailed(String),
    LoggedIn {
        token: String,
        user_id: String,
        user_name: String,
        is_admin: bool,
        server_id: Option<String>,
    },
    LoginFailed(String),
    HealthResult {
        ping: String,
        info: PublicSystemInfo,
    },
    HealthFailed(String),
    ViewsLoaded(BaseItemDtoQueryResult),
    SearchHintsLoaded(Vec<SearchHint>),
    SearchLoaded(BaseItemDtoQueryResult),
    SearchFailed(String),
    HomeContinueWatchingLoaded(Vec<BaseItemDto>),
    HomeRecentMoviesLoaded(Vec<BaseItemDto>),
    HomeRecentSeriesLoaded(Vec<BaseItemDto>),
    HomeLoadFailed(String),
    LibraryItemsLoaded {
        section: LibrarySection,
        items: Vec<BaseItemDto>,
    },
    LibraryItemsFailed(String),
    CollectionItemsLoaded(Vec<BaseItemDto>),
    CollectionItemsFailed(String),
    DetailSeasonsLoaded(Vec<BaseItemDto>),
    DetailSeasonsFailed(String),
    DetailEpisodesLoaded {
        season_id: String,
        items: Vec<BaseItemDto>,
    },
    DetailEpisodesFailed(String),
    DetailRelatedLoaded(Vec<BaseItemDto>),
    DetailRelatedFailed(String),
    DetailTechLoaded(Option<MediaSourceInfo>),
    DetailTechFailed(String),
    PlaylistsLoaded(Vec<BaseItemDto>),
    PlaylistsFailed(String),
    PlaylistItemsLoaded {
        playlist_id: String,
        items: Vec<BaseItemDto>,
    },
    PlaylistItemsFailed(String),
    LastPlayedLoaded(Option<BaseItemDto>),
    LastPlayedFailed(String),
    ThumbnailLoaded {
        key: String,
        bytes: Vec<u8>,
    },
    ThumbnailFailed {
        key: String,
        reason: String,
    },
    ItemLoaded(BaseItemDto),
    ItemFailed(String),
    PlaybackPrepared {
        item_id: String,
        stream_url: String,
        transcode_stream_url: Option<String>,
        used_transcode: bool,
        media_source_id: Option<String>,
        play_session_id: Option<String>,
    },
    PlaybackPrepareFailed(String),
    PlayerExited {
        generation: u64,
        elapsed_seconds: u64,
    },
    ProgressTick {
        position_ticks: i64,
        is_paused: bool,
    },
    ProgressFailed(String),
    PlaybackStopped,
    TasksLoaded(Vec<TaskInfo>),
    TasksFailed(String),
    ActionDone(String),
    ActionFailed(String),
    MarkPlayedDone {
        item_id: String,
    },
    MarkPlayedFailed(String),
}

#[derive(Debug, Clone)]
struct SessionView {
    user_id: String,
    user_name: String,
    is_admin: bool,
}

#[derive(Debug, Clone)]
struct PlaybackView {
    generation: u64,
    item_id: String,
    player_kind: PlayerKind,
    mpv_socket_path: Option<String>,
    used_transcode: bool,
    transcode_stream_url: Option<String>,
    media_source_id: Option<String>,
    play_session_id: Option<String>,
    position_ticks: i64,
    is_paused: bool,
    status_text: String,
}

pub struct SlimJellyApp {
    runtime: Arc<Runtime>,
    config: AppConfig,
    paths: AppPaths,
    client: Arc<Mutex<Option<JellyfinClient>>>,
    messages: Arc<Mutex<Vec<UiMessage>>>,
    progress_generation: Arc<AtomicU64>,

    current_screen: Screen,
    detail_return_screen: Screen,
    current_library_section: LibrarySection,
    sidebar_expanded: bool,
    sidebar_width: f32,
    hero_index: usize,
    login_password: String,
    status_line: String,
    session: Option<SessionView>,

    health_ping: Option<String>,
    health_info: Option<PublicSystemInfo>,

    selected_view_id: Option<String>,
    views: Vec<BaseItemDto>,
    library_items: Vec<BaseItemDto>,
    collection_items: Vec<BaseItemDto>,
    home_continue_watching: Vec<BaseItemDto>,
    home_recent_movies: Vec<BaseItemDto>,
    home_recent_series: Vec<BaseItemDto>,
    detail_seasons: Vec<BaseItemDto>,
    detail_selected_season_id: Option<String>,
    detail_episodes: Vec<BaseItemDto>,
    detail_related: Vec<BaseItemDto>,
    detail_media_source: Option<MediaSourceInfo>,
    playlists: Vec<BaseItemDto>,
    selected_playlist_id: Option<String>,
    playlist_items: Vec<BaseItemDto>,
    last_played_item: Option<BaseItemDto>,
    search_term: String,
    search_hints: Vec<SearchHint>,
    items: Vec<BaseItemDto>,
    selected_item: Option<BaseItemDto>,

    thumbnail_textures: std::collections::HashMap<String, egui::TextureHandle>,
    thumbnail_images: std::collections::HashMap<String, egui::ColorImage>,
    thumbnail_pending: std::collections::HashSet<String>,
    thumbnail_failed: std::collections::HashSet<String>,

    tasks: Vec<TaskInfo>,
    selected_library_id: String,

    playback: Option<PlaybackView>,
}

impl SlimJellyApp {
    pub fn new(runtime: Arc<Runtime>, config: AppConfig, paths: AppPaths) -> Self {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let mut app = Self {
            runtime,
            config,
            paths,
            client: Arc::new(Mutex::new(None)),
            messages,
            progress_generation: Arc::new(AtomicU64::new(0)),
            current_screen: Screen::Login,
            detail_return_screen: Screen::Home,
            current_library_section: LibrarySection::Movies,
            sidebar_expanded: false,
            sidebar_width: 68.0,
            hero_index: 0,
            login_password: String::new(),
            status_line: "Ready".to_string(),
            session: None,
            health_ping: None,
            health_info: None,
            selected_view_id: None,
            views: Vec::new(),
            library_items: Vec::new(),
            collection_items: Vec::new(),
            home_continue_watching: Vec::new(),
            home_recent_movies: Vec::new(),
            home_recent_series: Vec::new(),
            detail_seasons: Vec::new(),
            detail_selected_season_id: None,
            detail_episodes: Vec::new(),
            detail_related: Vec::new(),
            detail_media_source: None,
            playlists: Vec::new(),
            selected_playlist_id: None,
            playlist_items: Vec::new(),
            last_played_item: None,
            search_term: String::new(),
            search_hints: Vec::new(),
            items: Vec::new(),
            selected_item: None,
            thumbnail_textures: std::collections::HashMap::new(),
            thumbnail_images: std::collections::HashMap::new(),
            thumbnail_pending: std::collections::HashSet::new(),
            thumbnail_failed: std::collections::HashSet::new(),
            tasks: Vec::new(),
            selected_library_id: String::new(),
            playback: None,
        };

        app.try_restore_session();
        app
    }

    fn try_restore_session(&mut self) {
        if let Ok(Some(token)) = load_session(&self.paths.session_file) {
            match self.build_client_with_token(token.access_token.clone()) {
                Ok(client) => {
                    self.client = Arc::new(Mutex::new(Some(client)));
                    self.session = Some(SessionView {
                        user_id: token.user_id,
                        user_name: self.config.server.username.clone(),
                        is_admin: false,
                    });
                    self.current_screen = Screen::Home;
                    self.status_line = "Session restored".to_string();
                    self.validate_restored_session();
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
                    self.status_line = format!("Failed to restore session: {err}");
                }
            }
        }
    }

    fn validate_restored_session(&mut self) {
        let Some(client) = self.active_client() else {
            return;
        };

        let messages = self.messages.clone();
        self.runtime.spawn(async move {
            match client.get_me().await {
                Ok(user) => {
                    let user_name = user.name.unwrap_or_else(|| "unknown".to_string());
                    let is_admin = user
                        .policy
                        .as_ref()
                        .and_then(|p| p.is_administrator)
                        .unwrap_or(false);
                    Self::push_message(
                        &messages,
                        UiMessage::SessionValidated { user_name, is_admin },
                    );
                }
                Err(err) => {
                    Self::push_message(
                        &messages,
                        UiMessage::SessionValidationFailed(err.to_string()),
                    );
                }
            }
        });
    }

    fn build_client(&self) -> Result<JellyfinClient, AppError> {
        JellyfinClient::new(&self.config.server, self.config.client.device_id.clone())
    }

    fn build_client_with_token(&self, token: String) -> Result<JellyfinClient, AppError> {
        let mut client = self.build_client()?;
        client.set_token(Some(token));
        Ok(client)
    }

    fn describe_player_kind(player_kind: PlayerKind) -> &'static str {
        match player_kind {
            PlayerKind::Mpv => "mpv",
            PlayerKind::Vlc => "VLC",
        }
    }

    fn push_message(messages: &Arc<Mutex<Vec<UiMessage>>>, msg: UiMessage) {
        if let Ok(mut queue) = messages.lock() {
            queue.push(msg);
        }
    }

    fn take_messages(&self) -> Vec<UiMessage> {
        if let Ok(mut queue) = self.messages.lock() {
            return std::mem::take(&mut *queue);
        }
        Vec::new()
    }

    fn active_client(&self) -> Option<JellyfinClient> {
        self.client.lock().ok()?.as_ref().cloned()
    }

    fn image_tag_for_item(item: &BaseItemDto) -> Option<&str> {
        item.primary_image_tag
            .as_deref()
            .or_else(|| item.image_tags.as_ref().and_then(|tags| tags.primary.as_deref()))
            .or_else(|| item.image_tags.as_ref().and_then(|tags| tags.thumb.as_deref()))
    }

    fn thumbnail_key(
        item_id: &str,
        width: u32,
        height: u32,
        image_type: &str,
        tag: Option<&str>,
    ) -> String {
        format!(
            "{item_id}|{width}x{height}|{image_type}|{}",
            tag.unwrap_or("untagged")
        )
    }

    fn should_retry_transcode(playback: &PlaybackView, elapsed_seconds: u64) -> bool {
        !playback.used_transcode
            && playback.transcode_stream_url.is_some()
            && playback.position_ticks == 0
            && elapsed_seconds <= QUICK_EXIT_FALLBACK_SECONDS
    }

    fn stop_ticks_for_exit(playback: &PlaybackView, elapsed_seconds: u64) -> i64 {
        if playback.position_ticks > 0 {
            playback.position_ticks
        } else {
            (elapsed_seconds as i64).saturating_mul(10_000_000)
        }
    }
}

impl eframe::App for SlimJellyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.handle_messages();

        if self.current_screen == Screen::Login {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.draw_login(ui);
            });
        } else {
            self.draw_app_shell(ctx);
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(&self.status_line).small().weak());
            ui.add_space(2.0);
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(80));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playback_fixture() -> PlaybackView {
        PlaybackView {
            generation: 1,
            item_id: "item-1".to_string(),
            player_kind: PlayerKind::Mpv,
            mpv_socket_path: None,
            used_transcode: false,
            transcode_stream_url: Some("https://server/transcode".to_string()),
            media_source_id: Some("source-1".to_string()),
            play_session_id: Some("play-1".to_string()),
            position_ticks: 0,
            is_paused: false,
            status_text: "Playing".to_string(),
        }
    }

    #[test]
    fn retry_transcode_when_quick_exit_and_no_progress() {
        let playback = playback_fixture();
        assert!(SlimJellyApp::should_retry_transcode(&playback, QUICK_EXIT_FALLBACK_SECONDS));
    }

    #[test]
    fn no_retry_when_already_transcoding() {
        let mut playback = playback_fixture();
        playback.used_transcode = true;
        assert!(!SlimJellyApp::should_retry_transcode(&playback, 1));
    }

    #[test]
    fn no_retry_when_playback_has_progress() {
        let mut playback = playback_fixture();
        playback.position_ticks = 42;
        assert!(!SlimJellyApp::should_retry_transcode(&playback, 1));
    }

    #[test]
    fn uses_recorded_ticks_on_exit_when_available() {
        let mut playback = playback_fixture();
        playback.position_ticks = 88_000_000;
        assert_eq!(SlimJellyApp::stop_ticks_for_exit(&playback, 3), 88_000_000);
    }

    #[test]
    fn uses_elapsed_seconds_when_no_ticks_recorded() {
        let playback = playback_fixture();
        assert_eq!(SlimJellyApp::stop_ticks_for_exit(&playback, 9), 90_000_000);
    }

    #[test]
    fn status_text_reflects_pause_and_transcode() {
        let mut playback = playback_fixture();
        assert_eq!(SlimJellyApp::status_text_for_playback(&playback), "Playing");

        playback.is_paused = true;
        assert_eq!(SlimJellyApp::status_text_for_playback(&playback), "Paused");

        playback.used_transcode = true;
        playback.is_paused = false;
        assert_eq!(
            SlimJellyApp::status_text_for_playback(&playback),
            "Playing (transcode)"
        );

        playback.is_paused = true;
        assert_eq!(
            SlimJellyApp::status_text_for_playback(&playback),
            "Paused (transcode)"
        );
    }
}
