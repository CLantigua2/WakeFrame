use eframe::egui::{self, Color32, RichText, Rounding};

use crate::app::{ACCENT, MUTED, PANEL_RAISED, TEXT};

// Primary command button used for committing settings actions.
pub(crate) fn action_button(ui: &mut egui::Ui, label: &str) -> bool {
    let response = ui.add_sized(
        [112.0, 34.0],
        egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
            .fill(ACCENT)
            .rounding(Rounding::same(10.0)),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

// Trigger rows pair a pill switch with clickable descriptive text.
pub(crate) fn trigger_row(ui: &mut egui::Ui, enabled: &mut bool, id: usize, label: &str) {
    ui.horizontal(|ui| {
        pill_toggle(ui, enabled, id);
        let text_response =
            ui.add(egui::Label::new(RichText::new(label).color(TEXT)).sense(egui::Sense::click()));
        if text_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if text_response.clicked() {
            *enabled = !*enabled;
        }
    });
}

// Small inline help affordance for explaining settings without adding persistent text.
pub(crate) fn help_button(ui: &mut egui::Ui, description: &str) {
    let response = ui
        .add(egui::Button::new(RichText::new("?").color(MUTED)).frame(false))
        .on_hover_text(description);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

// Custom checkbox plus label used by bulk-selection controls.
pub(crate) fn checkbox_with_label(ui: &mut egui::Ui, checked: bool, label: &str) -> bool {
    let size = egui::vec2(20.0, 20.0);
    let (box_rect, box_response) = ui.allocate_exact_size(size, egui::Sense::click());
    let text_response = ui.add(
        egui::Label::new(RichText::new(label).size(13.0).color(TEXT)).sense(egui::Sense::click()),
    );
    let hovered = box_response.hovered() || text_response.hovered();
    let clicked = box_response.clicked() || text_response.clicked();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let animation = ui
        .ctx()
        .animate_bool_with_time(ui.id().with("select_all_cb"), checked, 0.16);
    let fill = Color32::from_rgba_unmultiplied(33, 134, 253, (animation * 255.0) as u8);
    ui.painter()
        .rect_filled(box_rect, Rounding::same(5.0), fill);
    ui.painter().rect_stroke(
        box_rect,
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
        let left = box_rect.left() + 4.5;
        let mid = box_rect.center().y + 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(left, mid),
                egui::pos2(left + 3.5 * animation, mid + 3.5 * animation),
            ],
            stroke,
        );
        ui.painter().line_segment(
            [
                egui::pos2(left + 3.5 * animation, mid + 3.5 * animation),
                egui::pos2(
                    left + 3.5 + (box_rect.right() - left - 7.5) * animation,
                    box_rect.top() + 4.5,
                ),
            ],
            stroke,
        );
        ui.ctx().request_repaint();
    }
    clicked
}

// Compact animated on/off switch used throughout the settings surface.
pub(crate) fn pill_toggle(ui: &mut egui::Ui, enabled: &mut bool, control_id: usize) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(48.0, 26.0), egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        *enabled = !*enabled;
    }
    let animation =
        ui.ctx()
            .animate_bool_with_time(ui.id().with(("pill", control_id)), *enabled, 0.16);
    let track = if animation > 0.01 {
        ACCENT
    } else {
        PANEL_RAISED
    };
    ui.painter().rect_filled(rect, Rounding::same(13.0), track);
    let knob_x = egui::lerp((rect.left() + 13.0)..=(rect.right() - 13.0), animation);
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), 9.0, Color32::WHITE);
    if animation > 0.01 && animation < 0.99 {
        ui.ctx().request_repaint();
    }
    response.clicked()
}

// Wider animated switch used for the Play Random control.
pub(crate) fn shuffle_toggle(ui: &mut egui::Ui, enabled: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(68.0, 28.0), egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        *enabled = !*enabled;
    }
    let animation = ui
        .ctx()
        .animate_bool_with_time(ui.id().with("toggle"), *enabled, 0.18);
    let track = if animation > 0.01 {
        ACCENT
    } else {
        PANEL_RAISED
    };
    ui.painter().rect_filled(rect, Rounding::same(14.0), track);
    let knob_x = egui::lerp((rect.left() + 14.0)..=(rect.right() - 14.0), animation);
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), 9.0, Color32::WHITE);
    if animation > 0.01 && animation < 0.99 {
        ui.ctx().request_repaint();
    }
    response.clicked()
}

// Draw the shuffle glyph next to its switch without depending on an icon font.
pub(crate) fn shuffle_icon(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 28.0), egui::Sense::hover());
    let stroke = egui::Stroke::new(2.4, MUTED);
    let left = rect.left() + 3.0;
    let right = rect.right() - 3.0;
    let top = rect.center().y - 7.0;
    let bottom = rect.center().y + 7.0;
    ui.painter()
        .line_segment([egui::pos2(left, top), egui::pos2(left + 5.0, top)], stroke);
    ui.painter().line_segment(
        [egui::pos2(left + 5.0, top), egui::pos2(right - 7.0, bottom)],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(right - 7.0, bottom), egui::pos2(right, bottom)],
        stroke,
    );
    ui.painter().line_segment(
        [
            egui::pos2(right - 5.0, bottom - 5.0),
            egui::pos2(right, bottom),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(left, bottom), egui::pos2(left + 5.0, bottom)],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(left + 5.0, bottom), egui::pos2(right - 7.0, top)],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(right - 7.0, top), egui::pos2(right, top)],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(right - 5.0, top + 5.0), egui::pos2(right, top)],
        stroke,
    );
}

// Secondary command button for lower-emphasis actions such as Add Folder.
pub(crate) fn secondary_button(ui: &mut egui::Ui, label: &str) -> bool {
    let response = ui.add_sized(
        [112.0, 34.0],
        egui::Button::new(RichText::new(label).color(TEXT))
            .fill(PANEL_RAISED)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(48, 67, 88)))
            .rounding(Rounding::same(10.0)),
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}
