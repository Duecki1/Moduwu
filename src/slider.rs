//! A labelled numeric slider with an editable value field.
//!
//! The track ignores mostly vertical drags so that sliders inside scroll areas
//! do not steal scrolling, and claims the pointer once a drag is clearly
//! horizontal. Scroll areas ask [`slider_scroll_locked`] whether a slider owns
//! the current drag. With a mouse the handle follows the pointer. On touch
//! screens a drag moves the value by the finger's travel from wherever the
//! drag started, so grabbing the slider never moves the value. Moving the
//! finger away from the track while dragging slows the value for fine
//! adjustment, and the small shift of a lifting finger is discarded.
//! Double-clicking the label, track or value resets to
//! [`Slider::reset_to`], zero by default, stored at full precision rather than
//! rounded to the displayed decimals. Sliders show no tooltip: a hint
//! must not cover a control while it is being adjusted. A [`SliderScale`]
//! sets where values sit along the track; the value field, keyboard steps and
//! accessibility values stay in the value's own units.

use std::collections::VecDeque;
use std::ops::RangeInclusive;

use egui::{self, emath::Numeric, Align, Align2, Color32, Layout, RichText, Sense, Stroke, Ui};

use crate::buttons::interaction_visuals_for_flags;
use crate::metrics::Metrics;
use crate::number_field::NumberField;
use crate::responsive::is_compact_portrait;

const TRACK_DRAG_THRESHOLD: f32 = 8.0;
const HANDLE_DRAG_THRESHOLD: f32 = 2.0;
/// Horizontal movement must exceed vertical movement by this factor before a
/// drag that began on the track moves the value.
const HORIZONTAL_INTENT_RATIO: f32 = 1.15;
/// Points beyond the track row at which a touch drag moves the value at half
/// speed; at three times this distance it moves at a quarter.
const FINE_TUNE_DISTANCE: f32 = 60.0;
/// Seconds before a touch is released during which a small movement counts as
/// the finger rolling off the screen rather than an adjustment.
const LIFT_OFF_WINDOW: f64 = 0.08;
/// Largest horizontal movement in points within [`LIFT_OFF_WINDOW`] that is
/// discarded on release.
const LIFT_OFF_SLOP: f32 = 4.0;
const COMPACT_ROW_GAP: f32 = 4.0;
const COMPACT_ROW_BOTTOM_SPACE: f32 = 1.0;
const COMPACT_LABEL_MIN_WIDTH: f32 = 76.0;
const COMPACT_LABEL_MAX_WIDTH: f32 = 104.0;
const COMPACT_TRACK_MIN_WIDTH: f32 = 64.0;
const GRADIENT_SEGMENTS: usize = 64;
const SWATCH_RADIUS: f32 = 4.5;

/// Sizes of a [`Slider`]. By default they follow the [`Metrics`] in effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderMetrics {
    /// Height of the label and value row; also the track row in compact layouts.
    pub header_height: f32,
    pub value_field_width: f32,
    /// Height of the track row in stacked layouts.
    pub track_row_height: f32,
    pub track_height: f32,
    pub handle_radius: f32,
    /// Radius of the area that grabs the handle rather than the track.
    pub handle_touch_radius: f32,
    /// Vertical gap between the label row and the track.
    pub control_gap: f32,
    pub row_bottom_space: f32,
}

impl SliderMetrics {
    pub const DESKTOP: Self = Self::for_platform(false);
    pub const ANDROID: Self = Self::for_platform(true);
    pub const CURRENT: Self = Self::for_platform(cfg!(target_os = "android"));

    /// Slider sizes matching the [`Metrics`] in effect for `ctx`.
    pub fn of(ctx: &egui::Context) -> Self {
        Self::for_platform(Metrics::of(ctx).touch_layout)
    }

    pub const fn for_platform(android: bool) -> Self {
        Self {
            header_height: Metrics::for_platform(android).control_height,
            value_field_width: if android { 60.0 } else { 72.0 },
            track_row_height: if android { 24.0 } else { 28.0 },
            track_height: 4.0,
            handle_radius: if android { 8.0 } else { 7.0 },
            handle_touch_radius: 18.0,
            control_gap: if android { 2.0 } else { 4.0 },
            row_bottom_space: if android { 3.0 } else { 7.0 },
        }
    }
}

/// How [`Slider::show`] arranges the label, track and value field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliderLayout {
    /// [`SliderLayout::Compact`] in Android portrait, otherwise stacked.
    #[default]
    Auto,
    /// Label and value on one row, the full-width track below.
    Stacked,
    /// Label, track and value on a single row.
    Compact,
}

/// Where values sit along a [`Slider`] track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliderScale {
    /// Track position proportional to the value.
    #[default]
    Linear,
    /// Track position proportional to the reciprocal of the value, such as a
    /// colour temperature placed by mired while it is shown in kelvin. Low
    /// values stay on the left. A range that contains zero is shown linearly.
    Reciprocal,
}

impl SliderScale {
    /// Track coordinate of `value`, increasing with the value.
    fn position(self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Reciprocal => -1.0 / value,
        }
    }

    /// The value at track coordinate `position`; inverse of [`Self::position`].
    fn value(self, position: f64) -> f64 {
        match self {
            Self::Linear => position,
            Self::Reciprocal => -1.0 / position,
        }
    }

    /// Fraction of the track from `start` to `end` at which `value` sits.
    fn fraction(self, value: f64, start: f64, end: f64) -> f64 {
        let (start, end) = (self.position(start), self.position(end));
        let span = end - start;
        if span.abs() <= f64::EPSILON {
            0.0
        } else {
            ((self.position(value) - start) / span).clamp(0.0, 1.0)
        }
    }

    /// The value at `fraction` of the track from `start` to `end`.
    fn value_at(self, fraction: f64, start: f64, end: f64) -> f64 {
        let (start, end) = (self.position(start), self.position(end));
        self.value(start + (end - start) * fraction)
    }
}

/// What happened to a slider during one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SliderResponse {
    pub changed: bool,
    /// The user asked for a reset, even when the value was already reset.
    pub reset_requested: bool,
}

/// A labelled numeric slider. See the [module documentation](self).
#[must_use = "call `show` or `show_track` to render the slider"]
pub struct Slider<'a, Num> {
    label: &'a str,
    value: &'a mut Num,
    range: RangeInclusive<Num>,
    decimals: usize,
    step: f64,
    reset_value: f64,
    accent: Option<Color32>,
    gradient: Option<Box<dyn Fn(f32) -> Color32 + 'a>>,
    metrics: Option<SliderMetrics>,
    layout: SliderLayout,
    scale: SliderScale,
}

impl<'a, Num: Numeric + Copy> Slider<'a, Num> {
    /// The label also identifies the widget, so it must be unique among its
    /// siblings.
    pub fn new(label: &'a str, value: &'a mut Num, range: RangeInclusive<Num>) -> Self {
        Self {
            label,
            value,
            range,
            decimals: 0,
            step: 1.0,
            reset_value: 0.0,
            accent: None,
            gradient: None,
            metrics: None,
            layout: SliderLayout::Auto,
            scale: SliderScale::Linear,
        }
    }

    /// Decimal places shown and stored.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Value change per arrow-key press or dragged point in the value field.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// The value a double-click restores, clamped to the range. Never derive
    /// it from the first value shown: widgets are reused across documents.
    pub fn reset_to(mut self, reset_value: Num) -> Self {
        self.reset_value = reset_value.to_f64();
        self
    }

    /// Colours the filled part of the track, the handle and a swatch beside
    /// the label.
    pub fn accent(mut self, accent: Color32) -> Self {
        self.accent = Some(accent);
        self
    }

    /// Paints the track with `color_at(fraction)` for fractions from 0 to 1,
    /// and the handle with the colour under it.
    pub fn gradient(mut self, color_at: impl Fn(f32) -> Color32 + 'a) -> Self {
        self.gradient = Some(Box::new(color_at));
        self
    }

    /// Overrides the sizes taken from the [`Metrics`] in effect.
    pub fn metrics(mut self, metrics: SliderMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn layout(mut self, layout: SliderLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Where values sit along the track; linear by default.
    pub fn scale(mut self, scale: SliderScale) -> Self {
        self.scale = scale;
        self
    }

    /// Renders the labelled row and returns whether the value changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let compact = self.is_compact(ui);
        let (track, value, range) = self.split(ui.ctx());
        let mut changed = false;
        ui.push_id(track.label, |ui| {
            let control_width = ui.available_width().max(1.0);
            ui.vertical(|ui| {
                ui.set_width(control_width);
                ui.spacing_mut().item_spacing.y = track.metrics.control_gap;
                if compact {
                    changed = compact_row(ui, &track, value, range, control_width);
                } else {
                    changed = stacked_rows(ui, &track, value, range, control_width);
                }
            });
        });
        changed
    }

    /// Renders only the track at `width`.
    pub fn show_track(self, ui: &mut Ui, width: f32) -> SliderResponse {
        let compact = self.is_compact(ui);
        let (track, value, range) = self.split(ui.ctx());
        ui.push_id(track.label, |ui| {
            track_widget(ui, &track, value, range, width, compact)
        })
        .inner
    }

    fn is_compact(&self, ui: &Ui) -> bool {
        match self.layout {
            SliderLayout::Auto => is_compact_portrait(ui),
            SliderLayout::Stacked => false,
            SliderLayout::Compact => true,
        }
    }

    fn split(self, ctx: &egui::Context) -> (TrackOptions<'a>, &'a mut Num, RangeInclusive<Num>) {
        let start = self.range.start().to_f64();
        let end = self.range.end().to_f64();
        let scale = match self.scale {
            SliderScale::Reciprocal if start * end <= 0.0 => SliderScale::Linear,
            scale => scale,
        };
        let options = TrackOptions {
            label: self.label,
            decimals: self.decimals,
            step: self.step,
            reset_value: self.reset_value.clamp(start.min(end), start.max(end)),
            accent: self.accent,
            gradient: self.gradient,
            metrics: self.metrics.unwrap_or_else(|| SliderMetrics::of(ctx)),
            scale,
        };
        (options, self.value, self.range)
    }
}

struct TrackOptions<'a> {
    label: &'a str,
    decimals: usize,
    step: f64,
    reset_value: f64,
    accent: Option<Color32>,
    gradient: Option<Box<dyn Fn(f32) -> Color32 + 'a>>,
    metrics: SliderMetrics,
    scale: SliderScale,
}

impl TrackOptions<'_> {
    fn reset<Num: Numeric + Copy>(&self, value: &mut Num) -> bool {
        set_reset_value(value, self.reset_value)
    }
}

fn compact_row<Num: Numeric + Copy>(
    ui: &mut Ui,
    track: &TrackOptions<'_>,
    value: &mut Num,
    range: RangeInclusive<Num>,
    control_width: f32,
) -> bool {
    let metrics = track.metrics;
    let mut changed = false;
    let (label_width, track_width) = compact_widths(control_width, metrics.value_field_width);
    ui.allocate_ui_with_layout(
        egui::vec2(control_width, metrics.header_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = COMPACT_ROW_GAP;
            let label_response = compact_label(ui, track.label, track.accent, label_width, metrics);
            if label_response.double_clicked() {
                changed |= track.reset(value);
            }

            changed |= track_widget(ui, track, value, range.clone(), track_width, true).changed;

            let (value_response, value_changed) =
                value_field(ui, value, range, track.decimals, track.step, metrics);
            changed |= value_changed;
            if value_response.double_clicked() {
                changed |= track.reset(value);
            }
        },
    );
    ui.add_space(COMPACT_ROW_BOTTOM_SPACE);
    changed
}

fn stacked_rows<Num: Numeric + Copy>(
    ui: &mut Ui,
    track: &TrackOptions<'_>,
    value: &mut Num,
    range: RangeInclusive<Num>,
    control_width: f32,
) -> bool {
    let metrics = track.metrics;
    let mut changed = false;
    ui.allocate_ui_with_layout(
        egui::vec2(control_width, metrics.header_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            let label_response = if let Some(accent) = track.accent {
                let (swatch_rect, swatch_response) = ui.allocate_exact_size(
                    egui::vec2(SWATCH_RADIUS * 2.0, SWATCH_RADIUS * 2.0),
                    Sense::hover(),
                );
                paint_swatch(ui.painter(), swatch_rect.center(), accent);
                swatch_response.union(
                    ui.add(egui::Label::new(RichText::new(track.label)).sense(Sense::click())),
                )
            } else {
                ui.add(egui::Label::new(track.label).sense(Sense::click()))
            };
            if label_response.double_clicked() {
                changed |= track.reset(value);
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (value_response, value_changed) = value_field(
                    ui,
                    value,
                    range.clone(),
                    track.decimals,
                    track.step,
                    metrics,
                );
                changed |= value_changed;
                if value_response.double_clicked() {
                    changed |= track.reset(value);
                }
            });
        },
    );

    changed |= track_widget(ui, track, value, range, control_width, false).changed;
    ui.add_space(metrics.row_bottom_space);
    changed
}

/// Label and track widths of a compact row; the value field takes the rest.
fn compact_widths(control_width: f32, value_field_width: f32) -> (f32, f32) {
    let usable_width = (control_width - value_field_width - COMPACT_ROW_GAP * 2.0).max(2.0);
    let label_width = (control_width * 0.27)
        .clamp(COMPACT_LABEL_MIN_WIDTH, COMPACT_LABEL_MAX_WIDTH)
        .min((usable_width - COMPACT_TRACK_MIN_WIDTH).max(1.0));
    (label_width, (usable_width - label_width).max(1.0))
}

fn paint_swatch(painter: &egui::Painter, center: egui::Pos2, accent: Color32) {
    painter.circle_filled(center, SWATCH_RADIUS, accent);
    painter.circle_stroke(
        center,
        SWATCH_RADIUS,
        Stroke::new(1.0, Color32::from_white_alpha(90)),
    );
}

fn compact_label(
    ui: &mut Ui,
    label: &str,
    accent: Option<Color32>,
    width: f32,
    metrics: SliderMetrics,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, metrics.header_height), Sense::click());
    let painter = ui.painter_at(rect);
    let mut text_x = rect.left();
    if let Some(accent) = accent {
        paint_swatch(
            &painter,
            egui::pos2(rect.left() + 5.0, rect.center().y),
            accent,
        );
        text_x += 15.0;
    }
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (rect.right() - text_x).max(1.0),
        egui::TextStyle::Body,
    );
    painter.galley(
        egui::pos2(text_x, rect.center().y - galley.size().y * 0.5),
        galley,
        ui.visuals().text_color(),
    );
    response
}

/// An editable [`NumberField`], or a read-only readout on touch screens where
/// a keyboard would cover the slider.
pub(crate) fn value_field<Num: Numeric + Copy>(
    ui: &mut Ui,
    value: &mut Num,
    range: RangeInclusive<Num>,
    decimals: usize,
    step: f64,
    metrics: SliderMetrics,
) -> (egui::Response, bool) {
    let size = egui::vec2(metrics.value_field_width, metrics.header_height);
    if ui.input(|input| input.has_touch_screen()) {
        return (
            touch_value_readout(ui, value.to_f64(), decimals, size),
            false,
        );
    }
    ui.allocate_ui_with_layout(
        size,
        Layout::centered_and_justified(ui.layout().main_dir()),
        |ui| {
            let response = ui.add(
                NumberField::new(value, range)
                    .speed(step)
                    .decimals(decimals),
            );
            let changed = response.changed();
            (response, changed)
        },
    )
    .inner
}

fn touch_value_readout(
    ui: &mut Ui,
    value: f64,
    decimals: usize,
    size: egui::Vec2,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let visuals = ui.style().interact(&response);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
    painter.rect_stroke(
        rect,
        visuals.corner_radius,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{value:.decimals$}"),
        egui::TextStyle::Monospace.resolve(ui.style()),
        visuals.fg_stroke.color,
    );
    response
}

fn track_widget<Num: Numeric + Copy>(
    ui: &mut Ui,
    options: &TrackOptions<'_>,
    value: &mut Num,
    range: RangeInclusive<Num>,
    width: f32,
    compact: bool,
) -> SliderResponse {
    let metrics = options.metrics;
    let decimals = options.decimals;
    let start = range.start().to_f64();
    let end = range.end().to_f64();
    let span = end - start;
    let scale = options.scale;
    let fraction = scale.fraction(value.to_f64(), start, end) as f32;

    let row_height = if compact {
        metrics.header_height
    } else {
        metrics.track_row_height
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_height), Sense::hover());
    let track_rect = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(
            (rect.width() - metrics.handle_radius * 2.0).max(1.0),
            metrics.track_height,
        ),
    );
    let handle_x = egui::lerp(track_rect.left()..=track_rect.right(), fraction);
    let handle_center = egui::pos2(handle_x, track_rect.center().y);
    let handle_hit_rect = egui::Rect::from_center_size(
        handle_center,
        egui::Vec2::splat(metrics.handle_touch_radius * 2.0),
    )
    .intersect(rect);

    let track_response = ui.interact(rect, ui.id().with("guarded-track"), Sense::click());
    let handle_response = ui.interact(
        handle_hit_rect,
        ui.id().with("guarded-handle"),
        Sense::click(),
    );

    let enabled = track_response.enabled() && handle_response.enabled();
    let drag_id = ui.id().with("guarded-slider-drag");
    if !enabled && scroll_lock_owner(ui.ctx()) == Some(drag_id) {
        ui.ctx().stop_dragging();
        ui.ctx()
            .data_mut(|data| data.remove::<egui::Id>(scroll_lock_id()));
    }

    if enabled && track_response.clicked() {
        track_response.request_focus();
    }
    if enabled && handle_response.clicked() {
        handle_response.request_focus();
    }

    let mut changed = false;
    let reset_requested =
        enabled && (track_response.double_clicked() || handle_response.double_clicked());
    if reset_requested {
        changed |= options.reset(value);
    }
    let (press_origin, pointer_pos, pointer_down) = if enabled {
        ui.input(|input| {
            (
                input.pointer.press_origin(),
                input.pointer.interact_pos(),
                input.pointer.any_down(),
            )
        })
    } else {
        (None, None, false)
    };
    let mut drag_active = pointer_down && scroll_lock_owner(ui.ctx()) == Some(drag_id);
    let (touch, now, released) = ui.input(|input| {
        (
            input.has_touch_screen(),
            input.time,
            input.pointer.any_released(),
        )
    });
    let touch_drag_id = drag_id.with("touch");
    let mut touch_drag = ui.ctx().data_mut(|data| {
        let touch_drag = data.get_temp::<TouchDrag>(touch_drag_id);
        data.remove::<TouchDrag>(touch_drag_id);
        touch_drag
    });
    if !drag_active {
        // A touch drag of this slider ended this frame or earlier; only one
        // released this frame may still settle the value.
        if let (Some(touch_drag), Some(position)) = (touch_drag.take(), pointer_pos) {
            let settled = touch_drag.settled_fraction(position.x, now);
            if let Some(settled) = settled.filter(|_| released && !pointer_down && !reset_requested)
            {
                changed |= set_from_fraction(value, scale, start, end, decimals, settled);
            }
        }
    }
    let owns_pointer =
        enabled && (track_response.contains_pointer() || handle_response.contains_pointer());
    if enabled && !reset_requested {
        if let (Some(origin), Some(position), true) = (press_origin, pointer_pos, pointer_down) {
            if drag_active {
                lock_slider_scroll(ui.ctx(), drag_id);
                if touch {
                    let touch_drag = touch_drag.get_or_insert_with(|| {
                        TouchDrag::start(scale.fraction(value.to_f64(), start, end), position, now)
                    });
                    if touch_drag.follow(position, now, rect, track_rect.width()) {
                        changed |= set_from_fraction(
                            value,
                            scale,
                            start,
                            end,
                            decimals,
                            touch_drag.fraction,
                        );
                    }
                } else {
                    changed |= set_from_pointer(
                        value, scale, start, end, decimals, track_rect, position.x,
                    );
                }
            } else if rect.contains(origin) && owns_pointer {
                let delta = position - origin;
                let threshold = if handle_hit_rect.contains(origin) {
                    HANDLE_DRAG_THRESHOLD
                } else {
                    TRACK_DRAG_THRESHOLD
                };
                let horizontal_intent = delta.x.abs() >= threshold
                    && delta.x.abs() >= delta.y.abs() * HORIZONTAL_INTENT_RATIO;
                if horizontal_intent {
                    drag_active = true;
                    lock_slider_scroll(ui.ctx(), drag_id);
                    track_response.request_focus();
                    if touch {
                        // Movement before the drag was recognised is not
                        // applied, so claiming the drag never moves the value.
                        touch_drag = Some(TouchDrag::start(
                            scale.fraction(value.to_f64(), start, end),
                            position,
                            now,
                        ));
                    } else {
                        changed |= set_from_pointer(
                            value, scale, start, end, decimals, track_rect, position.x,
                        );
                    }
                }
            }
        }
    }
    if let Some(touch_drag) = touch_drag.filter(|_| drag_active) {
        ui.ctx()
            .data_mut(|data| data.insert_temp(touch_drag_id, touch_drag));
    }

    let focused = track_response.has_focus() || handle_response.has_focus();
    let (min, max) = (start.min(end), start.max(end));
    if enabled {
        let mut steps = 0_i64;
        if focused {
            let decrease = ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            });
            let increase = ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
            });
            steps += i64::from(increase) - i64::from(decrease);
        }
        let set_value = ui.input(|input| {
            use egui::accesskit::{Action, ActionData};
            let id = track_response.id;
            steps += input.num_accesskit_action_requests(id, Action::Increment) as i64;
            steps -= input.num_accesskit_action_requests(id, Action::Decrement) as i64;
            input
                .accesskit_action_requests(id, Action::SetValue)
                .filter_map(|request| match request.data {
                    Some(ActionData::NumericValue(value)) => Some(value),
                    _ => None,
                })
                .last()
        });
        if steps != 0 {
            let next = (value.to_f64() + steps as f64 * options.step).clamp(min, max);
            changed |= set_numeric(value, next, decimals);
        }
        if let Some(next) = set_value {
            changed |= set_numeric(value, next.clamp(min, max), decimals);
        }
    }
    let current = value.to_f64();
    track_response.widget_info(|| egui::WidgetInfo::slider(enabled, current, options.label));
    ui.ctx()
        .accesskit_node_builder(track_response.id, |builder| {
            use egui::accesskit::Action;
            builder.set_min_numeric_value(min);
            builder.set_max_numeric_value(max);
            builder.set_numeric_value_step(options.step);
            if enabled {
                builder.add_action(Action::SetValue);
                if current < max {
                    builder.add_action(Action::Increment);
                }
                if current > min {
                    builder.add_action(Action::Decrement);
                }
            }
        });

    let active = drag_active
        || handle_response.is_pointer_button_down_on()
        || (track_response.is_pointer_button_down_on()
            && press_origin.is_some_and(|origin| rect.contains(origin)));
    let hovered = track_response.hovered() || handle_response.hovered();
    let interaction = interaction_visuals_for_flags(ui, enabled, false, active, hovered, focused);

    let painter = ui.painter();
    let track_radius = metrics.track_height * 0.5;
    let gradient = options.gradient.as_deref();
    let visual_accent = gradient
        .map(|color_at| color_at(fraction))
        .or(options.accent);
    if let Some(color_at) = gradient {
        paint_gradient_track(painter, track_rect, track_radius, color_at);
    } else {
        painter.rect_filled(
            track_rect,
            track_radius,
            ui.visuals().widgets.inactive.bg_fill,
        );
    }
    // Only a linear range can contain zero (see `Slider::split`).
    let bipolar = start < 0.0 && end > 0.0;
    let fill_origin = if bipolar {
        let zero_fraction = ((-start) / span).clamp(0.0, 1.0) as f32;
        egui::lerp(track_rect.left()..=track_rect.right(), zero_fraction)
    } else {
        track_rect.left()
    };
    let fill_left = fill_origin.min(handle_x);
    let fill_right = fill_origin.max(handle_x);
    if gradient.is_none() && fill_right - fill_left > 0.25 {
        let fill_rect = egui::Rect::from_min_max(
            egui::pos2(fill_left, track_rect.top()),
            egui::pos2(fill_right, track_rect.bottom()),
        );
        painter.rect_filled(
            fill_rect,
            track_radius,
            visual_accent.unwrap_or(ui.visuals().selection.bg_fill),
        );
    }
    if bipolar {
        painter.vline(
            fill_origin,
            (track_rect.center().y - 5.0)..=(track_rect.center().y + 5.0),
            Stroke::new(1.0, Color32::from_white_alpha(75)),
        );
    }
    painter.circle_filled(
        handle_center,
        metrics.handle_radius,
        visual_accent.unwrap_or(interaction.fill),
    );
    painter.circle_stroke(
        handle_center,
        metrics.handle_radius,
        Stroke::new(1.0, interaction.foreground),
    );

    track_response
        .union(handle_response)
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

    SliderResponse {
        changed,
        reset_requested,
    }
}

fn set_from_pointer<Num: Numeric + Copy>(
    value: &mut Num,
    scale: SliderScale,
    start: f64,
    end: f64,
    decimals: usize,
    track_rect: egui::Rect,
    pointer_x: f32,
) -> bool {
    let fraction = ((pointer_x - track_rect.left()) / track_rect.width().max(1.0)).clamp(0.0, 1.0);
    set_from_fraction(value, scale, start, end, decimals, f64::from(fraction))
}

/// Sets the value at `fraction` of the track; returns whether it changed.
fn set_from_fraction<Num: Numeric + Copy>(
    value: &mut Num,
    scale: SliderScale,
    start: f64,
    end: f64,
    decimals: usize,
    fraction: f64,
) -> bool {
    let raw = scale.value_at(fraction, start, end);
    // The reciprocal mapping can land a hair outside the range at its ends.
    set_numeric(value, raw.clamp(start.min(end), start.max(end)), decimals)
}

/// A finger's position on the track at one frame of a [`TouchDrag`].
#[derive(Clone, Copy, Debug)]
struct TouchSample {
    /// Input time in seconds.
    time: f64,
    fraction: f64,
    pointer_x: f32,
}

/// A touch drag in progress. It moves the value relative to where the drag
/// started instead of putting it under the finger, which a fingertip covers
/// and cannot place precisely.
#[derive(Clone, Debug)]
struct TouchDrag {
    /// Track fraction of the value, unrounded so that movements smaller than
    /// one displayed step still add up.
    fraction: f64,
    last_x: f32,
    /// Recent samples, oldest first. The first is the newest one at least
    /// [`LIFT_OFF_WINDOW`] old, the state a release can return to.
    samples: VecDeque<TouchSample>,
}

impl TouchDrag {
    fn start(fraction: f64, pointer: egui::Pos2, time: f64) -> Self {
        let mut drag = Self {
            fraction,
            last_x: pointer.x,
            samples: VecDeque::new(),
        };
        drag.record(time);
        drag
    }

    /// Moves by the finger's horizontal travel since the last frame, scaled
    /// down with its distance beyond `row`; returns whether the fraction moved.
    fn follow(
        &mut self,
        pointer: egui::Pos2,
        time: f64,
        row: egui::Rect,
        track_width: f32,
    ) -> bool {
        let beyond_row = ((pointer.y - row.center().y).abs() - row.height() * 0.5).max(0.0);
        let gain = 1.0 / (1.0 + beyond_row / FINE_TUNE_DISTANCE);
        let travel = (pointer.x - self.last_x) * gain / track_width.max(1.0);
        self.last_x = pointer.x;
        let previous = self.fraction;
        self.fraction = (self.fraction + f64::from(travel)).clamp(0.0, 1.0);
        self.record(time);
        self.fraction != previous
    }

    fn record(&mut self, time: f64) {
        self.samples.push_back(TouchSample {
            time,
            fraction: self.fraction,
            pointer_x: self.last_x,
        });
        self.trim(time);
    }

    /// Drops samples older than needed for a release at `time`.
    fn trim(&mut self, time: f64) {
        while self
            .samples
            .get(1)
            .is_some_and(|next| next.time <= time - LIFT_OFF_WINDOW)
        {
            self.samples.pop_front();
        }
    }

    /// The fraction to keep for a release at `pointer_x` and `time`, when it
    /// differs from the current one: the fraction from before the finger
    /// rolled off if it moved less than [`LIFT_OFF_SLOP`] meanwhile.
    fn settled_fraction(mut self, pointer_x: f32, time: f64) -> Option<f64> {
        self.trim(time);
        let before = self.samples.front()?;
        ((pointer_x - before.pointer_x).abs() <= LIFT_OFF_SLOP && before.fraction != self.fraction)
            .then_some(before.fraction)
    }
}

fn paint_gradient_track(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    color_at: &dyn Fn(f32) -> Color32,
) {
    for segment in 0..GRADIENT_SEGMENTS {
        let left_fraction = segment as f32 / GRADIENT_SEGMENTS as f32;
        let right_fraction = (segment + 1) as f32 / GRADIENT_SEGMENTS as f32;
        let left = egui::lerp(rect.left()..=rect.right(), left_fraction);
        let right = egui::lerp(rect.left()..=rect.right(), right_fraction);
        let color = color_at(((left_fraction + right_fraction) * 0.5).clamp(0.0, 1.0));
        // Overlap neighbours slightly so no background shows between them.
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(left - 0.25, rect.top()),
                egui::pos2(right + 0.25, rect.bottom()),
            ),
            0.0,
            color,
        );
    }
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(0.75, Color32::from_black_alpha(100)),
        egui::StrokeKind::Inside,
    );
}

/// Stores a caller-supplied reset value exactly; returns whether it changed.
/// Unlike [`set_numeric`] it keeps decimals the control does not display, so a
/// reset restores a precise default such as a per-photo starting value.
pub(crate) fn set_reset_value<Num: Numeric + Copy>(value: &mut Num, raw: f64) -> bool {
    let next = Num::from_f64(if Num::INTEGRAL { raw.round() } else { raw });
    if next == *value {
        false
    } else {
        *value = next;
        true
    }
}

/// Rounds `raw` to `decimals` places and stores it; returns whether it changed.
pub(crate) fn set_numeric<Num: Numeric + Copy>(value: &mut Num, raw: f64, decimals: usize) -> bool {
    let scale = 10_f64.powi(decimals.min(12) as i32);
    let next = Num::from_f64((raw * scale).round() / scale);
    if next == *value {
        false
    } else {
        *value = next;
        true
    }
}

fn scroll_lock_id() -> egui::Id {
    egui::Id::new("moduwu-slider-scroll-lock")
}

/// Whether a slider owns the current pointer drag; scroll areas should not
/// scroll while it does. Clears the lock once the pointer is released.
pub fn slider_scroll_locked(ctx: &egui::Context) -> bool {
    if !ctx.input(|input| input.pointer.any_down()) {
        ctx.data_mut(|data| data.remove::<egui::Id>(scroll_lock_id()));
        return false;
    }
    scroll_lock_owner(ctx).is_some()
}

/// Claims the current drag for the widget `id` so that
/// [`slider_scroll_locked`] reports it.
pub fn lock_slider_scroll(ctx: &egui::Context, id: egui::Id) {
    ctx.data_mut(|data| data.insert_temp(scroll_lock_id(), id));
    ctx.set_dragged_id(id);
}

fn scroll_lock_owner(ctx: &egui::Context) -> Option<egui::Id> {
    let owner = ctx.data(|data| data.get_temp::<egui::Id>(scroll_lock_id()));
    if owner.is_some_and(|owner| ctx.is_being_dragged(owner)) {
        return owner;
    }
    // Screens without a scroll area never call `slider_scroll_locked` while
    // the pointer is up. Do not let their stale owner turn a later press
    // elsewhere into another slider drag.
    if owner.is_some() {
        ctx.data_mut(|data| data.remove::<egui::Id>(scroll_lock_id()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Event, Key, Modifiers, PointerButton, RawInput};

    const METRICS: SliderMetrics = SliderMetrics::CURRENT;
    /// Vertical centre of the track row in a stacked layout at the top left.
    const TRACK_Y: f32 =
        METRICS.header_height + METRICS.control_gap + METRICS.track_row_height * 0.5;

    fn input(time: f64, events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 240.0),
            )),
            time: Some(time),
            events,
            ..RawInput::default()
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

    fn drag(from: egui::Pos2, to: egui::Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(from),
            press(from, true),
            Event::PointerMoved(to),
        ]
    }

    /// Drives a stacked 0..=100 slider, optionally disabled or covered by a
    /// foreground bar over its track.
    struct Harness {
        ctx: egui::Context,
        time: f64,
        value: u8,
        enabled: bool,
        covered: bool,
    }

    impl Harness {
        fn new() -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                time: 0.0,
                value: 50,
                enabled: true,
                covered: false,
            };
            harness.frame(Vec::new());
            harness
        }

        fn frame(&mut self, events: Vec<Event>) {
            let (enabled, covered) = (self.enabled, self.covered);
            let value = &mut self.value;
            self.time += 0.05;
            let _ = self.ctx.run_ui(input(self.time, events), |ui| {
                ui.set_width(400.0);
                ui.add_enabled_ui(enabled, |ui| {
                    Slider::new("Quality", value, 1..=100)
                        .layout(SliderLayout::Stacked)
                        .show(ui);
                });
                if covered {
                    egui::Area::new(egui::Id::new("test-bottom-bar"))
                        .order(egui::Order::Foreground)
                        .fixed_pos(pos2(0.0, METRICS.header_height + METRICS.control_gap))
                        .show(ui.ctx(), |ui| {
                            ui.allocate_exact_size(
                                egui::vec2(400.0, METRICS.track_row_height),
                                Sense::click_and_drag(),
                            );
                        });
                }
            });
        }

        fn click(&mut self, pos: egui::Pos2) {
            for pressed in [true, false] {
                self.frame(vec![Event::PointerMoved(pos), press(pos, pressed)]);
            }
        }

        /// A harness whose input has seen a touch screen, away from the slider.
        fn touch() -> Self {
            let mut harness = Self::new();
            let touch = |phase| Event::Touch {
                device_id: egui::TouchDeviceId(1),
                id: egui::TouchId(1),
                phase,
                pos: pos2(390.0, 230.0),
                force: None,
            };
            harness.frame(vec![
                touch(egui::TouchPhase::Start),
                touch(egui::TouchPhase::End),
            ]);
            harness
        }

        fn move_to(&mut self, pos: egui::Pos2) {
            self.frame(vec![Event::PointerMoved(pos)]);
        }

        fn release(&mut self, pos: egui::Pos2) {
            self.frame(vec![Event::PointerMoved(pos), press(pos, false)]);
        }
    }

    /// Track width of the harness slider and the x of its handle at 50.
    fn harness_track() -> (f32, f32) {
        let width = 400.0 - METRICS.handle_radius * 2.0;
        (width, METRICS.handle_radius + width * 49.0 / 99.0)
    }

    /// The harness value after moving `points` along its track from 50.
    fn harness_value_after(points: f32) -> u8 {
        let (width, _) = harness_track();
        (50.0 + points / width * 99.0).round() as u8
    }

    #[test]
    fn double_click_resets_to_the_explicit_value_never_the_first_one_shown() {
        // The same widget id is reused across documents with different values
        // and defaults; a reset must not remember the value first displayed.
        for gradient in [false, true] {
            for click_label in [false, true] {
                let ctx = egui::Context::default();
                let mut time = 0.0;
                for (initial, reset) in [
                    (37.0_f32, None),
                    (-24.0, None),
                    (81.0, Some(50.0)),
                    (72.0, Some(65.0)),
                ] {
                    let mut value = initial;
                    let mut show = |events| {
                        time += 0.05;
                        let _ = ctx.run_ui(input(time, events), |ui| {
                            let mut slider = Slider::new("Saturation", &mut value, -100.0..=100.0)
                                .layout(SliderLayout::Stacked);
                            if let Some(reset) = reset {
                                slider = slider.reset_to(reset);
                            }
                            if gradient {
                                slider = slider.gradient(|t| Color32::from_gray((t * 255.0) as u8));
                            }
                            slider.show(ui);
                        });
                    };
                    show(Vec::new());
                    let pos = if click_label {
                        pos2(20.0, METRICS.header_height * 0.5)
                    } else {
                        pos2(20.0, TRACK_Y)
                    };
                    for _ in 0..2 {
                        for pressed in [true, false] {
                            show(vec![Event::PointerMoved(pos), press(pos, pressed)]);
                        }
                    }
                    assert_eq!(
                        value,
                        reset.unwrap_or(0.0),
                        "gradient={gradient}, label={click_label}"
                    );
                    time += 1.0;
                }
            }
        }
    }

    impl Harness {
        fn tooltip_open(&self) -> bool {
            self.ctx.memory(|memory| {
                memory
                    .areas()
                    .visible_layer_ids()
                    .iter()
                    .any(|layer| layer.order == egui::Order::Tooltip)
            })
        }

        /// Rests the pointer at `pos` for `seconds`, one frame per 50 ms.
        fn rest(&mut self, pos: egui::Pos2, seconds: f64) {
            self.frame(vec![Event::PointerMoved(pos)]);
            for _ in 0..(seconds / 0.05) as usize {
                self.frame(Vec::new());
            }
        }
    }

    #[test]
    fn no_part_of_a_slider_shows_a_tooltip() {
        for pos in [
            pos2(20.0, METRICS.header_height * 0.5),
            pos2(200.0, TRACK_Y),
            pos2(390.0, METRICS.header_height * 0.5),
        ] {
            let mut harness = Harness::new();
            harness.rest(pos, 1.0);
            assert!(!harness.tooltip_open(), "tooltip at {pos:?}");
        }
    }

    #[test]
    fn track_reports_a_reset_even_when_the_value_is_already_reset() {
        let ctx = egui::Context::default();
        let reset_value = 0.18_f32;
        let mut value = reset_value;
        let mut reset_seen = false;
        let mut time = 0.0;
        let mut show = |events| {
            time += 0.05;
            let _ = ctx.run_ui(input(time, events), |ui| {
                let response = Slider::new("inline-reset-test", &mut value, 0.0..=1.0)
                    .decimals(5)
                    .step(0.01)
                    .reset_to(reset_value)
                    .layout(SliderLayout::Stacked)
                    .show_track(ui, 100.0);
                reset_seen |= response.reset_requested;
            });
        };
        show(Vec::new());
        let pos = pos2(50.0, METRICS.track_row_height * 0.5);
        for _ in 0..2 {
            for pressed in [true, false] {
                show(vec![Event::PointerMoved(pos), press(pos, pressed)]);
            }
        }
        assert!(reset_seen);
        assert_eq!(value, reset_value);
    }

    #[test]
    fn reset_keeps_decimals_the_slider_does_not_display() {
        let mut value = 40.0_f32;
        let slider = Slider::new("precise-reset", &mut value, 0.0..=100.0)
            .decimals(0)
            .reset_to(1.18);
        let (track, value, _) = slider.split(&egui::Context::default());
        assert!(track.reset(value));
        assert_eq!(*value, 1.18);
    }

    #[test]
    fn reset_value_is_clamped_to_the_range() {
        let mut value = 5_u8;
        let mut slider = Slider::new("clamped", &mut value, 1..=100);
        slider.reset_value = 0.0;
        let (track, value, _) = slider.split(&egui::Context::default());
        assert!(track.reset(value));
        assert_eq!(*value, 1);
    }

    #[test]
    fn reciprocal_scale_places_values_by_their_reciprocal() {
        let scale = SliderScale::Reciprocal;
        let midpoint = 2.0 / (1.0 / 2_000.0 + 1.0 / 20_000.0);
        assert!((scale.fraction(midpoint, 2_000.0, 20_000.0) - 0.5).abs() < 1e-12);
        assert!((scale.value_at(0.5, 2_000.0, 20_000.0) - midpoint).abs() < 1e-9);
        assert_eq!(scale.fraction(2_000.0, 2_000.0, 20_000.0), 0.0);
        assert_eq!(scale.fraction(20_000.0, 2_000.0, 20_000.0), 1.0);
        assert_eq!(
            SliderScale::Linear.fraction(5_000.0, 2_000.0, 20_000.0),
            3_000.0 / 18_000.0
        );
    }

    #[test]
    fn dragging_a_reciprocal_slider_to_the_middle_selects_the_reciprocal_midpoint() {
        let ctx = egui::Context::default();
        let mut value = 20_000.0_f32;
        let mut time = 0.0;
        let mut show = |events| {
            time += 0.05;
            let _ = ctx.run_ui(input(time, events), |ui| {
                ui.set_width(400.0);
                Slider::new("Temperature", &mut value, 2_000.0..=20_000.0)
                    .scale(SliderScale::Reciprocal)
                    .layout(SliderLayout::Stacked)
                    .show(ui);
            });
        };
        show(Vec::new());
        let handle = pos2(400.0 - METRICS.handle_radius, TRACK_Y);
        show(drag(handle, pos2(200.0, TRACK_Y)));
        assert_eq!(value, 3_636.0);
    }

    #[test]
    fn reciprocal_scale_over_zero_falls_back_to_linear() {
        let mut value = 0.5_f32;
        let slider = Slider::new("bipolar", &mut value, -1.0..=1.0).scale(SliderScale::Reciprocal);
        let (track, _, _) = slider.split(&egui::Context::default());
        assert_eq!(track.scale, SliderScale::Linear);
    }

    #[test]
    fn compact_row_uses_the_available_width() {
        for metrics in [SliderMetrics::DESKTOP, SliderMetrics::ANDROID] {
            let width = 360.0;
            let (label, track) = compact_widths(width, metrics.value_field_width);
            let total = label + track + metrics.value_field_width + COMPACT_ROW_GAP * 2.0;
            assert!((total - width).abs() < 0.001);
            assert!(track >= COMPACT_TRACK_MIN_WIDTH);
        }
    }

    #[test]
    fn header_row_keeps_the_platform_touch_target() {
        assert_eq!(
            SliderMetrics::DESKTOP.header_height,
            Metrics::DESKTOP.control_height
        );
        assert_eq!(
            SliderMetrics::ANDROID.header_height,
            Metrics::ANDROID.control_height
        );
        const { assert!(SliderMetrics::ANDROID.handle_radius >= SliderMetrics::DESKTOP.handle_radius) };
    }

    #[test]
    fn vertical_drag_on_the_track_is_left_to_the_scroll_area() {
        let mut harness = Harness::new();
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(104.0, TRACK_Y + 60.0)));
        assert_eq!(harness.value, 50);
        assert!(!slider_scroll_locked(&harness.ctx));
    }

    #[test]
    fn completed_drag_does_not_capture_a_later_drag_elsewhere() {
        let mut harness = Harness::new();
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(300.0, TRACK_Y)));
        let dragged = harness.value;
        assert_ne!(dragged, 50);
        assert!(slider_scroll_locked(&harness.ctx));

        harness.frame(vec![press(pos2(300.0, TRACK_Y), false)]);
        harness.frame(drag(pos2(100.0, 160.0), pos2(300.0, 160.0)));
        assert_eq!(harness.value, dragged);
    }

    #[test]
    fn touch_drag_moves_the_value_by_the_finger_travel_not_to_the_finger() {
        let mut harness = Harness::touch();
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(110.0, TRACK_Y)));
        assert_eq!(
            harness.value, 50,
            "claiming the drag must not move the value"
        );
        assert!(slider_scroll_locked(&harness.ctx));

        harness.move_to(pos2(150.0, TRACK_Y));
        assert_eq!(harness.value, harness_value_after(40.0));
    }

    #[test]
    fn grabbing_the_handle_off_centre_by_touch_keeps_the_value() {
        let mut harness = Harness::touch();
        let (_, handle_x) = harness_track();
        let grab = pos2(handle_x + 12.0, TRACK_Y);
        harness.frame(drag(grab, grab + egui::vec2(3.0, 0.0)));
        assert_eq!(harness.value, 50);
    }

    #[test]
    fn touch_drag_away_from_the_track_moves_the_value_more_slowly() {
        for (beyond_row, gain) in [(0.0, 1.0), (FINE_TUNE_DISTANCE * 2.0, 1.0 / 3.0)] {
            let mut harness = Harness::touch();
            harness.frame(drag(pos2(100.0, TRACK_Y), pos2(110.0, TRACK_Y)));
            let y = TRACK_Y + METRICS.track_row_height * 0.5 + beyond_row;
            // Moving away from the track alone keeps the value.
            harness.move_to(pos2(110.0, y));
            assert_eq!(harness.value, 50);
            harness.move_to(pos2(170.0, y));
            assert_eq!(
                harness.value,
                harness_value_after(60.0 * gain),
                "{beyond_row}"
            );
        }
    }

    #[test]
    fn touch_release_discards_the_shift_of_a_lifting_finger() {
        let mut harness = Harness::touch();
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(110.0, TRACK_Y)));
        harness.move_to(pos2(150.0, TRACK_Y));
        let set = harness.value;
        // The finger rolls 3 points within the last two frames before lifting.
        harness.move_to(pos2(152.0, TRACK_Y));
        assert_ne!(harness.value, set);
        harness.release(pos2(153.0, TRACK_Y));
        assert_eq!(harness.value, set);
        assert!(!slider_scroll_locked(&harness.ctx));
    }

    #[test]
    fn touch_release_keeps_a_deliberate_final_movement() {
        let mut harness = Harness::touch();
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(110.0, TRACK_Y)));
        harness.move_to(pos2(150.0, TRACK_Y));
        harness.move_to(pos2(180.0, TRACK_Y));
        let moved = harness.value;
        assert_eq!(moved, harness_value_after(70.0));
        harness.release(pos2(180.0, TRACK_Y));
        assert_eq!(harness.value, moved);
    }

    #[test]
    fn touch_release_without_movement_keeps_the_value_after_a_later_press() {
        // A finished touch drag must not settle the value again on a later
        // release elsewhere.
        let mut harness = Harness::touch();
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(110.0, TRACK_Y)));
        harness.move_to(pos2(150.0, TRACK_Y));
        harness.frame(Vec::new());
        harness.frame(Vec::new());
        harness.release(pos2(150.0, TRACK_Y));
        let set = harness.value;
        assert_eq!(set, harness_value_after(40.0));
        harness.click(pos2(200.0, 160.0));
        assert_eq!(harness.value, set);
    }

    #[test]
    fn foreground_layer_blocks_a_drag_beneath_it() {
        let mut harness = Harness::new();
        harness.covered = true;
        harness.frame(Vec::new());
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(300.0, TRACK_Y)));
        assert_eq!(harness.value, 50);
    }

    #[test]
    fn disabled_slider_does_not_start_or_continue_a_drag() {
        let mut harness = Harness::new();
        harness.enabled = false;
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(300.0, TRACK_Y)));
        assert_eq!(harness.value, 50);
        assert!(!slider_scroll_locked(&harness.ctx));

        harness.enabled = true;
        harness.frame(drag(pos2(100.0, TRACK_Y), pos2(300.0, TRACK_Y)));
        let dragged = harness.value;
        assert_ne!(dragged, 50);
        assert!(slider_scroll_locked(&harness.ctx));

        // Disabling mid-drag releases the drag and the scroll lock.
        harness.enabled = false;
        harness.frame(vec![Event::PointerMoved(pos2(350.0, TRACK_Y))]);
        assert_eq!(harness.value, dragged);
        assert!(!slider_scroll_locked(&harness.ctx));
        assert!(harness.ctx.dragged_id().is_none());
    }

    #[test]
    fn focused_track_consumes_arrow_keys_for_one_step() {
        let mut harness = Harness::new();
        harness.click(pos2(200.0, TRACK_Y));
        harness.frame(vec![key(Key::ArrowRight, true)]);
        assert_eq!(harness.value, 51);
        assert!(!harness
            .ctx
            .input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowRight)));
        harness.frame(vec![key(Key::ArrowRight, false), key(Key::ArrowDown, true)]);
        assert_eq!(harness.value, 50);
    }

    #[test]
    fn disabled_focused_track_ignores_arrow_keys() {
        let mut harness = Harness::new();
        harness.click(pos2(200.0, TRACK_Y));
        harness.enabled = false;
        harness.frame(vec![key(Key::ArrowRight, true)]);
        assert_eq!(harness.value, 50);
    }

    #[test]
    fn track_is_an_accessible_slider_that_follows_its_actions() {
        use egui::accesskit::{Action, ActionData, ActionRequest, Role};

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut value = 40_u8;
        let mut run = |events| {
            let output = ctx.run_ui(input(0.0, events), |ui| {
                Slider::new("Quality", &mut value, 0..=100)
                    .layout(SliderLayout::Stacked)
                    .show(ui);
            });
            let update = output
                .platform_output
                .accesskit_update
                .expect("AccessKit output is enabled");
            let (id, node) = update
                .nodes
                .into_iter()
                .find(|(_, node)| node.role() == Role::Slider)
                .expect("the track is a slider");
            (id, node, value)
        };
        let (id, slider, _) = run(Vec::new());
        assert_eq!(slider.label(), Some("Quality"));
        assert_eq!(slider.numeric_value(), Some(40.0));
        assert_eq!(slider.min_numeric_value(), Some(0.0));
        assert_eq!(slider.max_numeric_value(), Some(100.0));
        for action in [Action::Increment, Action::Decrement, Action::SetValue] {
            assert!(slider.supports_action(action));
        }

        let request = |action, data| {
            Event::AccessKitActionRequest(ActionRequest {
                action,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: id,
                data,
            })
        };
        let (_, slider, value) = run(vec![request(Action::Increment, None)]);
        assert_eq!(value, 41);
        assert_eq!(slider.numeric_value(), Some(41.0));
        // Requested values are clamped and rounded like dragged ones.
        run(vec![request(
            Action::SetValue,
            Some(ActionData::NumericValue(250.0)),
        )]);
        let (_, slider, value) = run(Vec::new());
        assert_eq!(value, 100);
        assert!(!slider.supports_action(Action::Increment));
    }
}
