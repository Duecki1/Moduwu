//! Touch holds. egui reports a long touch only after twice Android's default
//! delay and ignores the user's Touch & hold setting, so the platform layer
//! reports the delay with [`set_long_press_duration`] and [`touch_hold`] times
//! holds with it. Menus open on a long press with it, and applications can
//! use it for their own long-press gestures.

use std::time::Duration;

use egui::{Context, Id, Pos2, Response};

/// The delay used until the platform reports one: Android's default Touch &
/// hold delay.
pub const DEFAULT_LONG_PRESS: Duration = Duration::from_millis(400);

fn duration_id() -> Id {
    Id::new("moduwu-long-press-duration")
}

fn hold_id() -> Id {
    Id::new("moduwu-touch-hold")
}

fn frame_id() -> Id {
    Id::new("moduwu-touch-hold-frame")
}

/// Records how long a touch must be held to become a long press. The value
/// holds until it is replaced; a hold already under way keeps the delay it
/// began with.
pub fn set_long_press_duration(ctx: &Context, duration: Duration) {
    ctx.data_mut(|data| data.insert_temp(duration_id(), duration));
}

fn long_press_duration(ctx: &Context) -> Duration {
    ctx.data(|data| data.get_temp::<Duration>(duration_id()))
        .unwrap_or(DEFAULT_LONG_PRESS)
}

/// A touch being held.
#[derive(Clone, Copy, Debug)]
struct Hold {
    /// When the press began, which identifies it.
    press_start: f64,
    /// Seconds the touch must be held, read when it began.
    duration: f64,
    moved: bool,
    fired: bool,
}

/// What the current touch does this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TouchHold {
    /// Where a hold became a long press on this frame.
    pub long_press_at: Option<Pos2>,
    /// The touch became a long press, so lifting it is no tap.
    pub suppress_tap: bool,
}

/// What the current touch does this frame. A touch that stays within egui's
/// click distance for the long-press delay becomes a long press once, on the
/// frame the delay ends; lifting it afterwards is no tap. Every call in a
/// frame returns the same answer, so several widgets can ask.
pub fn touch_hold(ctx: &Context) -> TouchHold {
    let frame = ctx.cumulative_frame_nr();
    if let Some((computed, hold)) = ctx.data(|data| data.get_temp::<(u64, TouchHold)>(frame_id())) {
        if computed == frame {
            return hold;
        }
    }
    let hold = track(ctx);
    ctx.data_mut(|data| data.insert_temp(frame_id(), (frame, hold)));
    hold
}

/// Whether `response`'s widget became long-pressed on this frame.
pub(crate) fn long_pressed(response: &Response) -> bool {
    touch_hold(&response.ctx).long_press_at.is_some() && response.is_pointer_button_down_on()
}

fn track(ctx: &Context) -> TouchHold {
    let stored = ctx.data(|data| data.get_temp::<Hold>(hold_id()));
    let slop = ctx.options(|options| options.input_options.max_click_dist);
    let (held, press_start, origin, position, now) = ctx.input(|input| {
        (
            input.any_touches() && input.pointer.primary_down(),
            input.pointer.press_start_time(),
            input.pointer.press_origin(),
            input.pointer.interact_pos(),
            input.time,
        )
    });
    let Some(press_start) = press_start.filter(|_| held) else {
        // Lifted: this frame's tap belongs to the hold that just ended.
        ctx.data_mut(|data| data.remove::<Hold>(hold_id()));
        return TouchHold {
            long_press_at: None,
            suppress_tap: stored.is_some_and(|hold| hold.fired),
        };
    };
    let mut hold = stored
        .filter(|hold| hold.press_start == press_start)
        .unwrap_or_else(|| Hold {
            press_start,
            duration: long_press_duration(ctx).as_secs_f64(),
            moved: false,
            fired: false,
        });
    hold.moved |= origin
        .zip(position)
        .is_some_and(|(origin, position)| origin.distance(position) > slop);
    let mut long_press_at = None;
    if !hold.fired && !hold.moved {
        let remaining = hold.duration - (now - press_start);
        if remaining <= 0.0 {
            hold.fired = true;
            long_press_at = origin;
        } else {
            // A still finger sends no events; wake up when the delay ends.
            ctx.request_repaint_after(Duration::from_secs_f64(remaining));
        }
    }
    ctx.data_mut(|data| data.insert_temp(hold_id(), hold));
    TouchHold {
        long_press_at,
        suppress_tap: hold.fired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::touch;

    fn frame(ctx: &Context, time: f64, events: Vec<egui::Event>) -> [TouchHold; 2] {
        let mut holds = [TouchHold::default(); 2];
        let _ = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| holds = [touch_hold(ui.ctx()), touch_hold(ui.ctx())],
        );
        holds
    }

    #[test]
    fn a_still_touch_becomes_one_long_press_after_the_set_delay() {
        let ctx = Context::default();
        set_long_press_duration(&ctx, Duration::from_millis(300));
        let at = egui::pos2(40.0, 40.0);
        frame(&ctx, 0.0, touch(egui::TouchPhase::Start, at));
        let [early, _] = frame(&ctx, 0.2, Vec::new());
        assert_eq!(early, TouchHold::default());
        let [fired, asked_again] = frame(&ctx, 0.31, Vec::new());
        assert_eq!(fired.long_press_at, Some(at));
        assert_eq!(asked_again, fired, "every caller in a frame sees the press");
        let [later, _] = frame(&ctx, 0.5, Vec::new());
        assert_eq!(later.long_press_at, None);
        let [lifted, _] = frame(&ctx, 0.6, touch(egui::TouchPhase::End, at));
        assert!(lifted.suppress_tap, "lifting a long press is no tap");
    }

    #[test]
    fn a_touch_that_moves_is_no_long_press() {
        let ctx = Context::default();
        frame(
            &ctx,
            0.0,
            touch(egui::TouchPhase::Start, egui::pos2(40.0, 40.0)),
        );
        frame(
            &ctx,
            0.1,
            touch(egui::TouchPhase::Move, egui::pos2(80.0, 40.0)),
        );
        let [held, _] = frame(&ctx, 1.0, Vec::new());
        assert_eq!(held, TouchHold::default());
    }
}
