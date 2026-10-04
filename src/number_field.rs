//! An editable number: a drag value with keyboard stepping, a display
//! precision that adapts while editing, and an optional unit suffix.
//!
//! Built on `egui::DragValue`, which supplies the accessibility information
//! (role, value, focus, enabled state) and the increment/decrement and
//! set-value actions.

use egui::emath::Numeric;
use egui::{DragValue, Id, Key, Modifiers, Response, Ui, Widget};
use std::ops::RangeInclusive;
use std::time::Duration;

/// Delay before a held Up/Down arrow starts repeating, in seconds.
const ARROW_REPEAT_DELAY: f64 = 0.35;
/// Interval between repeated steps while an arrow stays held, in seconds.
const ARROW_REPEAT_INTERVAL: f64 = 0.05;

#[derive(Clone, Copy, Default)]
struct ArrowRepeat {
    direction: i8,
    next_at: f64,
}

/// A numeric field for forms and value boxes.
///
/// While focused, Up/Down step the value (see [`arrow_step`]) and repeat
/// after a short delay. Fractional values show [`NumberField::editing_decimals`]
/// while focused or whenever they are not whole, and
/// [`NumberField::decimals`] otherwise.
#[must_use = "add the field with `ui.add(field)`"]
pub struct NumberField<'a, Num: Numeric> {
    value: &'a mut Num,
    range: RangeInclusive<Num>,
    speed: f64,
    decimals: usize,
    editing_decimals: usize,
    suffix: String,
    commit_on_finish: bool,
    id_salt: Option<Id>,
}

impl<'a, Num: Numeric + Copy> NumberField<'a, Num> {
    pub fn new(value: &'a mut Num, range: RangeInclusive<Num>) -> Self {
        Self {
            value,
            range,
            speed: 1.0,
            decimals: 0,
            editing_decimals: 2,
            suffix: String::new(),
            commit_on_finish: false,
            id_salt: None,
        }
    }

    /// Value change per dragged point.
    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    /// Decimals shown while not editing a whole value.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Decimals shown for fractional types while focused or not whole. The
    /// larger of this and [`NumberField::decimals`] applies.
    pub fn editing_decimals(mut self, decimals: usize) -> Self {
        self.editing_decimals = decimals;
        self
    }

    /// Text after the number, e.g. `" px"`.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Apply typed text only when editing finishes (Enter or focus loss)
    /// instead of on every keystroke. Dragging and arrows still apply live.
    pub fn commit_on_finish(mut self, commit_on_finish: bool) -> Self {
        self.commit_on_finish = commit_on_finish;
        self
    }

    /// A stable identity independent of the widget's position in the layout.
    pub fn id_salt(mut self, id_salt: impl egui::AsId) -> Self {
        self.id_salt = Some(Id::new(id_salt));
        self
    }

    fn show(self, ui: &mut Ui) -> Response {
        // `DragValue` takes the next automatic ID; reading it first lets the
        // arrow keys act on the field's focus before the field is drawn.
        let id = ui.next_auto_id();
        let stepped = step_focused(ui, id, self.value, &self.range);
        let fractional = (self.value.to_f64() - self.value.to_f64().round()).abs() > 0.0001;
        let decimals = if !Num::INTEGRAL && (ui.memory(|memory| memory.has_focus(id)) || fractional)
        {
            self.decimals.max(self.editing_decimals)
        } else {
            self.decimals
        };
        let mut response = ui.add(
            DragValue::new(self.value)
                .range(self.range)
                .speed(self.speed)
                .fixed_decimals(decimals)
                .suffix(self.suffix)
                .update_while_editing(!self.commit_on_finish),
        );
        if stepped {
            response.mark_changed();
        }
        response
    }
}

impl<Num: Numeric + Copy> Widget for NumberField<'_, Num> {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.id_salt {
            Some(salt) => ui.push_id(salt, |ui| self.show(ui)).inner,
            None => self.show(ui),
        }
    }
}

/// The value change of one Up/Down step: 1 for integers, otherwise 0.2 % of
/// the larger range endpoint and at least 0.01 (0.01 for ±5, 0.2 for ±100).
pub fn arrow_step<Num: Numeric>(range: &RangeInclusive<Num>) -> f64 {
    if Num::INTEGRAL {
        return 1.0;
    }
    let maximum = range.start().to_f64().abs().max(range.end().to_f64().abs());
    (maximum / 500.0).max(0.01)
}

/// Steps `value` by [`arrow_step`] when the widget `id` has focus and Up or
/// Down is pressed or held; returns whether the value changed. Fractional
/// values are rounded to hundredths.
pub fn step_focused<Num: Numeric + Copy>(
    ui: &mut Ui,
    id: Id,
    value: &mut Num,
    range: &RangeInclusive<Num>,
) -> bool {
    let direction = focused_arrow_direction(ui, id);
    if direction == 0 {
        return false;
    }
    let start = range.start().to_f64();
    let end = range.end().to_f64();
    let next = (value.to_f64() + f64::from(direction) * arrow_step(range))
        .clamp(start.min(end), start.max(end));
    let decimals = if Num::INTEGRAL { 0 } else { 2 };
    let scale = 10_f64.powi(decimals);
    let next = Num::from_f64((next * scale).round() / scale);
    if next == *value {
        return false;
    }
    *value = next;
    // `DragValue` caches the text being edited under its widget ID.
    ui.data_mut(|data| data.remove_temp::<String>(id));
    true
}

/// -1, 0 or 1 for this frame's arrow step on the focused widget `id`,
/// consuming the key press and applying the repeat delay and interval.
fn focused_arrow_direction(ui: &mut Ui, id: Id) -> i8 {
    let repeat_id = id.with("arrow-repeat");
    if !ui.is_enabled() || !ui.memory(|memory| memory.has_focus(id)) {
        ui.data_mut(|data| data.remove_temp::<ArrowRepeat>(repeat_id));
        return 0;
    }
    let (now, up, down) = ui.input_mut(|input| {
        let up_pressed = input.consume_key(Modifiers::NONE, Key::ArrowUp);
        let down_pressed = input.consume_key(Modifiers::NONE, Key::ArrowDown);
        (
            input.time,
            input.key_down(Key::ArrowUp) || up_pressed,
            input.key_down(Key::ArrowDown) || down_pressed,
        )
    });
    let direction = i8::from(up) - i8::from(down);
    if direction == 0 {
        ui.data_mut(|data| data.remove_temp::<ArrowRepeat>(repeat_id));
        return 0;
    }
    let previous = ui.data(|data| data.get_temp::<ArrowRepeat>(repeat_id));
    let (step, next_at) = match previous {
        Some(previous) if previous.direction == direction && now < previous.next_at => {
            (0, previous.next_at)
        }
        Some(previous) if previous.direction == direction => {
            (direction, now + ARROW_REPEAT_INTERVAL)
        }
        _ => (direction, now + ARROW_REPEAT_DELAY),
    };
    ui.data_mut(|data| data.insert_temp(repeat_id, ArrowRepeat { direction, next_at }));
    ui.ctx()
        .request_repaint_after(Duration::from_secs_f64((next_at - now).max(0.0)));
    step
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Event, PointerButton, RawInput};

    fn key(key: Key, pressed: bool) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn input(time: f64, events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 200.0),
            )),
            time: Some(time),
            events,
            ..RawInput::default()
        }
    }

    /// Runs one frame with a field at the top left; returns value, focus and
    /// whether the response reported a change.
    fn frame(
        ctx: &egui::Context,
        value: &mut f32,
        limit: f32,
        time: f64,
        events: Vec<Event>,
    ) -> (bool, bool) {
        let mut state = (false, false);
        let _ = ctx.run_ui(input(time, events), |ui| {
            let response = ui.add(NumberField::new(value, -limit..=limit).speed(0.05));
            state = (response.has_focus(), response.changed());
        });
        state
    }

    fn focus(ctx: &egui::Context, value: &mut f32, limit: f32) {
        let field = pos2(10.0, 10.0);
        for (time, pressed) in [(0.01, true), (0.02, false)] {
            frame(
                ctx,
                value,
                limit,
                time,
                vec![
                    Event::PointerMoved(field),
                    Event::PointerButton {
                        pos: field,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn unfocused_fields_ignore_arrows() {
        let ctx = egui::Context::default();
        let mut value = 1.0_f32;
        frame(&ctx, &mut value, 5.0, 0.0, Vec::new());
        frame(&ctx, &mut value, 5.0, 0.001, vec![key(Key::ArrowUp, true)]);
        assert!((value - 1.0).abs() < 0.0001);
        assert!(
            ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp)),
            "an unfocused field leaves the key to others"
        );
    }

    #[test]
    fn focused_arrows_step_by_hundredths_and_repeat_after_delay() {
        let ctx = egui::Context::default();
        let mut value = 1.0_f32;
        frame(&ctx, &mut value, 5.0, 0.0, Vec::new());
        focus(&ctx, &mut value, 5.0);
        let (focused, _) = frame(&ctx, &mut value, 5.0, 0.03, Vec::new());
        assert!(focused);

        let (_, changed) = frame(&ctx, &mut value, 5.0, 0.10, vec![key(Key::ArrowUp, true)]);
        assert!(changed, "an arrow step reports a change");
        assert!((value - 1.01).abs() < 0.0001);
        frame(&ctx, &mut value, 5.0, 0.40, Vec::new());
        assert!((value - 1.01).abs() < 0.0001, "no repeat before the delay");
        frame(&ctx, &mut value, 5.0, 0.46, Vec::new());
        assert!((value - 1.02).abs() < 0.0001);
        frame(&ctx, &mut value, 5.0, 0.52, Vec::new());
        assert!((value - 1.03).abs() < 0.0001);
        frame(&ctx, &mut value, 5.0, 0.53, vec![key(Key::ArrowUp, false)]);
        frame(&ctx, &mut value, 5.0, 1.0, Vec::new());
        assert!((value - 1.03).abs() < 0.0001, "releasing stops repeating");

        frame(&ctx, &mut value, 5.0, 1.1, vec![key(Key::ArrowDown, true)]);
        assert!((value - 1.02).abs() < 0.0001);
        frame(
            &ctx,
            &mut value,
            5.0,
            1.11,
            vec![key(Key::ArrowDown, false)],
        );
        frame(&ctx, &mut value, 100.0, 1.2, vec![key(Key::ArrowUp, true)]);
        assert!(
            (value - 1.22).abs() < 0.0001,
            "the step scales with the range"
        );
    }

    #[test]
    fn steps_clamp_to_the_range() {
        let ctx = egui::Context::default();
        let mut value = 4.995_f32;
        frame(&ctx, &mut value, 5.0, 0.0, Vec::new());
        focus(&ctx, &mut value, 5.0);
        frame(&ctx, &mut value, 5.0, 0.1, vec![key(Key::ArrowUp, true)]);
        assert_eq!(value, 5.0);
    }

    #[test]
    fn arrow_step_scales_with_the_largest_endpoint() {
        assert!((arrow_step(&(-5.0_f32..=5.0)) - 0.01).abs() < 0.0001);
        assert!((arrow_step(&(-100.0_f32..=100.0)) - 0.2).abs() < 0.0001);
        assert!((arrow_step(&(0.0_f32..=100.0)) - 0.2).abs() < 0.0001);
        assert_eq!(arrow_step(&(1_u32..=100_u32)), 1.0);
    }

    #[test]
    fn the_field_exposes_its_value_to_accessibility() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut value = 42_u32;
        let output = ctx.run_ui(input(0.0, Vec::new()), |ui| {
            ui.add(NumberField::new(&mut value, 0..=100).suffix(" px"));
        });
        let update = output
            .platform_output
            .accesskit_update
            .expect("AccessKit output is enabled");
        let spin = update
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == egui::accesskit::Role::SpinButton)
            .expect("the drag value is a spin button");
        assert_eq!(spin.numeric_value(), Some(42.0));
        assert!(spin.supports_action(egui::accesskit::Action::Increment));
        assert!(spin.supports_action(egui::accesskit::Action::Decrement));
        assert!(spin.supports_action(egui::accesskit::Action::SetValue));
    }
}
