//! The on-screen keyboard. egui does not learn about soft keyboards that
//! overlay the window, so the platform layer reports the keyboard's height and
//! [`crate::Dialog`] stays above it. The platform layer also asks which layout
//! the keyboard should show.

use egui::{Context, Id, Rect};

fn keyboard_inset_id() -> Id {
    Id::new("moduwu-keyboard-inset")
}

/// Records that an on-screen keyboard covers the bottom `points` of the
/// viewport, or 0 while none is shown. The value holds until it is replaced.
pub fn set_keyboard_inset(ctx: &Context, points: f32) {
    let points = if points.is_finite() {
        points.max(0.0)
    } else {
        0.0
    };
    ctx.data_mut(|data| data.insert_temp(keyboard_inset_id(), points));
}

/// Height of the viewport covered by the on-screen keyboard, in points.
pub fn keyboard_inset(ctx: &Context) -> f32 {
    ctx.data(|data| data.get_temp::<f32>(keyboard_inset_id()))
        .unwrap_or(0.0)
}

fn numeric_keyboard_id() -> Id {
    Id::new("moduwu-numeric-keyboard")
}

/// Asks for the on-screen keyboard's number layout: a control that edits a
/// number is shown this frame.
pub(crate) fn request_numeric_keyboard(ctx: &Context) {
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| data.insert_temp(numeric_keyboard_id(), frame));
}

/// Whether a control that edits a number was shown in the current or the
/// previous frame, so the platform should give the on-screen keyboard its
/// number layout. egui has no input purpose for this, and the platform layer
/// usually asks before the frame's controls run.
pub fn numeric_keyboard_requested(ctx: &Context) -> bool {
    let frame = ctx.cumulative_frame_nr();
    ctx.data(|data| data.get_temp::<u64>(numeric_keyboard_id()))
        .is_some_and(|requested| requested + 1 >= frame)
}

/// [`Context::content_rect`] without the part the on-screen keyboard covers.
/// The keyboard inset is measured from the bottom of the viewport, which may
/// lie below the content rect's safe area.
pub fn unobscured_content_rect(ctx: &Context) -> Rect {
    let content = ctx.content_rect();
    let keyboard_top = ctx.viewport_rect().bottom() - keyboard_inset(ctx);
    let bottom = content.bottom().min(keyboard_top).max(content.top() + 1.0);
    Rect::from_min_max(content.min, egui::pos2(content.right(), bottom))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ctx: &Context, inset: Option<f32>) -> Rect {
        let mut rect = Rect::NOTHING;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(412.0, 860.0),
                )),
                ..Default::default()
            },
            |ui| {
                if let Some(inset) = inset {
                    set_keyboard_inset(ui.ctx(), inset);
                }
                rect = unobscured_content_rect(ui.ctx());
            },
        );
        rect
    }

    #[test]
    fn the_keyboard_inset_shortens_the_content_rect_until_it_is_cleared() {
        let ctx = Context::default();
        assert_eq!(run(&ctx, None).bottom(), 860.0);
        assert_eq!(run(&ctx, Some(320.0)).bottom(), 540.0);
        // The inset holds on frames that do not report it again.
        assert_eq!(run(&ctx, None).bottom(), 540.0);
        assert_eq!(run(&ctx, Some(0.0)).bottom(), 860.0);
    }

    #[test]
    fn a_numeric_keyboard_request_lasts_into_the_next_frame_only() {
        let ctx = Context::default();
        let frame = |request: bool| {
            let mut requested = false;
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                // The platform layer asks first, as an application frame does.
                requested = numeric_keyboard_requested(ui.ctx());
                if request {
                    request_numeric_keyboard(ui.ctx());
                }
            });
            requested
        };
        assert!(!frame(true));
        assert!(frame(false), "the previous frame's request still counts");
        assert!(!frame(false));
    }

    #[test]
    fn invalid_or_oversized_insets_keep_a_usable_rect() {
        let ctx = Context::default();
        assert_eq!(run(&ctx, Some(f32::NAN)).bottom(), 860.0);
        assert_eq!(run(&ctx, Some(-40.0)).bottom(), 860.0);
        let rect = run(&ctx, Some(2_000.0));
        assert!(rect.height() >= 1.0, "{rect:?}");
    }
}
