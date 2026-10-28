//! On/off options drawn as a label followed by a sliding switch.

use egui::{self, Color32, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Vec2};

use crate::metrics::{control_height, SPACE_SM};

/// The switch track for a row of `row_height`: about half the row high and
/// 1.8 times as wide, so 18 × 32 px on desktop and 22 × 40 px on Android.
pub(crate) fn track_size(row_height: f32) -> Vec2 {
    let height = (row_height * 0.56).round();
    egui::vec2((height * 1.8).round(), height)
}

/// Gap between the knob and the track edge.
const KNOB_INSET: f32 = 3.0;

/// An on/off option: its label, then a switch. Clicking anywhere on the row,
/// or Space and Enter while it has focus, flips `value` and marks the response
/// changed, as `ui.checkbox` does; assistive technology sees a checkbox with
/// its checked state.
///
/// The row is as wide as its label and switch, for rows of several options.
/// [`toggle_row`] fills the width instead.
pub fn toggle(ui: &mut Ui, value: &mut bool, label: impl Into<egui::WidgetText>) -> Response {
    switch_row(ui, value, label.into(), false)
}

/// A [`toggle`] spanning the available width, with the switch at the end of
/// the row: the layout for an option on its own line in a card or form.
/// [`crate::toggle_with_help`] adds a tooltip.
pub fn toggle_row(ui: &mut Ui, value: &mut bool, label: impl Into<egui::WidgetText>) -> Response {
    switch_row(ui, value, label.into(), true)
}

fn switch_row(ui: &mut Ui, value: &mut bool, label: egui::WidgetText, fill: bool) -> Response {
    let text = label.text().to_owned();
    let height = control_height(ui);
    let track = track_size(height);
    let available = ui.available_width().max(1.0);
    let max_text_width = if fill {
        (available - track.x - SPACE_SM).max(1.0)
    } else {
        f32::INFINITY
    };
    let galley = label.into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        max_text_width,
        egui::TextStyle::Button,
    );
    let width = if fill {
        available
    } else {
        galley.size().x + SPACE_SM + track.x
    };
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let (enabled, checked) = (ui.is_enabled(), *value);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, checked, &text)
    });
    if response.hovered() {
        if let Some(cursor) = ui.visuals().interact_cursor {
            ui.ctx().set_cursor_icon(cursor);
        }
    }
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let text_pos = egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5);
    let text_color = ui.visuals().widgets.inactive.text_color();
    ui.painter().galley(text_pos, galley, text_color);
    let track_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - track.x, rect.center().y - track.y * 0.5),
        track,
    );
    paint_switch(ui, &response, track_rect, checked);
    response
}

/// Paints a switch in `track_rect` for the row `response` belongs to: its
/// track fills and its knob slides to the end while `checked`, and it shows
/// the row's hover and focus.
pub(crate) fn paint_switch(ui: &Ui, response: &Response, track_rect: egui::Rect, checked: bool) {
    let visuals = ui.visuals();
    let track = track_rect.size();
    let on = ui.ctx().animate_bool_responsive(response.id, checked);
    let idle = if response.hovered() || response.has_focus() {
        &visuals.widgets.hovered
    } else {
        &visuals.widgets.inactive
    };
    let fill = lerp_color(idle.bg_fill, visuals.selection.bg_fill, on);
    let stroke = Stroke::new(
        1.0,
        lerp_color(
            idle.bg_stroke.color,
            visuals.widgets.active.bg_stroke.color,
            on,
        ),
    );
    let knob_radius = track.y * 0.5 - KNOB_INSET;
    let knob_x = egui::lerp(
        (track_rect.left() + track.y * 0.5)..=(track_rect.right() - track.y * 0.5),
        on,
    );
    let knob = lerp_color(idle.fg_stroke.color, Color32::WHITE, on);
    // The same focus ring as a segmented control's segments.
    let focus = response
        .has_focus()
        .then(|| Stroke::new(1.5, visuals.widgets.active.bg_stroke.color));

    let painter = ui.painter();
    let radius = track.y * 0.5;
    painter.rect(track_rect, radius, fill, stroke, StrokeKind::Inside);
    if let Some(focus) = focus {
        painter.rect_stroke(
            track_rect.expand(2.0),
            radius + 2.0,
            focus,
            StrokeKind::Outside,
        );
    }
    painter.circle_filled(egui::pos2(knob_x, track_rect.center().y), knob_radius, knob);
}

fn lerp_color(from: Color32, to: Color32, t: f32) -> Color32 {
    from.lerp_to_gamma(to, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{Metrics, CONTROL_HEIGHT};
    use crate::test_support::{click, key_press, last_widget_info, run_frame};

    fn click_toggle(value: &mut bool, enabled: bool) -> (bool, Option<egui::WidgetInfo>) {
        let ctx = egui::Context::default();
        let mut changed = false;
        let events = click(
            &ctx,
            |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let response = toggle(ui, value, "Option");
                    changed |= response.changed();
                    response.rect
                })
                .inner
            },
            |rect| rect.center(),
        );
        (changed, last_widget_info(&events))
    }

    #[test]
    fn toggles_flip_on_click_and_report_a_checkbox_state() {
        let mut value = false;
        let (changed, info) = click_toggle(&mut value, true);
        assert!(value);
        assert!(changed);
        let info = info.expect("toggle reports its click");
        assert_eq!(info.typ, egui::WidgetType::Checkbox);
        assert_eq!(info.selected, Some(true));
        assert_eq!(info.label.as_deref(), Some("Option"));

        let (changed, _) = click_toggle(&mut value, true);
        assert!(!value);
        assert!(changed);
    }

    #[test]
    fn disabled_toggles_ignore_clicks() {
        let mut value = false;
        let (changed, _) = click_toggle(&mut value, false);
        assert!(!value);
        assert!(!changed);
    }

    #[test]
    fn focused_toggles_flip_on_space_and_enter() {
        let ctx = egui::Context::default();
        let mut value = false;
        let mut press = |key: Option<egui::Key>| {
            let mut show = |ui: &mut Ui| {
                let response = toggle(ui, &mut value, "Option");
                if !response.has_focus() {
                    response.request_focus();
                }
                response.rect
            };
            run_frame(&ctx, key.into_iter().map(key_press).collect(), &mut show);
            value
        };
        assert!(!press(None));
        assert!(press(Some(egui::Key::Space)));
        assert!(!press(Some(egui::Key::Enter)));
    }

    #[test]
    fn toggle_hugs_its_label_and_toggle_row_fills_the_row() {
        egui::__run_test_ui(|ui| {
            ui.set_width(300.0);
            let mut value = true;
            let compact = toggle(ui, &mut value, "Option").rect;
            let full = toggle_row(ui, &mut value, "Option").rect;
            assert_eq!(compact.height(), CONTROL_HEIGHT);
            assert!(compact.width() < 150.0, "compact {compact:?}");
            assert_eq!(full.width(), 300.0);
            assert_eq!(full.height(), CONTROL_HEIGHT);
        });
    }

    #[test]
    fn long_labels_truncate_inside_a_full_width_row() {
        egui::__run_test_ui(|ui| {
            ui.set_width(120.0);
            let mut value = false;
            let label = "A very long option label that cannot fit beside the switch";
            let response = toggle_row(ui, &mut value, label);
            assert_eq!(response.rect.width(), 120.0);
        });
    }

    #[test]
    fn track_keeps_the_switch_inside_platform_rows() {
        for metrics in [Metrics::DESKTOP, Metrics::ANDROID] {
            let track = track_size(metrics.control_height);
            assert!(track.y < metrics.control_height);
            assert!(track.x > track.y);
            assert!(track.y * 0.5 - KNOB_INSET > 4.0);
        }
    }
}
