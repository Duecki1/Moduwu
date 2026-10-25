//! Double taps and double clicks that forgive the shift of a finger or hand
//! between the two presses. egui counts a double click only when the second
//! click lands within `max_click_dist` (6 points) of the first, and when its
//! release follows the first release within `max_double_click_delay` (0.3 s),
//! which a finger rarely manages.

use egui::{Id, Pos2, Response};

/// Longest time between the releases of the two taps, in seconds: Android's
/// 300 ms between one tap's release and the next press, plus the second tap.
pub const DOUBLE_TAP_DELAY: f64 = 0.4;
/// Farthest the second tap may land from the first on a touch screen, in
/// points.
const TOUCH_SLOP: f32 = 32.0;
/// Farthest the second click may land from the first with a mouse, in points.
const MOUSE_SLOP: f32 = 10.0;

/// The last click on a control.
#[derive(Clone, Copy, Debug)]
struct LastClick {
    frame: u64,
    time: f64,
    pos: Pos2,
    /// The click completed a double tap, so it cannot begin another one.
    completed_double: bool,
}

/// Whether this frame's click on `response` completes a double tap: a second
/// primary click within [`DOUBLE_TAP_DELAY`] near the first, or anything egui
/// itself counts as a double click. Call it on every frame the control is
/// shown, as it remembers the first tap. Repeated calls in one frame agree.
pub fn double_tapped(response: &Response) -> bool {
    double_tapped_on(response, response.id)
}

/// [`double_tapped`] for a control whose parts share `id`, so that the two
/// taps may land on different parts, such as a slider's track and handle.
pub fn double_tapped_on(response: &Response, id: Id) -> bool {
    if !response.clicked() {
        return false;
    }
    let ctx = &response.ctx;
    // Keyboard and accessibility activations have no position.
    let Some(pos) = response.interact_pointer_pos() else {
        return response.double_clicked();
    };
    let key = id.with("moduwu-double-tap");
    let frame = ctx.cumulative_frame_nr();
    let previous = ctx.data(|data| data.get_temp::<LastClick>(key));
    if let Some(previous) = previous.filter(|previous| previous.frame == frame) {
        return previous.completed_double;
    }
    let (time, touch) = ctx.input(|input| (input.time, input.has_touch_screen()));
    let slop = if touch { TOUCH_SLOP } else { MOUSE_SLOP };
    let double = response.double_clicked()
        || previous.is_some_and(|previous| {
            !previous.completed_double
                && time - previous.time <= DOUBLE_TAP_DELAY
                && previous.pos.distance(pos) <= slop
        });
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            LastClick {
                frame,
                time,
                pos,
                completed_double: double,
            },
        );
    });
    double
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, Event, Modifiers, PointerButton, RawInput, Sense};

    struct Harness {
        ctx: egui::Context,
        time: f64,
    }

    impl Harness {
        fn new(touch: bool) -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                time: 0.0,
            };
            let mut events = Vec::new();
            if touch {
                for phase in [egui::TouchPhase::Start, egui::TouchPhase::End] {
                    events.push(Event::Touch {
                        device_id: egui::TouchDeviceId(1),
                        id: egui::TouchId(1),
                        phase,
                        pos: pos2(390.0, 390.0),
                        force: None,
                    });
                }
            }
            harness.frame(events, 0.0);
            harness
        }

        /// Runs a frame `dt` seconds after the last one; returns how often
        /// the control reported a double tap, asked twice.
        fn frame(&mut self, events: Vec<Event>, dt: f64) -> usize {
            self.time += dt;
            let mut doubles = 0;
            let _ = self.ctx.run_ui(
                RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 400.0),
                    )),
                    time: Some(self.time),
                    events,
                    ..RawInput::default()
                },
                |ui| {
                    let response = ui.allocate_rect(
                        egui::Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(300.0, 300.0)),
                        Sense::click(),
                    );
                    doubles += usize::from(double_tapped(&response));
                    doubles += usize::from(double_tapped(&response));
                },
            );
            doubles
        }

        /// Taps at `pos` `gap` seconds after the last frame; returns how many
        /// of the two calls on the release frame reported a double tap.
        fn tap(&mut self, pos: Pos2, gap: f64) -> usize {
            let button = |pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![Event::PointerMoved(pos), button(true)], gap);
            self.frame(vec![button(false)], 0.08)
        }
    }

    #[test]
    fn touch_double_taps_forgive_a_shifted_finger() {
        let mut harness = Harness::new(true);
        assert_eq!(harness.tap(pos2(100.0, 100.0), 1.0), 0);
        // 25 points away, well past egui's 6, and repeated calls agree.
        assert_eq!(harness.tap(pos2(115.0, 120.0), 0.15), 2);
    }

    #[test]
    fn mouse_double_clicks_forgive_less() {
        let mut harness = Harness::new(false);
        harness.tap(pos2(100.0, 100.0), 1.0);
        assert_eq!(harness.tap(pos2(108.0, 100.0), 0.1), 2);
        harness.tap(pos2(100.0, 100.0), 1.0);
        assert_eq!(harness.tap(pos2(125.0, 100.0), 0.1), 0);
    }

    #[test]
    fn taps_too_far_apart_in_time_or_space_are_single() {
        let mut harness = Harness::new(true);
        harness.tap(pos2(100.0, 100.0), 1.0);
        assert_eq!(harness.tap(pos2(100.0, 100.0), 0.5), 0);
        assert_eq!(harness.tap(pos2(150.0, 100.0), 0.1), 0);
    }

    #[test]
    fn a_third_quick_tap_does_not_complete_another_double() {
        let mut harness = Harness::new(true);
        harness.tap(pos2(100.0, 100.0), 1.0);
        assert_eq!(harness.tap(pos2(110.0, 100.0), 0.1), 2);
        assert_eq!(harness.tap(pos2(120.0, 100.0), 0.1), 0);
        assert_eq!(harness.tap(pos2(120.0, 110.0), 0.1), 2);
    }
}
