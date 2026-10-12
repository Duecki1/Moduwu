//! Tooltips that wait for the pointer to rest.

use egui::{Response, WidgetText};

/// Shows `text` once the pointer has rested on `response` for the style's
/// `tooltip_delay`. Unlike [`Response::on_hover_text`], a tooltip shown
/// moments earlier does not skip the wait, so sweeping across a column of
/// controls does not flash a tooltip for each. Once open, the tooltip stays
/// while the pointer remains on the widget.
pub fn still_hover_text(response: Response, text: impl Into<WidgetText>) -> Response {
    if response.hovered() && !response.is_tooltip_open() {
        let delay = response.ctx.global_style().interaction.tooltip_delay;
        let still_for = response
            .ctx
            .input(|input| input.pointer.time_since_last_movement());
        if still_for < delay {
            response.ctx.request_repaint_after_secs(delay - still_for);
            return response;
        }
    }
    response.on_hover_text(text)
}
