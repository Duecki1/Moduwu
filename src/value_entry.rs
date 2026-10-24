//! Typing an exact value on touch screens. The value readout of a
//! [`crate::Slider`] or [`crate::AngleDial`] cannot be edited in place there,
//! because the on-screen keyboard would cover the control, so a tap opens a
//! [`Dialog`] with a text field instead, and asks for the keyboard's number
//! layout while it is open.

use egui::emath::Numeric;
use egui::text::{CCursor, CCursorRange};
use egui::{Response, Ui};
use std::cell::Cell;
use std::ops::RangeInclusive;

use crate::dialogs::{
    dialog_confirmation_actions, dialog_text_field, request_initial_focus, DialogAction,
    DialogKeyboard,
};
use crate::keyboard::request_numeric_keyboard;
use crate::modal_dialog::Dialog;
use crate::slider::set_numeric;

/// Where a tap on a readout stands. A tap opens the dialog only once it can
/// no longer become a double tap, which resets the value instead.
#[derive(Clone, Debug)]
enum Stage {
    Tapped { at: f64 },
    Editing { text: String, focus_requested: bool },
}

#[derive(Clone, Debug)]
struct ValueEntry {
    stage: Stage,
    /// The frame that last showed the readout. An entry whose readout was not
    /// shown on the previous frame is abandoned, so a control that went away
    /// does not leave its dialog open or reopen it later.
    frame: u64,
}

/// Handles taps on the touch value `readout` of the control `label`: opens
/// the entry dialog and applies a confirmed value, rounded to `decimals`.
/// Returns whether the value changed.
pub(crate) fn touch_value_entry<Num: Numeric + Copy>(
    ui: &Ui,
    readout: &Response,
    label: &str,
    value: &mut Num,
    range: RangeInclusive<Num>,
    decimals: usize,
) -> bool {
    let ctx = ui.ctx();
    let id = readout.id.with("value-entry");
    let frame = ctx.cumulative_frame_nr();
    let now = ui.input(|input| input.time);
    let double_click_delay = ctx.options(|options| options.input_options.max_double_click_delay);
    let mut entry = ctx
        .data(|data| data.get_temp::<ValueEntry>(id))
        .filter(|entry| entry.frame + 1 >= frame)
        .map(|entry| entry.stage);

    if readout.double_clicked() {
        entry = None;
    } else if readout.clicked() && entry.is_none() {
        entry = Some(Stage::Tapped { at: now });
    }
    if let Some(Stage::Tapped { at }) = entry {
        let waited = now - at;
        if waited >= double_click_delay {
            entry = Some(Stage::Editing {
                text: format!("{:.decimals$}", value.to_f64()),
                focus_requested: false,
            });
        } else {
            ctx.request_repaint_after_secs((double_click_delay - waited) as f32);
        }
    }

    let mut changed = false;
    if let Some(Stage::Editing {
        text,
        focus_requested,
    }) = &mut entry
    {
        request_numeric_keyboard(ctx);
        let (min, max) = {
            let (start, end) = (range.start().to_f64(), range.end().to_f64());
            (start.min(end), start.max(end))
        };
        match entry_dialog(ctx, id, label, text, focus_requested, (min, max), decimals) {
            EntryOutcome::Open => {}
            EntryOutcome::Cancelled => entry = None,
            EntryOutcome::Confirmed(typed) => {
                changed = set_numeric(value, typed, decimals);
                entry = None;
            }
        }
    }

    ctx.data_mut(|data| match entry {
        Some(stage) => {
            data.insert_temp(id, ValueEntry { stage, frame });
        }
        None => data.remove::<ValueEntry>(id),
    });
    changed
}

enum EntryOutcome {
    Open,
    Cancelled,
    Confirmed(f64),
}

/// Shows the entry dialog for values within `(min, max)`.
fn entry_dialog(
    ctx: &egui::Context,
    id: egui::Id,
    label: &str,
    text: &mut String,
    focus_requested: &mut bool,
    (min, max): (f64, f64),
    decimals: usize,
) -> EntryOutcome {
    // The body parses the text before the footer decides whether the value
    // can be set in the same frame.
    let typed = Cell::new(None);
    let mut action = DialogAction::None;
    let hint = format!("{min:.decimals$} to {max:.decimals$}");
    let response = Dialog::new(id, label).show_with_footer(
        ctx,
        |ui| {
            let field = dialog_text_field(ui, text, "value", &hint);
            if !*focus_requested {
                // Select the shown value so that typing replaces it.
                let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
                state.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(0),
                    CCursor::new(text.chars().count()),
                )));
                state.store(ui.ctx(), field.id);
            }
            request_initial_focus(&field, focus_requested);
            typed.set(parse_entry(text, min, max, decimals));
        },
        |ui| {
            action = dialog_confirmation_actions(
                ui,
                "Cancel",
                "Set",
                typed.get().is_some(),
                false,
                DialogKeyboard::CONFIRM_ON_ENTER,
            );
        },
    );
    match (action, typed.get()) {
        (DialogAction::Confirm, Some(typed)) => EntryOutcome::Confirmed(typed),
        (DialogAction::Cancel, _) => EntryOutcome::Cancelled,
        _ if response.dismissed => EntryOutcome::Cancelled,
        _ => EntryOutcome::Open,
    }
}

/// The typed value rounded to `decimals`, when it is a number within
/// `min..=max`. Accepts surrounding spaces, a leading `+`, the Unicode minus
/// sign and a decimal comma, as phone keyboards offer them.
fn parse_entry(text: &str, min: f64, max: f64, decimals: usize) -> Option<f64> {
    let normalized = text
        .trim()
        .trim_start_matches('+')
        .replace('\u{2212}', "-")
        .replace(',', ".");
    let typed = normalized
        .parse::<f64>()
        .ok()
        .filter(|typed| typed.is_finite())?;
    let scale = 10_f64.powi(decimals.min(12) as i32);
    let rounded = (typed * scale).round() / scale;
    (min..=max).contains(&rounded).then_some(rounded)
}

#[cfg(test)]
mod tests {
    use super::parse_entry;

    #[test]
    fn typed_values_accept_phone_keyboard_notation() {
        for (text, expected) in [
            ("0.08", Some(0.08)),
            (" +0,25 ", Some(0.25)),
            ("\u{2212}1.5", Some(-1.5)),
            ("-5", Some(-5.0)),
            ("5.004", Some(5.0)),
            ("1.239", Some(1.24)),
        ] {
            assert_eq!(parse_entry(text, -5.0, 5.0, 2), expected, "{text:?}");
        }
    }

    #[test]
    fn typed_values_outside_the_range_or_not_numbers_are_rejected() {
        for text in ["", "abc", "5.01", "-6", "inf", "NaN", "1e400"] {
            assert_eq!(parse_entry(text, -5.0, 5.0, 2), None, "{text:?}");
        }
    }
}
