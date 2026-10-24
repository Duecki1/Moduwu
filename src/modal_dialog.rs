//! A modal dialog: a dimmed backdrop, a header with the title and an optional
//! close button, a scrolling body and a fixed footer for the actions.

use egui::{self, Align, Align2, Color32, CornerRadius, Layout, Response, RichText, Sense, Ui};

use crate::buttons::ghost_icon_button;
use crate::dialogs::{close_popups_on_escape, DIALOG_VIEWPORT_PADDING, DIALOG_WIDTH_PREVIEW};
use crate::keyboard::unobscured_content_rect;
use crate::metrics::{Metrics, SPACE_SM};
use crate::preview_dialog::{preview_body, DialogPreview};
use crate::responsive::ScreenLayout;

/// The width of a centred [`Dialog`]. Bottom sheets always span the viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogSize {
    /// Confirmations, short messages and single-field forms.
    #[default]
    Compact,
    /// Forms with several rows, lists and longer explanations.
    Wide,
    /// An image preview beside its controls, for [`Dialog::show_with_preview`].
    Preview,
}

impl DialogSize {
    pub const fn width(self) -> f32 {
        match self {
            Self::Compact => 400.0,
            Self::Wide => 560.0,
            Self::Preview => DIALOG_WIDTH_PREVIEW,
        }
    }
}

/// Touch layouts narrower than this show dialogs as bottom sheets.
const SHEET_BREAKPOINT: f32 = 600.0;
/// A bottom sheet leaves the top of the viewport visible.
const SHEET_MAX_HEIGHT_FRACTION: f32 = 0.9;
const SHEET_CORNER_RADIUS: u8 = 14;

/// Where a dialog sits in the viewport and how large it may grow.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Placement {
    sheet: bool,
    width: f32,
    max_height: f32,
}

impl Placement {
    fn for_viewport(viewport: egui::Rect, metrics: Metrics, size: DialogSize) -> Self {
        let fits = viewport.width() >= size.width() + DIALOG_VIEWPORT_PADDING;
        if fits && !(metrics.touch_layout && viewport.width() < SHEET_BREAKPOINT) {
            Self {
                sheet: false,
                width: size.width(),
                max_height: (viewport.height() - DIALOG_VIEWPORT_PADDING).max(1.0),
            }
        } else {
            Self {
                sheet: true,
                width: viewport.width().max(1.0),
                max_height: (viewport.height() * SHEET_MAX_HEIGHT_FRACTION).max(1.0),
            }
        }
    }
}

/// A modal dialog. The backdrop dims and blocks the rest of the UI; on narrow
/// viewports and touch layouts in portrait the dialog becomes a bottom sheet
/// whose footer actions span its width. The dialog sits above the on-screen
/// keyboard reported with [`crate::set_keyboard_inset`].
///
/// The dialog is shown for as long as the caller calls [`Dialog::show`] or
/// [`Dialog::show_with_footer`] each frame. Keyboard handling stays with the
/// caller: run [`crate::dialog_keyboard_action`], or use
/// [`crate::dialog_confirmation_actions`] in the footer.
///
/// Footer actions are added primary first: a centred dialog lays them out
/// right to left, so the primary action is rightmost, and a bottom sheet
/// stacks them top to bottom.
pub struct Dialog {
    id: egui::Id,
    title: String,
    size: DialogSize,
    close_button: bool,
    dismiss_on_backdrop: bool,
}

/// What a [`Dialog`] returns each frame.
pub struct DialogResponse<R> {
    pub inner: R,
    /// The close button was used, or the backdrop was clicked while
    /// [`Dialog::dismiss_on_backdrop`] allows that. The caller closes the
    /// dialog; Escape is reported by the caller's keyboard handling instead.
    pub dismissed: bool,
    /// The dialog's frame.
    pub response: Response,
}

impl Dialog {
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id: Self::area_id(id_salt),
            title: title.into(),
            size: DialogSize::default(),
            close_button: false,
            dismiss_on_backdrop: false,
        }
    }

    /// The id of the area a dialog created with `id_salt` is shown in, for
    /// looking it up in egui's memory.
    pub fn area_id(id_salt: impl egui::AsIdSalt) -> egui::Id {
        egui::Id::new("moduwu-dialog").with(id_salt)
    }

    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Shows a close button at the end of the header. Off by default.
    pub fn close_button(mut self, show: bool) -> Self {
        self.close_button = show;
        self
    }

    /// Reports a click on the backdrop as a dismissal. Off by default, so a
    /// stray click cannot discard a form. A click that only closes an open
    /// popup never dismisses the dialog.
    pub fn dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    pub fn show<R>(
        self,
        ctx: &egui::Context,
        add_body: impl FnOnce(&mut Ui) -> R,
    ) -> DialogResponse<R> {
        self.show_impl(
            ctx,
            |ui, max_height, _| scrolling_body(ui, max_height, add_body),
            None::<fn(&mut Ui)>,
        )
    }

    pub fn show_with_footer<R>(
        self,
        ctx: &egui::Context,
        add_body: impl FnOnce(&mut Ui) -> R,
        add_footer: impl FnOnce(&mut Ui),
    ) -> DialogResponse<R> {
        self.show_impl(
            ctx,
            |ui, max_height, _| scrolling_body(ui, max_height, add_body),
            Some(add_footer),
        )
    }

    /// Shows `preview`, painted by `paint_preview` into the rect it fills,
    /// beside the scrolling `add_controls` on landscape screens and above them
    /// otherwise; only the controls scroll. Use [`DialogSize::Preview`] so a
    /// landscape screen has room for both columns.
    pub fn show_with_preview(
        self,
        ctx: &egui::Context,
        preview: DialogPreview<'_>,
        paint_preview: impl FnOnce(&mut Ui, egui::Rect),
        add_controls: impl FnOnce(&mut Ui),
        add_footer: impl FnOnce(&mut Ui),
    ) -> DialogResponse<()> {
        let landscape =
            ScreenLayout::from_size(ctx.content_rect().size()) == ScreenLayout::Horizontal;
        self.show_impl(
            ctx,
            |ui, max_height, sheet| {
                let scrolls = preview_body(
                    ui,
                    max_height,
                    landscape && !sheet,
                    preview,
                    paint_preview,
                    add_controls,
                );
                ((), scrolls)
            },
            Some(add_footer),
        )
    }

    /// `add_body` lays out the body within the height it is given, knowing
    /// whether the dialog is a bottom sheet, and returns its result and
    /// whether its content scrolls.
    fn show_impl<R>(
        self,
        ctx: &egui::Context,
        add_body: impl FnOnce(&mut Ui, f32, bool) -> (R, bool),
        add_footer: Option<impl FnOnce(&mut Ui)>,
    ) -> DialogResponse<R> {
        let Self {
            id,
            title,
            size,
            close_button,
            dismiss_on_backdrop,
        } = self;
        close_popups_on_escape(ctx);
        let metrics = Metrics::of(ctx);
        // Stay above an on-screen keyboard so the focused field and the footer
        // actions remain visible while typing.
        let viewport = unobscured_content_rect(ctx);
        let placement = Placement::for_viewport(viewport, metrics, size);
        let style = ctx.global_style();
        let mut frame = egui::Frame::window(&style);
        let anchor = if placement.sheet {
            let radius = SHEET_CORNER_RADIUS;
            frame = frame.corner_radius(CornerRadius {
                nw: radius,
                ne: radius,
                sw: 0,
                se: 0,
            });
            Align2::CENTER_BOTTOM
        } else {
            Align2::CENTER_CENTER
        };
        let backdrop = Color32::from_black_alpha(if style.visuals.dark_mode { 140 } else { 90 });
        let chrome = frame.total_margin().sum();
        let inner_width = (placement.width - chrome.x).max(1.0);
        let item_spacing = style.spacing.item_spacing.y;
        let header_height = metrics.control_height;
        let footer_height_id = id.with("footer-height");

        let modal = egui::Modal::new(id)
            .area(
                egui::Modal::default_area(id)
                    .anchor(anchor, egui::Vec2::ZERO)
                    .constrain_to(viewport),
            )
            .frame(frame)
            .backdrop_color(backdrop)
            .show(ctx, |ui| {
                ui.set_width(inner_width);
                let mut closed = false;
                ui.allocate_ui_with_layout(
                    egui::vec2(inner_width, header_height),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        if close_button {
                            closed = ghost_icon_button(
                                ui,
                                "×",
                                egui::Vec2::splat(header_height),
                                "Close",
                            )
                            .clicked();
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(title)
                                        .strong()
                                        .size(metrics.panel_title_text_size),
                                )
                                .truncate(),
                            );
                        });
                    },
                );
                ui.add_space(SPACE_SM);

                // The footer's height from the previous pass, so the body can
                // leave room for it before the footer is laid out.
                let footer_reserve = if add_footer.is_some() {
                    let footer = ui
                        .data(|data| data.get_temp::<f32>(footer_height_id))
                        .unwrap_or(metrics.control_height);
                    footer + SPACE_SM + item_spacing * 2.0
                } else {
                    0.0
                };
                let body_max_height = (placement.max_height
                    - chrome.y
                    - header_height
                    - SPACE_SM
                    - item_spacing * 2.0
                    - footer_reserve)
                    .max(metrics.control_height);
                let (body, body_scrolls) = add_body(ui, body_max_height, placement.sheet);

                if let Some(add_footer) = add_footer {
                    let (gap, _) =
                        ui.allocate_exact_size(egui::vec2(inner_width, SPACE_SM), Sense::hover());
                    if body_scrolls {
                        ui.painter().hline(
                            gap.x_range(),
                            gap.center().y,
                            ui.visuals().widgets.noninteractive.bg_stroke,
                        );
                    }
                    let footer = if placement.sheet {
                        ui.with_layout(Layout::top_down_justified(Align::Center), add_footer)
                    } else {
                        ui.allocate_ui_with_layout(
                            egui::vec2(inner_width, metrics.control_height),
                            Layout::right_to_left(Align::Center).with_main_wrap(true),
                            add_footer,
                        )
                    };
                    let height = footer.response.rect.height();
                    let previous = ui.data_mut(|data| {
                        let previous = data.get_temp::<f32>(footer_height_id);
                        data.insert_temp(footer_height_id, height);
                        previous
                    });
                    if previous.is_none_or(|previous| (previous - height).abs() > 0.5) {
                        ui.ctx().request_discard("dialog footer height changed");
                    }
                }
                (body, closed)
            });

        let (inner, closed) = modal.inner;
        let backdrop_dismissed =
            dismiss_on_backdrop && !modal.any_popup_open && modal.backdrop_response.clicked();
        DialogResponse {
            inner,
            dismissed: closed || backdrop_dismissed,
            response: modal.response,
        }
    }
}

/// A body that grows with its content up to `max_height`, then scrolls.
fn scrolling_body<R>(
    ui: &mut Ui,
    max_height: f32,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let body = egui::ScrollArea::vertical()
        .id_salt("dialog-body")
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, add_body);
    let scrolls = body.content_size.y > body.inner_rect.height() + 0.5;
    (body.inner, scrolls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buttons::{primary_action_button, secondary_button};
    use crate::dialogs::{dialog_confirmation_actions, DialogAction, DialogKeyboard};
    use crate::preview_dialog::DialogPreview;
    use crate::test_support::key_press;

    struct Frame {
        dialog: egui::Rect,
        footer: Option<egui::Rect>,
        dismissed: bool,
        action: DialogAction,
    }

    fn render(
        ctx: &egui::Context,
        viewport: egui::Vec2,
        events: Vec<egui::Event>,
        dialog: impl Fn() -> Dialog,
        body_rows: usize,
    ) -> Frame {
        let mut frame = None;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, viewport)),
                events,
                ..Default::default()
            },
            |root| {
                let mut footer = None;
                let mut action = DialogAction::None;
                let response = dialog().show_with_footer(
                    root.ctx(),
                    |ui| {
                        for _ in 0..body_rows {
                            ui.label("A body line long enough to wrap in a narrow dialog.");
                        }
                    },
                    |ui| {
                        let start = ui.min_rect();
                        action = dialog_confirmation_actions(
                            ui,
                            "Cancel",
                            "Save",
                            true,
                            false,
                            DialogKeyboard::CONFIRM_ON_ENTER,
                        );
                        footer = Some(ui.min_rect().union(start));
                    },
                );
                frame = Some(Frame {
                    dialog: response.response.rect,
                    footer,
                    dismissed: response.dismissed,
                    action,
                });
            },
        );
        frame.expect("the dialog renders every frame")
    }

    fn settle(
        ctx: &egui::Context,
        viewport: egui::Vec2,
        dialog: impl Fn() -> Dialog,
        rows: usize,
    ) -> Frame {
        for _ in 0..3 {
            render(ctx, viewport, Vec::new(), &dialog, rows);
        }
        render(ctx, viewport, Vec::new(), &dialog, rows)
    }

    fn desktop() -> egui::Context {
        let ctx = egui::Context::default();
        Metrics::DESKTOP.install(&ctx);
        ctx
    }

    #[test]
    fn short_dialogs_are_centred_at_their_size_and_shrink_to_content() {
        let ctx = desktop();
        let viewport = egui::vec2(900.0, 700.0);
        let frame = settle(&ctx, viewport, || Dialog::new("short", "Rename"), 1);
        assert_eq!(frame.dialog.width(), DialogSize::Compact.width());
        assert!(frame.dialog.height() < 200.0, "{:?}", frame.dialog);
        let centre = frame.dialog.center();
        assert!((centre.x - 450.0).abs() < 1.0 && (centre.y - 350.0).abs() < 1.0);

        let wide = settle(
            &ctx,
            viewport,
            || Dialog::new("wide", "Export").size(DialogSize::Wide),
            1,
        );
        assert_eq!(wide.dialog.width(), DialogSize::Wide.width());
    }

    #[test]
    fn narrow_and_touch_portrait_viewports_show_a_bottom_sheet() {
        for (metrics, viewport) in [
            (Metrics::ANDROID, egui::vec2(412.0, 860.0)),
            (Metrics::DESKTOP, egui::vec2(380.0, 600.0)),
        ] {
            let ctx = egui::Context::default();
            metrics.install(&ctx);
            let frame = settle(&ctx, viewport, || Dialog::new("sheet", "Delete preset?"), 1);
            assert!(
                (frame.dialog.width() - viewport.x).abs() < 1.0,
                "{frame:?}",
                frame = frame.dialog
            );
            assert!(
                (frame.dialog.bottom() - viewport.y).abs() < 1.0,
                "{:?}",
                frame.dialog
            );
            let footer = frame.footer.expect("footer renders");
            assert!(
                footer.width() > viewport.x * 0.8,
                "stacked actions span the sheet: {footer:?}"
            );
        }
    }

    #[test]
    fn dialogs_stay_above_the_on_screen_keyboard() {
        for (viewport, keyboard, rows) in [
            // A portrait bottom sheet rises with the keyboard.
            (egui::vec2(412.0, 860.0), 320.0, 1),
            // A centred landscape dialog shrinks and scrolls into what is left.
            (egui::vec2(900.0, 412.0), 220.0, 40),
        ] {
            let ctx = egui::Context::default();
            Metrics::ANDROID.install(&ctx);
            crate::set_keyboard_inset(&ctx, keyboard);
            let frame = settle(&ctx, viewport, || Dialog::new("keyboard", "Rename"), rows);
            let visible_bottom = viewport.y - keyboard;
            assert!(
                frame.dialog.bottom() <= visible_bottom + 0.5 && frame.dialog.top() >= -0.5,
                "{viewport:?}: {:?}",
                frame.dialog
            );
            let footer = frame.footer.expect("footer renders");
            assert!(
                footer.bottom() <= visible_bottom,
                "{viewport:?}: {footer:?}"
            );
            if rows == 1 {
                assert!(
                    (frame.dialog.bottom() - visible_bottom).abs() < 1.0,
                    "the sheet sits on the keyboard: {:?}",
                    frame.dialog
                );
            }
        }
    }

    #[test]
    fn touch_landscape_viewports_keep_a_centred_dialog() {
        let ctx = egui::Context::default();
        Metrics::ANDROID.install(&ctx);
        let frame = settle(
            &ctx,
            egui::vec2(900.0, 412.0),
            || Dialog::new("land", "Rename"),
            1,
        );
        assert_eq!(frame.dialog.width(), DialogSize::Compact.width());
    }

    #[test]
    fn footer_stays_in_view_when_the_body_scrolls() {
        for viewport in [
            egui::vec2(720.0, 700.0),
            egui::vec2(720.0, 260.0),
            egui::vec2(180.0, 260.0),
        ] {
            let ctx = desktop();
            let frame = settle(
                &ctx,
                viewport,
                || Dialog::new("long", "Download model?"),
                40,
            );
            let footer = frame.footer.expect("footer renders");
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, viewport);
            assert!(
                frame.dialog.contains_rect(footer) && screen.contains_rect(footer),
                "viewport {viewport:?}, dialog {:?}, footer {footer:?}",
                frame.dialog
            );
        }
    }

    #[test]
    fn close_button_dismisses_from_the_keyboard() {
        let ctx = desktop();
        let viewport = egui::vec2(900.0, 700.0);
        let dialog = || Dialog::new("close", "Shortcuts").close_button(true);
        settle(&ctx, viewport, dialog, 1);
        // The close button is the first focusable control.
        render(&ctx, viewport, vec![key_press(egui::Key::Tab)], dialog, 1);
        let frame = render(&ctx, viewport, vec![key_press(egui::Key::Space)], dialog, 1);
        assert!(frame.dismissed);
    }

    #[test]
    fn backdrop_clicks_dismiss_only_when_allowed() {
        for allowed in [false, true] {
            let ctx = desktop();
            let viewport = egui::vec2(900.0, 700.0);
            let dialog = || Dialog::new("backdrop", "Paste").dismiss_on_backdrop(allowed);
            settle(&ctx, viewport, dialog, 1);
            let pos = egui::pos2(10.0, 10.0);
            let button = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            render(
                &ctx,
                viewport,
                vec![egui::Event::PointerMoved(pos), button(true)],
                dialog,
                1,
            );
            let frame = render(&ctx, viewport, vec![button(false)], dialog, 1);
            assert_eq!(frame.dismissed, allowed);
        }
    }

    #[test]
    fn the_rest_of_the_ui_cannot_be_used_while_a_dialog_is_open() {
        let ctx = desktop();
        settle(
            &ctx,
            egui::vec2(900.0, 700.0),
            || Dialog::new("modal", "Rename"),
            1,
        );
        assert!(!ctx.memory(|memory| memory.allows_interaction(egui::LayerId::background())));
    }

    #[test]
    fn footer_keyboard_actions_confirm_and_cancel() {
        let viewport = egui::vec2(900.0, 700.0);
        for (key, expected) in [
            (egui::Key::Enter, DialogAction::Confirm),
            (egui::Key::Escape, DialogAction::Cancel),
        ] {
            let ctx = desktop();
            let dialog = || Dialog::new("keys", "Rename");
            settle(&ctx, viewport, dialog, 1);
            let frame = render(&ctx, viewport, vec![key_press(key)], dialog, 1);
            assert_eq!(frame.action, expected, "{key:?}");
        }
    }

    #[test]
    fn area_id_finds_the_shown_dialog() {
        let ctx = desktop();
        let frame = settle(
            &ctx,
            egui::vec2(900.0, 700.0),
            || Dialog::new("lookup", "Rename"),
            1,
        );
        let area = ctx
            .memory(|memory| memory.area_rect(Dialog::area_id("lookup")))
            .expect("the dialog's area is known");
        assert_eq!(area, frame.dialog);
    }

    #[test]
    fn preview_dialogs_keep_the_preview_and_footer_in_view() {
        for (metrics, viewport, side_by_side) in [
            (Metrics::DESKTOP, egui::vec2(1280.0, 800.0), true),
            (Metrics::DESKTOP, egui::vec2(1280.0, 360.0), true),
            (Metrics::ANDROID, egui::vec2(412.0, 860.0), false),
        ] {
            let ctx = egui::Context::default();
            metrics.install(&ctx);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, viewport);
            let mut rects = None;
            // A tall scrolling column grows into its final height over a few
            // frames.
            for _ in 0..12 {
                let _ = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |root| {
                        let mut preview = egui::Rect::NOTHING;
                        let mut controls = egui::Rect::NOTHING;
                        let mut footer = egui::Rect::NOTHING;
                        let response = Dialog::new("preview", "Lens grid")
                            .size(DialogSize::Preview)
                            .show_with_preview(
                                root.ctx(),
                                DialogPreview {
                                    label: "Grid",
                                    aspect: 2.0 / 3.0,
                                },
                                |_, rect| preview = rect,
                                |ui| {
                                    controls = ui.label("First control").rect;
                                    for row in 0..40 {
                                        ui.label(format!("Control {row}"));
                                    }
                                },
                                |ui| footer = secondary_button(ui, "Done").rect,
                            );
                        rects = Some((response.response.rect, preview, controls, footer));
                    },
                );
            }
            let (dialog, preview, controls, footer) = rects.expect("dialog renders");
            assert!(preview.height() > 40.0, "{viewport:?}: preview {preview:?}");
            for rect in [preview, footer] {
                assert!(
                    dialog.contains_rect(rect) && screen.contains_rect(rect),
                    "{viewport:?}: {rect:?} outside dialog {dialog:?}"
                );
            }
            if side_by_side {
                assert!(preview.right() < controls.left(), "{viewport:?}");
            } else {
                assert!(preview.bottom() < controls.top(), "{viewport:?}");
            }
        }
    }

    #[test]
    fn centred_footers_put_the_primary_action_last() {
        let ctx = desktop();
        let mut rects = None;
        for _ in 0..3 {
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    ..Default::default()
                },
                |root| {
                    Dialog::new("order", "Paste adjustments").show_with_footer(
                        root.ctx(),
                        |ui| ui.label("Two images already contain edits."),
                        |ui| {
                            let primary = primary_action_button(ui, "Replace").rect;
                            let cancel = secondary_button(ui, "Cancel").rect;
                            rects = Some((primary, cancel));
                        },
                    );
                },
            );
        }
        let (primary, cancel) = rects.expect("footer renders");
        assert!(cancel.right() < primary.left());
    }
}
