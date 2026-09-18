use crate::app::{ACCENT, MUTED, PANEL_RAISED, TEXT};
use crate::library::format_duration;
use eframe::egui::{self, Color32, RichText, Rounding, TextureId};

// Describes every interaction a row can report back to the parent list.
pub struct VideoRowAction {
    pub checked: bool,
    pub preview_clicked: bool,
    pub remove_clicked: bool,
    pub drag_started: bool,
    pub drag_stopped: bool,
    pub dragged: bool,
    pub rect: egui::Rect,
    pub preview_rect: egui::Rect,
    pub remove_rect: egui::Rect,
    pub drag_rect: egui::Rect,
}

// Draw one video row with selection, thumbnail/title preview, remove, and drag controls.
pub fn video_row(
    ui: &mut egui::Ui,
    name: &str,
    duration: Option<u32>,
    thumbnail: Option<TextureId>,
    checked: bool,
    preview_active: bool,
    row_id: usize,
) -> VideoRowAction {
    let row_width = ui.available_width();
    let mut checked_state = checked;
    let mut action = VideoRowAction {
        checked,
        preview_clicked: false,
        remove_clicked: false,
        drag_started: false,
        drag_stopped: false,
        dragged: false,
        rect: egui::Rect::NOTHING,
        preview_rect: egui::Rect::NOTHING,
        remove_rect: egui::Rect::NOTHING,
        drag_rect: egui::Rect::NOTHING,
    };

    let inner_margin = egui::Margin::symmetric(24.0, 7.0);
    let response = egui::Frame::none()
        .fill(PANEL_RAISED)
        .rounding(Rounding::same(10.0))
        .inner_margin(inner_margin)
        .show(ui, |ui| {
            let total_inner_width = (row_width - inner_margin.left - inner_margin.right).max(100.0);
            ui.set_width(total_inner_width);

            let row_height = 90.0;
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(total_inner_width, row_height),
                egui::Sense::hover(),
            );

            let checkbox_size = egui::vec2(22.0, 22.0);
            let checkbox_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.center().y - checkbox_size.y * 0.5),
                checkbox_size,
            );
            let mut cb_ui = ui.new_child(egui::UiBuilder::new().max_rect(checkbox_rect));
            checkbox(&mut cb_ui, &mut checked_state, row_id);

            let mut current_left = checkbox_rect.right() + 20.0;
            if let Some(texture) = thumbnail {
                let thumb_size = egui::vec2(170.0, 90.0);
                let thumb_rect = egui::Rect::from_min_size(
                    egui::pos2(current_left, rect.center().y - thumb_size.y * 0.5),
                    thumb_size,
                );
                let mut thumb_ui = ui.new_child(egui::UiBuilder::new().max_rect(thumb_rect));
                thumb_ui.add(egui::Image::new((texture, thumb_size)));
                current_left = thumb_rect.right() + 20.0;
            }

            let drag_size = egui::vec2(36.0, 34.0);
            let drag_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.right() - drag_size.x,
                    rect.center().y - drag_size.y * 0.5,
                ),
                drag_size,
            );
            let mut drag_ui = ui.new_child(egui::UiBuilder::new().max_rect(drag_rect));
            let drag = drag_handle(&mut drag_ui, row_id);
            if drag.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            if drag.is_pointer_button_down_on() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            action.drag_rect = drag.rect;
            action.drag_started = drag.drag_started();
            action.drag_stopped = drag.drag_stopped();
            action.dragged = drag.dragged();

            let remove_size = egui::vec2(78.0, 30.0);
            let remove_rect = egui::Rect::from_min_size(
                egui::pos2(
                    drag_rect.left() - 12.0 - remove_size.x,
                    rect.center().y - remove_size.y * 0.5,
                ),
                remove_size,
            );
            let mut remove_ui = ui.new_child(egui::UiBuilder::new().max_rect(remove_rect));
            let remove = remove_ui
                .add_sized(
                    remove_size,
                    egui::Button::new(RichText::new("- Remove").size(11.0).color(MUTED))
                        .fill(PANEL_RAISED)
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(65, 84, 106)))
                        .rounding(Rounding::same(7.0)),
                )
                .on_hover_text("Remove from library");
            if remove.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            action.remove_rect = remove.rect;
            if remove.clicked() {
                action.remove_clicked = true;
            }

            let title_right = (remove_rect.left() - 16.0).max(current_left + 40.0);
            let title_rect = egui::Rect::from_min_max(
                egui::pos2(current_left, rect.top() + 10.0),
                egui::pos2(title_right, rect.bottom() - 10.0),
            );
            action.preview_rect = title_rect;
            let mut preview_ui = ui.new_child(egui::UiBuilder::new().max_rect(title_rect));
            let response = preview_ui
                .allocate_ui_with_layout(
                    title_rect.size(),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let label = ui.add(
                            egui::Label::new(RichText::new(name).color(TEXT))
                                .truncate()
                                .sense(egui::Sense::click()),
                        );
                        if label.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if let Some(seconds) = duration {
                            ui.label(
                                RichText::new(format_duration(seconds))
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        }
                        label
                    },
                )
                .inner;
            action.preview_clicked = response.clicked();
        });
    action.checked = checked_state;
    action.rect = response.response.rect;
    if preview_active {
        ui.painter().rect_stroke(
            action.rect,
            Rounding::same(10.0),
            egui::Stroke::new(1.0, ACCENT),
        );
    }
    action
}

// Custom checkbox drawing keeps row selection visually consistent with the app theme.
fn checkbox(ui: &mut egui::Ui, checked: &mut bool, row_id: usize) {
    ui.push_id(("video-checkbox", row_id), |ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            *checked = !*checked;
        }
        let animation = ui
            .ctx()
            .animate_bool_with_time(ui.id().with("checked"), *checked, 0.16);
        let fill = Color32::from_rgba_unmultiplied(33, 134, 253, (animation * 255.0) as u8);
        ui.painter().rect_filled(rect, Rounding::same(5.0), fill);
        ui.painter().rect_stroke(
            rect,
            Rounding::same(5.0),
            egui::Stroke::new(
                1.5,
                if animation > 0.5 {
                    ACCENT
                } else {
                    Color32::from_rgb(104, 130, 157)
                },
            ),
        );
        if animation > 0.01 {
            let stroke = egui::Stroke::new(2.0, Color32::WHITE);
            let left = rect.left() + 5.0;
            let mid = rect.center().y + 1.0;
            ui.painter().line_segment(
                [
                    egui::pos2(left, mid),
                    egui::pos2(left + 4.0 * animation, mid + 4.0 * animation),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(left + 4.0 * animation, mid + 4.0 * animation),
                    egui::pos2(
                        left + 4.0 + (rect.right() - left - 8.0) * animation,
                        rect.top() + 5.0,
                    ),
                ],
                stroke,
            );
            ui.ctx().request_repaint();
        }
    });
}

// Drag handle reports movement to the parent list, which owns the actual reordering.
fn drag_handle(ui: &mut egui::Ui, row_id: usize) -> egui::Response {
    ui.push_id(("video-drag", row_id), |ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(36.0, 34.0), egui::Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        let stroke = egui::Stroke::new(2.0, MUTED);
        for offset in [-6.0, 0.0, 6.0] {
            ui.painter().line_segment(
                [
                    egui::pos2(rect.center().x - 10.0, rect.center().y + offset),
                    egui::pos2(rect.center().x + 10.0, rect.center().y + offset),
                ],
                stroke,
            );
        }
        response
    })
    .inner
}
