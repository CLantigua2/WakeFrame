use std::{collections::HashMap, path::PathBuf, process::Command};

use crate::library::{
    collect_metadata, discover_videos, format_duration, order_videos, VideoMetadata,
};
use crate::storage::{load_config, save_config};
use crate::ui_controls::{
    action_button, checkbox_with_label, help_button, pill_toggle, secondary_button, shuffle_icon,
    shuffle_toggle, trigger_row,
};
use crate::video_row;

use eframe::egui::{self, Color32, RichText, Rounding};
use wakeframe_common::AppConfig;

pub(crate) const BACKGROUND: Color32 = Color32::from_rgb(16, 26, 39);
pub(crate) const PANEL: Color32 = Color32::from_rgb(23, 34, 49);
pub(crate) const PANEL_RAISED: Color32 = Color32::from_rgb(28, 42, 60);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(33, 134, 253);
pub(crate) const TEXT: Color32 = Color32::from_rgb(235, 242, 250);
pub(crate) const MUTED: Color32 = Color32::from_rgb(151, 169, 190);

// Holds all editable settings, discovered videos, metadata, and transient UI state.
pub(crate) struct WakeVideoApp {
    config: AppConfig,
    videos: Vec<String>,
    selected: Vec<bool>,
    preview_index: Option<usize>,
    saved_message: Option<String>,
    dragging_index: Option<usize>,
    drag_target: Option<usize>,
    metadata: Vec<VideoMetadata>,
    thumbnails: HashMap<String, egui::TextureHandle>,
}

impl WakeVideoApp {
    // Initialize the settings UI from disk and the currently configured video folder.
    pub(crate) fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        let config = load_config();
        let videos = discover_videos(&config);
        let selected = videos
            .iter()
            .map(|path| {
                config.selected_video_ids.is_empty()
                    || config.selected_video_ids.iter().any(|id| id == path)
            })
            .collect();
        let metadata = collect_metadata(&videos);
        Self {
            config,
            videos,
            selected,
            preview_index: None,
            saved_message: None,
            dragging_index: None,
            drag_target: None,
            metadata,
            thumbnails: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(videos: Vec<String>) -> Self {
        let selected = vec![true; videos.len()];
        let metadata = vec![VideoMetadata::default(); videos.len()];
        Self {
            config: AppConfig::default(),
            videos,
            selected,
            preview_index: Some(0),
            saved_message: Some("Video folder saved".to_string()),
            dragging_index: None,
            drag_target: None,
            metadata,
            thumbnails: HashMap::new(),
        }
    }

    // Re-scan the configured folder after the user chooses a new library location.
    fn refresh_videos(&mut self) {
        self.videos = discover_videos(&self.config);
        self.selected = vec![true; self.videos.len()];
        order_videos(
            &mut self.videos,
            &mut self.selected,
            &self.config.video_order,
        );
        self.metadata = collect_metadata(&self.videos);
    }

    // Persist the current settings and rotation choices.
    fn save(&mut self) {
        self.sync_rotation_config();
        if save_config(&self.config).is_ok() {
            self.saved_message = Some("Settings saved".to_string());
        }
    }

    // Mirror the visible rotation list into the persisted config fields.
    fn sync_rotation_config(&mut self) {
        self.config.selected_video_ids = self
            .videos
            .iter()
            .zip(&self.selected)
            .filter_map(|(path, enabled)| enabled.then_some(path.clone()))
            .collect();
        self.config.video_order = self.videos.clone();
    }

    // Remove a video from the visible library without disturbing other checked states.
    pub(crate) fn remove_video_at(&mut self, index: usize) {
        if index >= self.videos.len() {
            return;
        }

        let path = self.videos.remove(index);
        self.selected.remove(index);
        if index < self.metadata.len() {
            self.metadata.remove(index);
        }
        self.config.excluded_video_ids.push(path);
        self.preview_index = match self.preview_index {
            Some(preview_index) if preview_index == index => None,
            Some(preview_index) if preview_index > index => Some(preview_index - 1),
            other => other,
        };
        self.sync_rotation_config();
    }

    #[cfg(test)]
    pub(crate) fn set_all_selected_for_test(&mut self, selected: bool) {
        self.selected.fill(selected);
    }

    #[cfg(test)]
    pub(crate) fn selected_for_test(&self) -> &[bool] {
        &self.selected
    }

    fn thumbnail_texture(
        &mut self,
        context: &egui::Context,
        index: usize,
    ) -> Option<egui::TextureId> {
        let path = self.metadata.get(index)?.thumbnail_path.as_ref()?;
        let key = path.to_string_lossy().to_string();
        if !self.thumbnails.contains_key(&key) {
            let image = image::open(path).ok()?.to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
            let texture =
                context.load_texture(key.clone(), color_image, egui::TextureOptions::LINEAR);
            self.thumbnails.insert(key.clone(), texture);
        }
        self.thumbnails.get(&key).map(|texture| texture.id())
    }
}

impl eframe::App for WakeVideoApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(context);
    }
}

impl WakeVideoApp {
    // Render the full settings surface: triggers, preview, library folder, and rotation list.
    pub(crate) fn render(&mut self, context: &egui::Context) {
        context.set_visuals(visuals());
        egui::TopBottomPanel::bottom("save_bar")
            .min_height(62.0)
            .frame(egui::Frame::none().fill(BACKGROUND).inner_margin(12.0))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    if let Some(message) = &self.saved_message {
                        ui.label(
                            RichText::new(message)
                                .size(12.0)
                                .color(Color32::from_rgb(120, 210, 160)),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if action_button(ui, "Save") {
                            self.save();
                        }
                    });
                });
            });
        egui::CentralPanel::default().show(context, |ui| {
            ui.add_space(18.0);

            egui::Frame::none()
                .fill(PANEL)
                .rounding(Rounding::same(14.0))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        pill_toggle(ui, &mut self.config.enabled, 1);
                        ui.label(
                            RichText::new("Enable WakeFrame")
                                .size(15.0)
                                .strong()
                                .color(TEXT),
                        );
                        help_button(
                            ui,
                            "Show or hide the WakeFrame intro when a configured trigger occurs.",
                        );
                    });
                    ui.label(
                        RichText::new("Play a welcome animation as soon as Windows resumes.")
                            .size(12.0)
                            .color(MUTED),
                    );
                });

            ui.add_space(12.0);
            egui::Frame::none()
                .fill(PANEL)
                .rounding(Rounding::same(14.0))
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Playback triggers")
                                .size(15.0)
                                .strong()
                                .color(TEXT),
                        );
                        help_button(
                            ui,
                            "Select which Windows events trigger WakeFrame playback:\n\n• Resume: Waking from sleep or hibernation\n• Unlock: Returning to desktop from lock screen\n• Sign in: Logging in after signing out\n• Startup: Launching WakeFrame when Windows boots",
                        );
                    });
                    ui.label(
                        RichText::new("Choose when WakeFrame should show the intro.")
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.add_space(8.0);
                    trigger_row(
                        ui,
                        &mut self.config.play_on_resume,
                        10,
                        "Resume from sleep or hibernation",
                    );
                    trigger_row(ui, &mut self.config.play_on_unlock, 11, "Unlock this PC");
                    trigger_row(
                        ui,
                        &mut self.config.play_on_logon,
                        12,
                        "Sign in after logout",
                    );
                    trigger_row(
                        ui,
                        &mut self.config.play_on_startup,
                        13,
                        "Start WakeFrame with Windows",
                    );
                });

            ui.add_space(14.0);
            ui.label(RichText::new("Preview").size(15.0).strong().color(TEXT));
            ui.add_space(6.0);
            egui::Frame::none()
                .fill(PANEL_RAISED)
                .rounding(Rounding::same(14.0))
                .show(ui, |ui| {
                    ui.set_min_height(if self.preview_index.is_some() {
                        250.0
                    } else {
                        96.0
                    });
                    ui.vertical_centered(|ui| {
                        ui.add_space(if self.preview_index.is_some() {
                            24.0
                        } else {
                            18.0
                        });
                        let preview_text = self
                            .preview_index
                            .and_then(|index| self.videos.get(index))
                            .and_then(|path| {
                                PathBuf::from(path)
                                    .file_stem()
                                    .and_then(|value| value.to_str())
                                    .map(str::to_owned)
                            })
                            .map(|name| format!("Previewing {}", name))
                            .unwrap_or_else(|| {
                                if self.videos.is_empty() {
                                    "No wake video selected".to_string()
                                } else {
                                    "Select a video below to preview".to_string()
                                }
                            });
                        if let Some(index) = self.preview_index {
                            if let Some(texture) = self.thumbnail_texture(context, index) {
                                ui.add(egui::Image::new((texture, egui::vec2(420.0, 190.0))));
                            }
                        } else {
                            ui.label(RichText::new(preview_text).size(15.0).color(MUTED));
                        }
                        if let Some(index) = self.preview_index {
                            if let Some(metadata) = self.metadata.get(index) {
                                if let Some(seconds) = metadata.duration_seconds {
                                    ui.label(
                                        RichText::new(format_duration(seconds))
                                            .size(12.0)
                                            .color(MUTED),
                                    );
                                }
                            }
                        }
                        if let Some(index) = self.preview_index {
                            if self.videos.get(index).is_some() {
                                ui.add_space(12.0);
                                if action_button(ui, "◉  Preview") {
                                    launch_preview(&self.videos[index]);
                                }
                            }
                        }
                    });
                });

            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Video library folder")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, "+  Add Folder") {
                        println!("WakeFrame UI: opening video folder picker");
                        match rfd::FileDialog::new()
                            .set_title("Choose your WakeFrame video folder")
                            .pick_folder()
                        {
                            Some(folder) => {
                                let folder_path = folder.display().to_string();
                                println!("WakeFrame UI: selected video folder: {}", folder_path);
                                self.config.video_directory = Some(folder_path);
                                self.refresh_videos();
                                match save_config(&self.config) {
                                    Ok(()) => {
                                        self.saved_message = Some("Video folder saved".to_string())
                                    }
                                    Err(error) => {
                                        self.saved_message =
                                            Some(format!("Could not save folder: {}", error))
                                    }
                                }
                            }
                            None => {
                                println!("WakeFrame UI: video folder picker cancelled");
                                self.saved_message = Some("Folder selection cancelled".to_string());
                            }
                        }
                    }
                });
            });
            ui.add_space(12.0);
            ui.label(
                RichText::new("Video Rotation")
                    .size(15.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(6.0);
            egui::Frame::none()
                .fill(PANEL)
                .rounding(Rounding::same(12.0))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let all_selected = !self.videos.is_empty()
                            && self.selected.iter().all(|selected| *selected);
                        if checkbox_with_label(ui, all_selected, "Select All") {
                            self.selected.fill(!all_selected);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            shuffle_toggle(ui, &mut self.config.play_random);
                            ui.add_space(8.0);
                            let text_response = ui.add(
                                egui::Label::new(RichText::new("Play Random").color(TEXT))
                                    .sense(egui::Sense::click()),
                            );
                            if text_response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if text_response.clicked() {
                                self.config.play_random = !self.config.play_random;
                            }
                            ui.add_space(8.0);
                            shuffle_icon(ui);
                        });
                    });
                });
            ui.add_space(8.0);

            egui::Frame::none()
                .fill(PANEL)
                .rounding(Rounding::same(12.0))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if self.videos.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Your video library is empty").color(MUTED));
                        ui.label(
                            RichText::new(
                                "Add a folder below to start building your wake rotation.",
                            )
                            .size(12.0)
                            .color(MUTED),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(430.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let mut remove_index = None;
                                let mut move_to = None;
                                let mut drag_released = false;
                                for index in 0..self.videos.len() {
                                    let path = self.videos[index].clone();
                                    let thumbnail = self.thumbnail_texture(context, index);
                                    let name = PathBuf::from(&path)
                                        .file_stem()
                                        .and_then(|value| value.to_str())
                                        .unwrap_or("Wake Video")
                                        .to_string();
                                    let duration = self
                                        .metadata
                                        .get(index)
                                        .and_then(|item| item.duration_seconds);
                                    let action = video_row::video_row(
                                        ui,
                                        &name,
                                        duration,
                                        thumbnail,
                                        self.selected[index],
                                        self.preview_index == Some(index),
                                        index + 1000,
                                    );
                                    self.selected[index] = action.checked;
                                    if action.preview_clicked {
                                        self.preview_index = Some(index);
                                    }
                                    if action.remove_clicked {
                                        remove_index = Some(index);
                                    }
                                    let row = action.rect;
                                    if action.drag_started {
                                        self.dragging_index = Some(index);
                                        self.drag_target = Some(index);
                                    }
                                    if action.drag_stopped && self.dragging_index == Some(index) {
                                        drag_released = true;
                                    }
                                    if action.dragged && self.dragging_index == Some(index) {
                                        self.drag_target = Some(index);
                                    }
                                    ui.add_space(8.0);
                                    if self.dragging_index == Some(index) {
                                        ui.painter().rect_filled(
                                            row,
                                            Rounding::same(10.0),
                                            Color32::from_rgba_unmultiplied(33, 134, 253, 48),
                                        );
                                        ui.painter().rect_stroke(
                                            row,
                                            Rounding::same(10.0),
                                            egui::Stroke::new(2.0, ACCENT),
                                        );
                                        ui.ctx().request_repaint();
                                    }
                                    if self.drag_target == Some(index) {
                                        let y = row.top();
                                        ui.painter().line_segment(
                                            [egui::pos2(row.left(), y), egui::pos2(row.right(), y)],
                                            egui::Stroke::new(3.0, ACCENT),
                                        );
                                    }
                                    if self.dragging_index.is_some() {
                                        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                                            if row.contains(pointer) {
                                                self.drag_target =
                                                    Some(if pointer.y < row.center().y {
                                                        index
                                                    } else {
                                                        index + 1
                                                    });
                                            }
                                        }
                                    }
                                    if drag_released {
                                        if let (Some(from), Some(target)) =
                                            (self.dragging_index, self.drag_target)
                                        {
                                            let destination = if target > from {
                                                target.saturating_sub(1)
                                            } else {
                                                target
                                            };
                                            if destination != from
                                                && destination < self.videos.len()
                                            {
                                                move_to = Some((from, destination));
                                            }
                                        }
                                        self.dragging_index = None;
                                        self.drag_target = None;
                                        drag_released = false;
                                    }
                                    if index + 1 < self.videos.len() {
                                        ui.separator();
                                    }
                                }
                                if let Some((from, to)) = move_to {
                                    let path = self.videos.remove(from);
                                    let selected = self.selected.remove(from);
                                    let metadata = self.metadata.remove(from);
                                    self.videos.insert(to, path);
                                    self.selected.insert(to, selected);
                                    self.metadata.insert(to, metadata);
                                    self.preview_index = match self.preview_index {
                                        Some(index) if index == from => Some(to),
                                        Some(index) if from < index && index <= to => {
                                            Some(index - 1)
                                        }
                                        Some(index) if to <= index && index < from => {
                                            Some(index + 1)
                                        }
                                        other => other,
                                    };
                                    self.config.video_order = self.videos.clone();
                                    let _ = save_config(&self.config);
                                }
                                if let Some(index) = remove_index {
                                    if index < self.videos.len() {
                                        self.remove_video_at(index);
                                        let _ = save_config(&self.config);
                                        self.saved_message =
                                            Some("Video removed from library".to_string());
                                    }
                                }
                            });
                    }
                });
        });
    }
}

// Preview currently reuses the player process, so it opens as the normal player window.
fn launch_preview(video: &str) {
    let player_name = if cfg!(windows) {
        "wakeframe-player.exe"
    } else {
        "wakeframe-player"
    };
    let player_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join(player_name)));

    if let Some(path) = player_path.filter(|path| path.exists() && !cfg!(debug_assertions)) {
        let _ = Command::new(path).arg(video).spawn();
        return;
    }

    let workspace_root = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent()?.parent().map(PathBuf::from));
    if let Some(root) = workspace_root {
        let _ = Command::new("cargo")
            .args(["run", "--bin", "wakeframe-player", "--quiet", "--", video])
            .current_dir(root)
            .spawn();
    }
}

// Centralize the dark visual theme used by the settings UI.
fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = PANEL;
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.hovered.bg_fill = ACCENT;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = ACCENT;
    visuals.window_rounding = Rounding::same(14.0);
    visuals
}
