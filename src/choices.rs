//! Controls for picking among a few options: segmented controls and chips.

use egui::{self, Color32, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui};

use crate::metrics::{control_height, CARD_RADIUS};

/// Gap between the track and the highlight of the selected segment.
const SEGMENT_INSET: f32 = 2.0;

/// One choice among a few options, drawn as a single rounded track of equal
/// segments with a highlight that slides to the selected one.
///
/// Clicking a segment, or Space and Enter while it has focus, selects its
/// option; the arrow keys move the selection and focus to the neighbouring
/// segment. The response is marked changed when `value` changes. Each segment
/// is a radio button for assistive technology. When `value` matches no option,
/// no segment is highlighted.
pub fn segmented_control<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    width: f32,
) -> Response {
    let height = control_height(ui);
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    if options.is_empty() {
        return response;
    }
    let enabled = ui.is_enabled();
    let segment_width = width / options.len() as f32;
    let segment_rect = |index: usize| {
        egui::Rect::from_min_size(
            rect.min + egui::vec2(segment_width * index as f32, 0.0),
            egui::vec2(segment_width, height),
        )
    };
    let segment_id = |index: usize| response.id.with(("segment", index));
    let selected = options.iter().position(|(option, _)| option == value);

    let mut segments = Vec::with_capacity(options.len());
    let mut picked = None;
    for (index, (_, label)) in options.iter().enumerate() {
        let segment = ui.interact(segment_rect(index), segment_id(index), Sense::click());
        let is_selected = selected == Some(index);
        segment.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::RadioButton, enabled, is_selected, *label)
        });
        if segment.clicked() {
            picked = Some(index);
        }
        if segment.has_focus() {
            let step = ui.input_mut(|input| {
                let right = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight);
                let left = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft);
                i32::from(right) - i32::from(left)
            });
            let target = index
                .saturating_add_signed(step as isize)
                .min(options.len() - 1);
            if target != index {
                picked = Some(target);
                ui.memory_mut(|memory| memory.request_focus(segment_id(target)));
            }
        }
        segments.push(segment);
    }
    if let Some(index) = picked {
        if selected != Some(index) {
            *value = options[index].0;
            response.mark_changed();
        }
    }
    let selected = picked.or(selected);
    if segments.iter().any(Response::hovered) {
        if let Some(cursor) = ui.visuals().interact_cursor {
            ui.ctx().set_cursor_icon(cursor);
        }
    }
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let visuals = ui.visuals();
    let painter = ui.painter();
    let inner_radius = (CARD_RADIUS - SEGMENT_INSET).max(0.0);
    painter.rect(
        rect,
        CARD_RADIUS,
        visuals.widgets.inactive.bg_fill,
        visuals.widgets.inactive.bg_stroke,
        StrokeKind::Inside,
    );
    for (index, segment) in segments.iter().enumerate() {
        if segment.hovered() && selected != Some(index) {
            painter.rect_filled(
                segment_rect(index).shrink(SEGMENT_INSET),
                inner_radius,
                visuals.widgets.hovered.bg_fill,
            );
        }
    }
    if let Some(selected) = selected {
        let position = ui.ctx().animate_value_with_time(
            response.id.with("highlight"),
            selected as f32,
            ui.style().animation_time,
        );
        let highlight = egui::Rect::from_min_size(
            rect.min + egui::vec2(segment_width * position, 0.0),
            egui::vec2(segment_width, height),
        )
        .shrink(SEGMENT_INSET);
        painter.rect_filled(highlight, inner_radius, visuals.selection.bg_fill);
    }
    let padding = ui.spacing().button_padding.x;
    for (index, (_, label)) in options.iter().enumerate() {
        let segment = segment_rect(index);
        let color = if selected == Some(index) {
            visuals.selection.stroke.color
        } else {
            visuals.widgets.inactive.text_color()
        };
        let galley = egui::WidgetText::from(*label).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (segment.width() - padding * 2.0).max(1.0),
            egui::TextStyle::Button,
        );
        painter.galley(segment.center() - galley.size() * 0.5, galley, color);
    }
    for (index, segment) in segments.iter().enumerate() {
        if segment.has_focus() {
            painter.rect_stroke(
                segment_rect(index).shrink(SEGMENT_INSET),
                inner_radius,
                Stroke::new(1.5, visuals.widgets.active.bg_stroke.color),
                StrokeKind::Inside,
            );
        }
    }
    response
}

/// A pill-shaped selectable option for filters and quick choices, grouped in
/// an [`crate::action_row`]. Unselected chips are outlined and selected chips
/// filled with the accent. The response reports the selected state; the
/// caller decides what a click selects, so a row can allow one choice or
/// several.
pub fn chip(ui: &mut Ui, label: impl Into<egui::WidgetText>, selected: bool) -> Response {
    let height = control_height(ui);
    let mut button = egui::Button::new(label.into())
        .selected(selected)
        .corner_radius(height * 0.5)
        .min_size(egui::vec2(height, height));
    if !selected {
        button = button.fill(Color32::TRANSPARENT);
    }
    ui.add(button)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::CONTROL_HEIGHT;
    use crate::test_support::{click, key_press, last_widget_info, run_frame};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Mode {
        Library,
        Develop,
        Settings,
    }

    const MODES: [(Mode, &str); 3] = [
        (Mode::Library, "Library"),
        (Mode::Develop, "Develop"),
        (Mode::Settings, "Settings"),
    ];

    #[test]
    fn segmented_control_fills_its_width_at_control_height() {
        egui::__run_test_ui(|ui| {
            let mut mode = Mode::Library;
            let response = segmented_control(ui, &mut mode, &MODES, 240.0);
            assert_eq!(response.rect.width(), 240.0);
            assert_eq!(response.rect.height(), CONTROL_HEIGHT);
        });
    }

    #[test]
    fn clicking_a_segment_selects_it_and_reports_a_radio_button() {
        let ctx = egui::Context::default();
        let mut mode = Mode::Library;
        let mut changed = false;
        let events = click(
            &ctx,
            |ui| {
                let response = segmented_control(ui, &mut mode, &MODES, 240.0);
                changed |= response.changed();
                response.rect
            },
            // The middle of the third segment.
            |rect| egui::pos2(rect.left() + rect.width() * 5.0 / 6.0, rect.center().y),
        );
        assert_eq!(mode, Mode::Settings);
        assert!(changed);
        let info = last_widget_info(&events).expect("segment reports its click");
        assert_eq!(info.typ, egui::WidgetType::RadioButton);
        assert_eq!(info.label.as_deref(), Some("Settings"));
    }

    #[test]
    fn clicking_the_selected_segment_changes_nothing() {
        let ctx = egui::Context::default();
        let mut mode = Mode::Library;
        let mut changed = false;
        click(
            &ctx,
            |ui| {
                let response = segmented_control(ui, &mut mode, &MODES, 240.0);
                changed |= response.changed();
                response.rect
            },
            |rect| egui::pos2(rect.left() + 10.0, rect.center().y),
        );
        assert_eq!(mode, Mode::Library);
        assert!(!changed);
    }

    #[test]
    fn arrow_keys_move_the_selection_within_the_options() {
        let ctx = egui::Context::default();
        let mut mode = Mode::Library;
        let mut show = |ui: &mut Ui| segmented_control(ui, &mut mode, &MODES, 240.0).rect;
        run_frame(&ctx, Vec::new(), &mut show);
        // Focus the first segment, then step right twice and once past the end.
        run_frame(&ctx, vec![key_press(egui::Key::Tab)], &mut show);
        for _ in 0..3 {
            run_frame(&ctx, vec![key_press(egui::Key::ArrowRight)], &mut show);
        }
        run_frame(&ctx, vec![key_press(egui::Key::ArrowLeft)], &mut show);
        assert_eq!(mode, Mode::Develop);
    }

    #[test]
    fn disabled_segments_ignore_clicks() {
        let ctx = egui::Context::default();
        let mut mode = Mode::Library;
        click(
            &ctx,
            |ui| {
                ui.add_enabled_ui(false, |ui| {
                    segmented_control(ui, &mut mode, &MODES, 240.0).rect
                })
                .inner
            },
            |rect| egui::pos2(rect.right() - 10.0, rect.center().y),
        );
        assert_eq!(mode, Mode::Library);
    }

    #[test]
    fn chips_are_control_height_pills_that_report_selection() {
        let ctx = egui::Context::default();
        let mut height = 0.0;
        let events = click(
            &ctx,
            |ui| {
                let response = chip(ui, "4:3", true);
                height = response.rect.height();
                response.rect
            },
            |rect| rect.center(),
        );
        assert_eq!(height, CONTROL_HEIGHT);
        let info = last_widget_info(&events).expect("chip reports its click");
        assert_eq!(info.selected, Some(true));
        assert_eq!(info.label.as_deref(), Some("4:3"));
    }
}
