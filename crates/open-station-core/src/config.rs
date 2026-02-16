use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub team_number: u32,
    pub use_usb: bool,
    pub dashboard_command: Option<String>,
    pub game_data: String,
    pub practice_timing: PracticeTiming,
    pub practice_audio: bool,
    pub joystick_locks: HashMap<String, u8>, // UUID → slot
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeTiming {
    pub countdown_secs: u32,
    pub auto_secs: u32,
    pub delay_secs: u32,
    pub teleop_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            team_number: 0,
            use_usb: false,
            dashboard_command: None,
            game_data: String::new(),
            practice_timing: PracticeTiming::default(),
            practice_audio: true,
            joystick_locks: HashMap::new(),
            window: WindowConfig::default(),
        }
    }
}

impl Default for PracticeTiming {
    fn default() -> Self {
        Self {
            countdown_secs: 3,
            auto_secs: 15,
            delay_secs: 1,
            teleop_secs: 135,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1000,
            height: 400,
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("open-station");
        dir
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let contents = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(Self::config_path(), contents)
    }

    pub fn load_from(path: &std::path::Path) -> Self {
        match fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save_to(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.team_number, 0);
        assert_eq!(config.practice_timing.auto_secs, 15);
        assert_eq!(config.practice_timing.teleop_secs, 135);
        assert_eq!(config.practice_timing.countdown_secs, 3);
        assert_eq!(config.practice_timing.delay_secs, 1);
        assert!(config.practice_audio);
        assert!(!config.use_usb);
        assert_eq!(config.window.width, 1000);
    }

    #[test]
    fn test_round_trip() {
        // Use tempfile for testing
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = Config {
            team_number: 1234,
            use_usb: true,
            game_data: "LRL".to_string(),
            ..Default::default()
        };
        config.joystick_locks.insert("uuid-123".to_string(), 0);

        config.save_to(&path).unwrap();
        let loaded = Config::load_from(&path);

        assert_eq!(loaded.team_number, 1234);
        assert!(loaded.use_usb);
        assert_eq!(loaded.game_data, "LRL");
        assert_eq!(loaded.joystick_locks.get("uuid-123"), Some(&0));
    }

    #[test]
    fn test_missing_file_returns_default() {
        let path = std::path::Path::new("/tmp/nonexistent_open_station_test/config.toml");
        let config = Config::load_from(path);
        assert_eq!(config.team_number, 0);
    }

    #[test]
    fn test_invalid_toml_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "this is not valid toml {{{{").unwrap();
        let config = Config::load_from(&path);
        assert_eq!(config.team_number, 0); // returns default
    }
}
