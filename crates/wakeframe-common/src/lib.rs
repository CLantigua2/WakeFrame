use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

// Recursively discover supported video files under the configured library folder.
pub fn candidate_video_paths_in_directory(dir: &str) -> Vec<String> {
    let root = Path::new(dir);
    if !root.exists() {
        return Vec::new();
    }

    let mut paths = Vec::new();
    collect_video_files(root, &mut paths);
    paths.sort();
    paths
}

fn collect_video_files(path: &Path, out: &mut Vec<String>) {
    if path.is_file() {
        if is_supported_video_file(path) {
            out.push(path.to_string_lossy().into_owned());
        }
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            collect_video_files(&child, out);
        } else if child.is_file() && is_supported_video_file(&child) {
            out.push(child.to_string_lossy().into_owned());
        }
    }
}

fn is_supported_video_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "mp4" | "mkv" | "mov" | "avi" | "webm"
    )
}

// Persisted library entry shared by the UI and background agent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoEntry {
    pub id: String,
    pub title: String,
    pub path: String,
    pub enabled: bool,
    pub duration_seconds: u32,
}

// User-editable settings stored in %APPDATA%\WakeFrame\config.json.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub enabled: bool,
    pub play_random: bool,
    pub avoid_immediate_repeats: bool,
    pub video_directory: Option<String>,
    pub display_time_seconds: u32,
    pub selected_video_ids: Vec<String>,
    pub last_played_video: Option<String>,
    pub excluded_video_ids: Vec<String>,
    pub video_order: Vec<String>,
    pub play_on_resume: bool,
    pub play_on_unlock: bool,
    pub play_on_logon: bool,
    pub play_on_startup: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AppConfig {
    // Runtime defaults also drive serde defaults for older config files.
    pub fn new() -> Self {
        Self {
            enabled: true,
            play_random: true,
            avoid_immediate_repeats: true,
            display_time_seconds: 4,
            selected_video_ids: Vec::new(),
            video_directory: None,
            last_played_video: None,
            excluded_video_ids: Vec::new(),
            video_order: Vec::new(),
            play_on_resume: true,
            play_on_unlock: true,
            play_on_logon: true,
            play_on_startup: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{AppConfig, VideoEntry};

    #[test]
    fn config_defaults_are_valid() {
        let config = AppConfig::new();
        assert!(config.enabled);
        assert!(config.play_random);
        assert!(config.avoid_immediate_repeats);
        assert!(config.play_on_resume);
        assert!(config.play_on_unlock);
        assert!(config.play_on_logon);
        assert!(!config.play_on_startup);
        assert_eq!(config.display_time_seconds, 4);
    }

    #[test]
    fn missing_config_fields_use_runtime_defaults() {
        let json = r#"{"enabled":true,"play_random":false}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();

        assert!(config.play_on_resume);
        assert!(config.play_on_unlock);
        assert!(config.play_on_logon);
        assert!(!config.play_on_startup);
    }

    #[test]
    fn video_entry_round_trips() {
        let entry = VideoEntry {
            id: "mountain-lake".to_string(),
            title: "Mountain Lake".to_string(),
            path: "C:/Videos/mountain-lake.mp4".to_string(),
            enabled: true,
            duration_seconds: 8,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let round_trip: VideoEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, round_trip);
    }

    #[test]
    fn candidate_video_paths_in_directory_finds_supported_files() {
        let root = std::env::temp_dir().join("wakeframe-common-video-tests");
        let nested = root.join("nested");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("alpha.mp4"), "video").unwrap();
        std::fs::write(nested.join("beta.mkv"), "video").unwrap();
        std::fs::write(root.join("notes.txt"), "ignore me").unwrap();

        let videos = super::candidate_video_paths_in_directory(root.to_str().unwrap());
        let file_names: Vec<String> = videos
            .iter()
            .map(|path| {
                PathBuf::from(path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert!(file_names.contains(&"alpha.mp4".to_string()));
        assert!(file_names.contains(&"beta.mkv".to_string()));
        assert!(!file_names.contains(&"notes.txt".to_string()));

        let _ = std::fs::remove_dir_all(&root);
    }
}
