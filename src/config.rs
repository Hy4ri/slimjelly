use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

const CONFIG_FILE_NAME: &str = "config.toml";
const SESSION_FILE_NAME: &str = "session.enc";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub session_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub client: ClientConfig,
    pub server: ServerConfig,
    pub player: PlayerConfig,
    pub playback: PlaybackConfig,
    pub subtitles: SubtitleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClientConfig {
    pub app_name: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub base_url: String,
    pub username: String,
    pub allow_self_signed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerConfig {
    pub preferred: PreferredPlayer,
    pub mpv_path: Option<String>,
    pub vlc_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreferredPlayer {
    Mpv,
    Vlc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackConfig {
    pub direct_first: bool,
    pub fallback_once: bool,
    pub resume_policy: ResumePolicy,
    pub sync_mode: SyncMode,
    pub base_sync_interval_seconds: u64,
    pub remember_track_choice: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResumePolicy {
    AlwaysResume,
    Ask,
    StartOver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleConfig {
    pub api_key: String,
    pub username: String,
    pub password: String,
    pub default_language: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    Adaptive,
    Fixed,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client: ClientConfig::default(),
            server: ServerConfig::default(),
            player: PlayerConfig::default(),
            playback: PlaybackConfig::default(),
            subtitles: SubtitleConfig::default(),
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            app_name: "slimjelly".to_string(),
            device_id: Uuid::new_v4().to_string(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            username: String::new(),
            allow_self_signed: false,
        }
    }
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            preferred: PreferredPlayer::Mpv,
            mpv_path: None,
            vlc_path: None,
        }
    }
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            direct_first: true,
            fallback_once: true,
            resume_policy: ResumePolicy::AlwaysResume,
            sync_mode: SyncMode::Adaptive,
            base_sync_interval_seconds: 15,
            remember_track_choice: true,
        }
    }
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            username: String::new(),
            password: String::new(),
            default_language: "en".to_string(),
        }
    }
}

pub fn load_or_create() -> Result<(AppConfig, AppPaths), AppError> {
    let project_dirs =
        ProjectDirs::from("io", "slimjelly", "slimjelly").ok_or(AppError::ConfigDirUnavailable)?;

    let paths = AppPaths {
        config_dir: project_dirs.config_dir().to_path_buf(),
        data_dir: project_dirs.data_dir().to_path_buf(),
        config_file: project_dirs.config_dir().join(CONFIG_FILE_NAME),
        session_file: project_dirs.data_dir().join(SESSION_FILE_NAME),
    };

    load_or_create_from_paths(paths)
}

fn load_or_create_from_paths(paths: AppPaths) -> Result<(AppConfig, AppPaths), AppError> {
    fs::create_dir_all(&paths.config_dir)?;
    fs::create_dir_all(&paths.data_dir)?;

    let config = if paths.config_file.exists() {
        let raw = fs::read_to_string(&paths.config_file)?;
        toml::from_str::<AppConfig>(&raw)?
    } else {
        let cfg = AppConfig::default();
        save_config(&paths, &cfg)?;
        cfg
    };

    Ok((config, paths))
}

pub fn save_config(paths: &AppPaths, config: &AppConfig) -> Result<(), AppError> {
    let output = toml::to_string_pretty(config)?;
    fs::write(&paths.config_file, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use uuid::Uuid;

    use super::*;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("slimjelly-config-tests")
            .join(format!("{name}-{}", Uuid::new_v4()))
    }

    fn test_paths(root: &PathBuf) -> AppPaths {
        let config_dir = root.join("config");
        let data_dir = root.join("data");

        AppPaths {
            config_dir: config_dir.clone(),
            data_dir: data_dir.clone(),
            config_file: config_dir.join(CONFIG_FILE_NAME),
            session_file: data_dir.join(SESSION_FILE_NAME),
        }
    }

    fn cleanup(root: &PathBuf) {
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_or_create_creates_default_config_and_dirs() -> Result<(), AppError> {
        let root = test_root("create-default");
        let paths = test_paths(&root);

        let (config, returned_paths) = load_or_create_from_paths(paths.clone())?;

        assert!(paths.config_dir.exists());
        assert!(paths.data_dir.exists());
        assert!(paths.config_file.exists());
        assert_eq!(returned_paths.config_file, paths.config_file);
        assert_eq!(returned_paths.session_file, paths.session_file);
        assert_eq!(config.client.app_name, "slimjelly");
        assert!(config.playback.direct_first);
        assert!(config.playback.fallback_once);
        assert_eq!(config.playback.base_sync_interval_seconds, 15);

        cleanup(&root);
        Ok(())
    }

    #[test]
    fn load_or_create_reads_existing_config_file() -> Result<(), AppError> {
        let root = test_root("read-existing");
        let paths = test_paths(&root);

        fs::create_dir_all(&paths.config_dir)?;
        fs::create_dir_all(&paths.data_dir)?;

        let raw = r#"
[client]
app_name = "myjelly"
device_id = "device-123"

[server]
base_url = "https://example.com"
username = "alice"
allow_self_signed = true

[player]
preferred = "vlc"
mpv_path = "/usr/bin/mpv"
vlc_path = "/usr/bin/vlc"

[playback]
direct_first = false
fallback_once = false
resume_policy = "ask"
sync_mode = "fixed"
base_sync_interval_seconds = 30
remember_track_choice = false
"#;

        fs::write(&paths.config_file, raw)?;

        let (config, _) = load_or_create_from_paths(paths.clone())?;

        assert_eq!(config.client.app_name, "myjelly");
        assert_eq!(config.client.device_id, "device-123");
        assert_eq!(config.server.base_url, "https://example.com");
        assert_eq!(config.server.username, "alice");
        assert!(config.server.allow_self_signed);
        assert_eq!(config.player.preferred, PreferredPlayer::Vlc);
        assert_eq!(config.player.mpv_path.as_deref(), Some("/usr/bin/mpv"));
        assert_eq!(config.player.vlc_path.as_deref(), Some("/usr/bin/vlc"));
        assert!(!config.playback.direct_first);
        assert!(!config.playback.fallback_once);
        assert_eq!(config.playback.resume_policy, ResumePolicy::Ask);
        assert_eq!(config.playback.sync_mode, SyncMode::Fixed);
        assert_eq!(config.playback.base_sync_interval_seconds, 30);
        assert!(!config.playback.remember_track_choice);

        cleanup(&root);
        Ok(())
    }

    #[test]
    fn load_or_create_returns_error_for_invalid_toml() -> Result<(), AppError> {
        let root = test_root("invalid-toml");
        let paths = test_paths(&root);

        fs::create_dir_all(&paths.config_dir)?;
        fs::write(&paths.config_file, "[client\n")?;

        let result = load_or_create_from_paths(paths.clone());
        assert!(matches!(result, Err(AppError::TomlDeserialize(_))));

        cleanup(&root);
        Ok(())
    }

    #[test]
    fn save_config_persists_and_roundtrips() -> Result<(), AppError> {
        let root = test_root("save-roundtrip");
        let paths = test_paths(&root);

        fs::create_dir_all(&paths.config_dir)?;
        fs::create_dir_all(&paths.data_dir)?;

        let mut config = AppConfig::default();
        config.server.base_url = "https://media.local".to_string();
        config.server.username = "bob".to_string();
        config.playback.base_sync_interval_seconds = 22;
        config.player.preferred = PreferredPlayer::Vlc;

        save_config(&paths, &config)?;

        let raw = fs::read_to_string(&paths.config_file)?;
        let loaded: AppConfig = toml::from_str(&raw)?;

        assert_eq!(loaded.server.base_url, "https://media.local");
        assert_eq!(loaded.server.username, "bob");
        assert_eq!(loaded.playback.base_sync_interval_seconds, 22);
        assert_eq!(loaded.player.preferred, PreferredPlayer::Vlc);

        cleanup(&root);
        Ok(())
    }

    #[test]
    fn default_config_serializes_expected_keys() -> Result<(), AppError> {
        let raw = toml::to_string_pretty(&AppConfig::default())?;

        assert!(raw.contains("[client]"));
        assert!(raw.contains("app_name = \"slimjelly\""));
        assert!(raw.contains("[server]"));
        assert!(raw.contains("allow_self_signed = false"));
        assert!(raw.contains("[playback]"));
        assert!(raw.contains("resume_policy = \"always-resume\""));
        assert!(raw.contains("sync_mode = \"adaptive\""));

        Ok(())
    }
}
