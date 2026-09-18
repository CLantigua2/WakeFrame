use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    process::Command,
};

use wakeframe_common::{candidate_video_paths_in_directory, AppConfig};

#[derive(Clone, Default)]
pub(crate) struct VideoMetadata {
    pub(crate) duration_seconds: Option<u32>,
    pub(crate) thumbnail_path: Option<PathBuf>,
}

// Discover playable videos while honoring files the user removed from the library.
pub(crate) fn discover_videos(config: &AppConfig) -> Vec<String> {
    config
        .video_directory
        .as_deref()
        .map(candidate_video_paths_in_directory)
        .unwrap_or_default()
        .into_iter()
        .filter(|path| {
            !config
                .excluded_video_ids
                .iter()
                .any(|excluded| excluded == path)
        })
        .collect()
}

// Collect duration and thumbnail data for display in the settings UI.
pub(crate) fn collect_metadata(videos: &[String]) -> Vec<VideoMetadata> {
    videos.iter().map(|path| inspect_video(path)).collect()
}

// Use ffprobe/ffmpeg opportunistically; missing tools simply leave metadata empty.
fn inspect_video(path: &str) -> VideoMetadata {
    let duration_seconds = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|value| value.trim().parse::<f64>().ok())
        .map(|value| value.round() as u32);

    VideoMetadata {
        duration_seconds,
        thumbnail_path: create_thumbnail(path),
    }
}

// Cache thumbnails by hashed video path in the temp directory.
fn create_thumbnail(video: &str) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    video.hash(&mut hasher);
    let output = std::env::temp_dir()
        .join("WakeFrame")
        .join("thumbnails")
        .join(format!("{:x}.jpg", hasher.finish()));
    if output.exists() {
        return Some(output);
    }
    std::fs::create_dir_all(output.parent()?).ok()?;
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-ss",
            "1",
            "-i",
            video,
            "-frames:v",
            "1",
            "-vf",
            "scale=180:-1",
        ])
        .arg(&output)
        .status()
        .ok()?;
    status.success().then_some(output)
}

pub(crate) fn format_duration(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

// Keep videos and their checked states aligned while applying saved ordering.
pub(crate) fn order_videos(videos: &mut Vec<String>, selected: &mut Vec<bool>, order: &[String]) {
    if order.is_empty() {
        return;
    }
    let mut ordered = Vec::with_capacity(videos.len());
    let mut ordered_selected = Vec::with_capacity(selected.len());
    for path in order {
        if let Some(index) = videos.iter().position(|candidate| candidate == path) {
            ordered.push(videos.remove(index));
            ordered_selected.push(selected.remove(index));
        }
    }
    ordered.append(videos);
    ordered_selected.append(selected);
    *videos = ordered;
    *selected = ordered_selected;
}
