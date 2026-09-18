use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};

use super::app::WakeVideoApp;
use super::video_row::video_row;

#[test]
fn test_wake_video_app_full_render() {
    for width in [1100.0, 950.0, 900.0] {
        let context = egui::Context::default();
        let mut app = WakeVideoApp::new_for_test(vec![
            "Destiny 2 2022.11.25 - 13.30.05.17.DVR.mp4".to_string(),
            "RB clone demo.mp4".to_string(),
        ]);

        let raw_input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.0))),
            ..Default::default()
        };

        let full_output = context.run(raw_input, |ctx| {
            app.render(ctx);
        });

        let mut max_x: f32 = 0.0;
        for shape in &full_output.shapes {
            let r = shape.shape.visual_bounding_rect();
            if r.max.x > max_x && r.max.x < 10000.0 {
                max_x = r.max.x;
            }
        }
        println!(
            "Width {}: max_x across all rendered shapes = {:.1}",
            width, max_x
        );
        assert!(
            max_x <= width + 1.0,
            "Shape rendered beyond window width {}: max_x={:.1}",
            width,
            max_x
        );
    }
}

#[test]
fn removing_video_preserves_unchecked_selection_state() {
    let mut app = WakeVideoApp::new_for_test(vec![
        "first.mp4".to_string(),
        "second.mp4".to_string(),
        "third.mp4".to_string(),
    ]);

    app.set_all_selected_for_test(false);
    app.remove_video_at(1);

    assert_eq!(app.selected_for_test(), &[false, false]);
}

#[test]
fn video_row_anchors_actions_to_right_edge_under_all_conditions() {
    let context = egui::Context::default();
    let test_cases = [
        // (window_width, title, has_thumb)
        (1100.0, "Destiny 2 2022.11.25 - 13.30.05.17.DVR", true),
        (1100.0, "Short", false),
        (
            1100.0,
            "Very Long Title That Must Never Push Buttons Off Screen Or Cause Overflow At All.mp4",
            true,
        ),
        (800.0, "Destiny 2 2022.11.25 - 13.30.05.17.DVR", true),
        (800.0, "Short", false),
        (650.0, "Destiny 2 2022.11.25 - 13.30.05.17.DVR", true),
    ];

    for (width, title, has_thumb) in test_cases {
        let mut action = None;
        context.begin_pass(RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 700.0))),
            ..Default::default()
        });
        egui::CentralPanel::default().show(&context, |ui| {
            egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        let thumb = if has_thumb {
                            Some(egui::TextureId::User(1))
                        } else {
                            None
                        };
                        action = Some(video_row(ui, title, Some(120), thumb, true, false, 100));
                    });
            });
        });
        let _ = context.end_pass();

        let action = action.expect("action should render");
        let row_rect = action.rect;
        let drag_rect = action.drag_rect;
        let remove_rect = action.remove_rect;

        println!("Width {width}, thumb {has_thumb}: row.right={:.1}, drag.right={:.1}, remove.right={:.1}",
            row_rect.right(), drag_rect.right(), remove_rect.right());

        assert!(
            row_rect.right() <= width,
            "Row escaped window: {:.1} > {:.1}",
            row_rect.right(),
            width
        );
        assert!(
            drag_rect.right() <= row_rect.right(),
            "Drag handle escaped row at width {width}: {:.1} > {:.1}",
            drag_rect.right(),
            row_rect.right()
        );
        assert!(
            remove_rect.right() <= drag_rect.left(),
            "Remove overlaps drag at width {width}: remove.right={:.1} > drag.left={:.1}",
            remove_rect.right(),
            drag_rect.left()
        );
        assert!(
            remove_rect.left() >= row_rect.left(),
            "Remove escaped left at width {width}"
        );
        assert!(
            action.preview_rect.right() <= remove_rect.left(),
            "Preview overlaps remove at width {width}: preview.right={:.1} > remove.left={:.1}",
            action.preview_rect.right(),
            remove_rect.left()
        );
        // Drag handle must stay anchored within 30px of the row's right edge
        assert!(
            row_rect.right() - drag_rect.right() <= 30.0,
            "Drag handle not anchored to right at width {width}: gap is {:.1}",
            row_rect.right() - drag_rect.right()
        );
    }
}
