use std::{fs, path::PathBuf};

use wakeframe_common::AppConfig;

// The agent and UI share this per-user JSON settings file.
pub fn config_path() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(app_data)
            .join("WakeFrame")
            .join("config.json"),
    )
}

// Missing or invalid config falls back to defaults so the tray agent can start.
pub fn load_or_default() -> AppConfig {
    let Some(path) = config_path() else {
        return AppConfig::new();
    };

    if !path.exists() {
        return AppConfig::new();
    }

    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|_| AppConfig::new()),
        Err(_) => AppConfig::new(),
    }
}

// Persist settings after tray toggles or playback history updates.
pub fn save(config: &AppConfig) -> Result<(), String> {
    let Some(path) = config_path() else {
        return Err("APPDATA is not available on this system".to_string());
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let serialized = serde_json::to_string_pretty(config)
        .map_err(|err| format!("Failed to serialize config: {}", err))?;

    fs::write(path, serialized).map_err(|err| format!("Failed to write config: {}", err))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{load_or_default, save};
    use wakeframe_common::AppConfig;

    #[test]
    fn save_and_load_config_round_trip() {
        let mut config = AppConfig::new();
        config.enabled = false;
        config.play_random = false;
        config.video_directory = Some("D:/Videos/WakeFrame".to_string());

        let path = std::env::var_os("APPDATA").map(|app_data| {
            std::path::PathBuf::from(app_data)
                .join("WakeFrame")
                .join("config.json")
        });
        if let Some(path) = path {
            let result = save(&config);
            assert!(result.is_ok());
            let loaded = load_or_default();
            assert!(!loaded.enabled);
            assert!(!loaded.play_random);
            assert_eq!(
                loaded.video_directory.as_deref(),
                Some("D:/Videos/WakeFrame")
            );
            let _ = std::fs::remove_file(path);
        }
    }
}
