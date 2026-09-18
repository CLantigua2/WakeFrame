use std::path::PathBuf;

use wakeframe_common::AppConfig;

// Settings live in the same per-user config file consumed by the tray agent.
fn config_path() -> Option<PathBuf> {
    Some(
        PathBuf::from(std::env::var_os("APPDATA")?)
            .join("WakeFrame")
            .join("config.json"),
    )
}

// Load UI state, falling back to runtime defaults when no config exists yet.
pub(crate) fn load_config() -> AppConfig {
    config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_else(AppConfig::new)
}

// Write the complete settings document after user edits.
pub(crate) fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path().ok_or_else(|| "APPDATA is unavailable".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    std::fs::write(path, contents).map_err(|error| error.to_string())
}
