//! A labelled angle control: a dial and a value field on one row, styled
//! like the header row of a [`crate::Slider`].
//!
//! Angles are in degrees with 0° pointing right and positive angles turning
//! clockwise on screen (image coordinates, y down). Dragging the dial points
//! it at the pointer; Shift snaps to [`SNAP_DEGREES`]. Arrow keys step the
//! value, wrapping around a full-circle range. Double-clicking the label,
//! dial or value field resets it.

use std::ops::RangeInclusive;

use egui::{self, Align, Layout, Sense, Stroke, Ui};

use crate::buttons::interaction_visuals_for_flags;
use crate::slider::{lock_slider_scroll, set_numeric, value_field, SliderMetrics};

/// Angle increment used while Shift is held during a drag.
pub const SNAP_DEGREES: f32 = 15.0;
/// Pointer positions this close to the dial centre have no usable direction.
const CENTER_DEAD_ZONE: f32 = 3.0;

/// A labelled angle dial. See the [module documentation](self).
#[must_use = "call `show` to render the angle dial"]
pub struct AngleDial<'a> {
    label: &'a str,
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    decimals: usize,
    step: f32,
    reset_value: f32,
    hover_text: Option<&'a str>,
}

impl<'a> AngleDial<'a> {
    /// A `-180..=180` degree dial. The label also identifies the widget, so it
    /// must be unique among its siblings.
    pub fn new(label: &'a str, value: &'a mut f32) -> Self {
        Self {
            label,
            value,
            range: -180.0..=180.0,
            decimals: 0,
            step: 1.0,
            reset_value: 0.0,
            hover_text: None,
        }
    }

    /// Allowed degrees. A range spanning 360° or more wraps.
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Decimal places shown and stored.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Degrees per arrow-key press.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn reset_to(mut self, reset_value: f32) -> Self {
        self.reset_value = reset_value;
        self
    }

    pub fn hover_text(mut self, hover_text: impl Into<Option<&'a str>>) -> Self {
        self.hover_text = hover_text.into();
        self
    }

    /// Shows the dial and returns whether the value changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let metrics = SliderMetrics::of(ui.ctx());
        let tooltip = match self.hover_text {
            Some(text) => format!("{text}\nDouble-click to reset."),
            None => "Double-click to reset.".to_owned(),
        };
        let (min, max) = ordered(&self.range);
        let reset_value = self.reset_value.clamp(min, max);
        let decimals = self.decimals;
        let value = self.value;
        let mut changed = false;
        ui.push_id(self.label, |ui| {
            let width = ui.available_width().max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, metrics.header_height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    let label = ui
                        .add(egui::Label::new(self.label).sense(Sense::click()))
                        .on_hover_text(&tooltip);
                    if label.double_clicked() {
                        changed |= set_numeric(value, f64::from(reset_value), decimals);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (field, field_changed) = value_field(
                            ui,
                            value,
                            min..=max,
                            decimals,
                            f64::from(self.step),
                            metrics,
                        );
                        changed |= field_changed;
                        if field.on_hover_text(&tooltip).double_clicked() {
                            changed |= set_numeric(value, f64::from(reset_value), decimals);
                        }
                        let options = DialOptions {
                            label: self.label,
                            min,
                            max,
                            step: self.step,
                            decimals,
                            reset_value,
                            edge: metrics.header_height,
                        };
                        changed |= dial(ui, value, &options).on_hover_text(&tooltip).changed();
                    });
                },
            );
            ui.add_space(metrics.row_bottom_space);
        });
        changed
    }
}

fn ordered(range: &RangeInclusive<f32>) -> (f32, f32) {
    let (start, end) = (*range.start(), *range.end());
    (start.min(end), start.max(end))
}

/// `degrees` mapped into `min..=max`: wrapped when the range covers a full
/// turn, clamped otherwise.
fn fit_angle(degrees: f32, min: f32, max: f32) -> f32 {
    if max - min >= 360.0 {
        let wrapped = min + (degrees - min).rem_euclid(360.0);
        // Keep the range's upper end (e.g. 180 rather than -180) reachable
        // from exactly that direction.
        if (wrapped - min).abs() < 1e-4 && (degrees - max).abs() < 1e-4 {
            max
        } else {
            wrapped
        }
    } else {
        degrees.clamp(min, max)
    }
}

struct DialOptions<'a> {
    label: &'a str,
    min: f32,
    max: f32,
    step: f32,
    decimals: usize,
    reset_value: f32,
    edge: f32,
}

fn dial(ui: &mut Ui, value: &mut f32, options: &DialOptions<'_>) -> egui::Response {
    let (rect, mut response) =
        ui.allocate_exact_size(egui::Vec2::splat(options.edge), Sense::click_and_drag());
    let enabled = ui.is_enabled();
    let center = rect.center();
    let mut changed = false;

    if enabled && response.clicked() {
        response.request_focus();
    }
    if enabled && response.double_clicked() {
        changed |= set_numeric(value, f64::from(options.reset_value), options.decimals);
    } else if enabled && (response.dragged() || response.drag_started()) {
        lock_slider_scroll(ui.ctx(), response.id);
        response.request_focus();
        if let Some(pointer) = response.interact_pointer_pos() {
            let delta = pointer - center;
            if delta.length() > CENTER_DEAD_ZONE {
                let mut degrees = delta.y.atan2(delta.x).to_degrees();
                if ui.input(|input| input.modifiers.shift) {
                    degrees = (degrees / SNAP_DEGREES).round() * SNAP_DEGREES;
                }
                let next = fit_angle(degrees, options.min, options.max);
                changed |= set_numeric(value, f64::from(next), options.decimals);
            }
        }
    }

    if enabled {
        let mut steps = 0_i32;
        if response.has_focus() {
            // Arrow keys adjust the angle instead of moving focus away.
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
            let decrease = ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            });
            let increase = ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
            });
            steps += i32::from(increase) - i32::from(decrease);
        }
        let set_value = ui.input(|input| {
            use egui::accesskit::{Action, ActionData};
            let id = response.id;
            steps += input.num_accesskit_action_requests(id, Action::Increment) as i32;
            steps -= input.num_accesskit_action_requests(id, Action::Decrement) as i32;
            input
                .accesskit_action_requests(id, Action::SetValue)
                .filter_map(|request| match request.data {
                    Some(ActionData::NumericValue(value)) => Some(value as f32),
                    _ => None,
                })
                .last()
        });
        if steps != 0 {
            let next = fit_angle(
                *value + steps as f32 * options.step,
                options.min,
                options.max,
            );
            changed |= set_numeric(value, f64::from(next), options.decimals);
        }
        if let Some(next) = set_value {
            let next = fit_angle(next, options.min, options.max);
            changed |= set_numeric(value, f64::from(next), options.decimals);
        }
    }
    if changed {
        response.mark_changed();
    }

    let current = f64::from(*value);
    let (min, max) = (f64::from(options.min), f64::from(options.max));
    response.widget_info(|| egui::WidgetInfo::slider(enabled, current, options.label));
    ui.ctx().accesskit_node_builder(response.id, |builder| {
        use egui::accesskit::Action;
        builder.set_min_numeric_value(min);
        builder.set_max_numeric_value(max);
        builder.set_numeric_value_step(f64::from(options.step));
        if enabled {
            builder.add_action(Action::SetValue);
            builder.add_action(Action::Increment);
            builder.add_action(Action::Decrement);
        }
    });

    let active = response.dragged() || response.is_pointer_button_down_on();
    let interaction = interaction_visuals_for_flags(
        ui,
        enabled,
        false,
        active,
        response.hovered(),
        response.has_focus(),
    );
    // Drawn like a slider: a track-weight ring and an accent handle on it,
    // joined to the centre by the needle.
    let metrics = SliderMetrics::of(ui.ctx());
    let painter = ui.painter();
    let knob_radius = (metrics.handle_radius - 2.0).max(3.0);
    let radius = options.edge * 0.5 - knob_radius;
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(
            metrics.track_height * 0.5,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ),
    );
    let knob = center + egui::Vec2::angled(value.to_radians()) * radius;
    let accent = if enabled {
        ui.visuals().selection.bg_fill
    } else {
        interaction.foreground
    };
    painter.line_segment([center, knob], Stroke::new(2.0, accent));
    painter.circle_filled(center, 2.0, accent);
    painter.circle_filled(knob, knob_radius, accent);
    painter.circle_stroke(knob, knob_radius, Stroke::new(1.0, interaction.foreground));

    response.on_hover_cursor(if active {
        egui::CursorIcon::Grabbing
    } else {
        egui::CursorIcon::Grab
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Event, Key, Modifiers, PointerButton, RawInput};

    struct Harness {
        ctx: egui::Context,
        time: f64,
        value: f32,
        enabled: bool,
    }

    impl Harness {
        fn new(value: f32) -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                time: 0.0,
                value,
                enabled: true,
            };
            harness.frame(Vec::new(), Modifiers::NONE);
            harness
        }

        fn frame(&mut self, events: Vec<Event>, modifiers: Modifiers) -> bool {
            self.time += 0.05;
            let (enabled, value) = (self.enabled, &mut self.value);
            let mut changed = false;
            let _ = self.ctx.run_ui(
                RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 120.0),
                    )),
                    time: Some(self.time),
                    events,
                    modifiers,
                    ..RawInput::default()
                },
                |ui| {
                    ui.set_width(400.0);
                    ui.add_enabled_ui(enabled, |ui| {
                        changed = AngleDial::new("Angle", value).reset_to(30.0).show(ui);
                    });
                },
            );
            changed
        }

        fn drag_to(&mut self, offset: egui::Vec2, modifiers: Modifiers) {
            let center = self.dial_center();
            let target = center + offset;
            self.frame(
                vec![Event::PointerMoved(center), press(center, true)],
                modifiers,
            );
            self.frame(vec![Event::PointerMoved(target)], modifiers);
            self.frame(vec![Event::PointerMoved(target)], modifiers);
            self.frame(vec![press(target, false)], modifiers);
        }

        fn dial_center(&self) -> egui::Pos2 {
            // The dial sits immediately left of the right-aligned value field.
            let metrics = SliderMetrics::CURRENT;
            let item_gap = egui::Style::default().spacing.item_spacing.x;
            pos2(
                400.0 - metrics.value_field_width - item_gap - metrics.header_height * 0.5,
                metrics.header_height * 0.5,
            )
        }
    }

    fn press(pos: egui::Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    fn key(key: Key, pressed: bool) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn dragging_points_the_dial_at_the_pointer_in_screen_degrees() {
        for (offset, expected) in [
            (egui::vec2(40.0, 0.0), 0.0),
            (egui::vec2(0.0, 40.0), 90.0),
            (egui::vec2(0.0, -40.0), -90.0),
            (egui::vec2(-40.0, 0.0), 180.0),
            (egui::vec2(40.0, 40.0), 45.0),
        ] {
            let mut harness = Harness::new(12.0);
            harness.drag_to(offset, Modifiers::NONE);
            assert!(
                (harness.value - expected).abs() < 1.0,
                "{offset:?}: {}",
                harness.value
            );
        }
    }

    #[test]
    fn shift_snaps_dragged_angles() {
        let mut harness = Harness::new(0.0);
        let radians = 37.0_f32.to_radians();
        harness.drag_to(
            egui::vec2(radians.cos(), radians.sin()) * 40.0,
            Modifiers::SHIFT,
        );
        assert_eq!(harness.value, 30.0);
    }

    #[test]
    fn arrow_keys_step_and_wrap_a_full_turn() {
        let mut harness = Harness::new(179.0);
        let center = harness.dial_center();
        for pressed in [true, false] {
            harness.frame(
                vec![Event::PointerMoved(center), press(center, pressed)],
                Modifiers::NONE,
            );
        }
        // egui applies the arrow-key focus lock from the frame after focus
        // is gained, as for any key pressed after clicking.
        harness.frame(Vec::new(), Modifiers::NONE);
        harness.frame(vec![key(Key::ArrowRight, true)], Modifiers::NONE);
        assert_eq!(harness.value, 180.0);
        harness.frame(
            vec![key(Key::ArrowRight, false), key(Key::ArrowRight, true)],
            Modifiers::NONE,
        );
        assert_eq!(harness.value, -179.0);
    }

    #[test]
    fn double_click_resets_and_disabled_dials_ignore_input() {
        let mut harness = Harness::new(-90.0);
        let center = harness.dial_center();
        for _ in 0..2 {
            for pressed in [true, false] {
                harness.frame(
                    vec![Event::PointerMoved(center), press(center, pressed)],
                    Modifiers::NONE,
                );
            }
        }
        assert_eq!(harness.value, 30.0);

        let mut disabled = Harness::new(-90.0);
        disabled.enabled = false;
        disabled.drag_to(egui::vec2(40.0, 0.0), Modifiers::NONE);
        assert_eq!(disabled.value, -90.0);
    }

    #[test]
    fn angles_wrap_on_full_turns_and_clamp_otherwise() {
        assert_eq!(fit_angle(190.0, -180.0, 180.0), -170.0);
        assert_eq!(fit_angle(180.0, -180.0, 180.0), 180.0);
        assert_eq!(fit_angle(-45.0, 0.0, 360.0), 315.0);
        assert_eq!(fit_angle(120.0, -90.0, 90.0), 90.0);
    }
}
