//! [`DialogWindow::show_with_preview`]: a dialog that pairs an image preview
//! with its controls, side by side on landscape screens and the preview above
//! the controls on portrait ones. The preview always stays in view; only the
//! controls scroll, and the action row stays below both. The window is a
//! [`dialog_window`](crate::dialog_window) like every other dialog, so it is
//! sized, placed and dismissed the same way.

use egui::{InnerResponse, Sense, Ui, Vec2};

use crate::dialogs::{footer_reserve, DialogWindow};
use crate::metrics::{SPACE_MD, SPACE_SM};
use crate::responsive::ScreenLayout;

/// The controls column of a landscape preview dialog.
const CONTROLS_WIDTH: std::ops::RangeInclusive<f32> = 260.0..=360.0;
/// The share of a portrait preview dialog's body the preview may take.
const PORTRAIT_PREVIEW_SHARE: f32 = 0.55;

/// The image a preview dialog shows.
#[derive(Clone, Copy, Debug)]
pub struct DialogPreview<'a> {
    /// The image's accessible name.
    pub label: &'a str,
    /// The image's height divided by its width.
    pub aspect: f32,
}

impl DialogWindow {
    /// Shows the dialog with `preview`, painted by `paint_preview` into the
    /// rect it fills, beside or above the scrolling `add_controls`, and
    /// `add_actions` below both. Open it with
    /// [`DIALOG_WIDTH_PREVIEW`](crate::DIALOG_WIDTH_PREVIEW) so a landscape
    /// screen has room for both columns.
    pub fn show_with_preview(
        self,
        ctx: &egui::Context,
        preview: DialogPreview<'_>,
        paint_preview: impl FnOnce(&mut Ui, egui::Rect),
        add_controls: impl FnOnce(&mut Ui),
        add_actions: impl FnOnce(&mut Ui),
    ) -> Option<InnerResponse<Option<()>>> {
        let landscape =
            ScreenLayout::from_size(ctx.content_rect().size()) == ScreenLayout::Horizontal;
        let Self {
            window,
            title,
            max_height,
            ..
        } = self;
        // The content's share of the window's height limit, inside the frame
        // egui draws around it.
        let content_height = {
            let style = ctx.global_style();
            max_height
                - style.spacing.window_margin.sum().y
                - 2.0 * style.visuals.window_stroke.width
        };
        window.show(ctx, |ui| {
            let title_top = ui.cursor().top();
            ui.heading(title);
            ui.add_space(SPACE_SM);
            let title_height = ui.cursor().top() - title_top;
            // The title, body and action row share the height a plain dialog's
            // content may take. The row is measured on the frame before
            // (estimated on the first), so the dialog never outgrows the
            // screen. A size read from `ui` here would follow the window's last
            // size instead.
            let actions_id = ui.id().with("preview-dialog-actions-height");
            let actions_height = ui
                .data(|data| data.get_temp::<f32>(actions_id))
                .unwrap_or_else(|| footer_reserve(ui.ctx()));
            let item_spacing = ui.spacing().item_spacing.y;
            // egui spaces the action row from the body by one item spacing.
            preview_body(
                ui,
                (content_height - title_height - item_spacing - actions_height).max(1.0),
                landscape,
                preview,
                paint_preview,
                add_controls,
            );
            let actions_top = ui.cursor().top();
            add_actions(ui);
            let measured = ui.min_rect().bottom() - actions_top;
            if (measured - actions_height).abs() > 0.5 {
                ui.data_mut(|data| data.insert_temp(actions_id, measured));
                ui.ctx().request_repaint();
            }
        })
    }
}

/// Lays out `preview` beside the scrolling controls on landscape screens and
/// above them otherwise, within `body_height`. Returns whether the controls
/// overflow and scroll.
pub(crate) fn preview_body(
    ui: &mut Ui,
    body_height: f32,
    landscape: bool,
    preview: DialogPreview<'_>,
    paint_preview: impl FnOnce(&mut Ui, egui::Rect),
    add_controls: impl FnOnce(&mut Ui),
) -> bool {
    let layout = PreviewLayout::new(
        ui.available_width(),
        body_height,
        ui.spacing().item_spacing.y,
        preview.aspect,
        landscape,
    );
    let show_preview = |ui: &mut Ui| {
        let (rect, response) = ui.allocate_exact_size(layout.preview, Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Image, true, preview.label)
        });
        paint_preview(ui, rect);
    };
    let show_controls = |ui: &mut Ui| {
        let output = egui::ScrollArea::vertical()
            .id_salt("preview-dialog-controls")
            .max_height(layout.controls_max_height)
            .auto_shrink([false, true])
            .show(ui, add_controls);
        output.content_size.y > output.inner_rect.height() + 0.5
    };
    if layout.landscape {
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(layout.preview_column_width, layout.preview.y),
                egui::Layout::top_down(egui::Align::Center),
                show_preview,
            );
            ui.add_space(SPACE_MD);
            ui.vertical(show_controls).inner
        })
        .inner
    } else {
        ui.vertical_centered(show_preview);
        ui.add_space(SPACE_SM);
        show_controls(ui)
    }
}

/// Sizes inside a preview dialog whose content is `width` wide and whose
/// body may be `body_height` tall, with `item_spacing` between stacked rows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewLayout {
    landscape: bool,
    /// The landscape preview column, which centres the preview.
    preview_column_width: f32,
    preview: Vec2,
    controls_max_height: f32,
}

impl PreviewLayout {
    fn new(width: f32, body_height: f32, item_spacing: f32, aspect: f32, landscape: bool) -> Self {
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        // Fits the preview into `width`×`height`.
        let fit = |width: f32, height: f32| {
            let height = (width * aspect).min(height).max(1.0);
            Vec2::new(height / aspect, height)
        };
        if landscape {
            let controls_width =
                (width * 0.36).clamp(*CONTROLS_WIDTH.start(), *CONTROLS_WIDTH.end());
            let preview_column_width = (width - controls_width - SPACE_MD).max(1.0);
            Self {
                landscape,
                preview_column_width,
                preview: fit(preview_column_width, body_height),
                controls_max_height: body_height,
            }
        } else {
            let preview = fit(width, body_height * PORTRAIT_PREVIEW_SHARE);
            Self {
                landscape,
                preview_column_width: width,
                preview,
                controls_max_height: (body_height - preview.y - SPACE_SM - item_spacing).max(1.0),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dialog_confirmation_buttons, dialog_window, Design, DialogKeyboard, Metrics,
        DIALOG_WIDTH_DEFAULT, DIALOG_WIDTH_PREVIEW,
    };
    use egui::{Pos2, Rect};

    /// The window, preview and action rects of a preview dialog with more
    /// controls than fit, and the window rect of a plain dialog, on a screen.
    struct Rendered {
        window: Rect,
        preview: Rect,
        actions: Rect,
        plain_window: Rect,
    }

    fn render(screen: Vec2, metrics: Metrics) -> Rendered {
        let ctx = egui::Context::default();
        Design::default().theme().apply_with_metrics(&ctx, metrics);
        let mut rendered = Rendered {
            window: Rect::NOTHING,
            preview: Rect::NOTHING,
            actions: Rect::NOTHING,
            plain_window: Rect::NOTHING,
        };
        // egui settles an anchored window's size and place over a few frames.
        for _ in 0..10 {
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
                    ..Default::default()
                },
                |ui| {
                    let ctx = ui.ctx();
                    let window = dialog_window("Preview", ctx, DIALOG_WIDTH_PREVIEW)
                        .show_with_preview(
                            ctx,
                            DialogPreview {
                                label: "Preview",
                                aspect: 2.0 / 3.0,
                            },
                            |_, rect| rendered.preview = rect,
                            |ui| {
                                for row in 0..40 {
                                    ui.label(format!("Control {row}"));
                                }
                            },
                            |ui| {
                                let top = ui.cursor().top();
                                dialog_confirmation_buttons(
                                    ui,
                                    "Cancel",
                                    "Apply",
                                    true,
                                    false,
                                    DialogKeyboard::CLOSE_ONLY,
                                );
                                rendered.actions = Rect::from_x_y_ranges(
                                    ui.min_rect().x_range(),
                                    top..=ui.cursor().top(),
                                );
                            },
                        );
                    rendered.window = window.map_or(Rect::NOTHING, |window| window.response.rect);
                    let plain = dialog_window("Plain", ctx, DIALOG_WIDTH_DEFAULT)
                        .show(ctx, |ui| ui.label("Body"));
                    rendered.plain_window =
                        plain.map_or(Rect::NOTHING, |plain| plain.response.rect);
                },
            );
        }
        rendered
    }

    /// Whether `window` keeps the padding every dialog keeps from the screen
    /// edges.
    fn within_dialog_bounds(screen: Vec2, window: Rect) -> bool {
        let padding = crate::dialogs::DIALOG_VIEWPORT_PADDING * 0.5;
        Rect::from_min_max(
            Pos2::new(padding, padding),
            (screen - Vec2::splat(padding)).to_pos2(),
        )
        .expand(0.5)
        .contains_rect(window)
    }

    #[test]
    fn a_phone_shows_it_like_any_other_dialog_with_the_preview_on_top() {
        let screen = Vec2::new(400.0, 800.0);
        let rendered = render(screen, Metrics::ANDROID);
        assert_eq!(rendered.window.x_range(), rendered.plain_window.x_range());
        assert!((rendered.window.center().y - screen.y * 0.5).abs() <= 1.0);
        assert!(within_dialog_bounds(screen, rendered.window));
        assert!(rendered.window.contains_rect(rendered.preview));
        assert!(rendered.actions.top() >= rendered.preview.bottom());
        assert!(rendered.window.contains_rect(rendered.actions));
    }

    #[test]
    fn a_landscape_screen_centres_it_with_the_preview_beside_the_controls() {
        let screen = Vec2::new(1280.0, 720.0);
        let rendered = render(screen, Metrics::DESKTOP);
        assert!((rendered.window.center().x - screen.x * 0.5).abs() <= 1.0);
        assert!((rendered.window.center().y - screen.y * 0.5).abs() <= 1.0);
        assert!(within_dialog_bounds(screen, rendered.window));
        assert!(rendered.window.contains_rect(rendered.preview));
        // The preview takes the left column, at a useful size.
        assert!(rendered.preview.center().x < rendered.window.center().x);
        assert!(rendered.preview.height() > 300.0);
        assert!(rendered.window.contains_rect(rendered.actions));
    }

    #[test]
    fn a_landscape_preview_fits_its_column_and_the_body_height() {
        let wide = PreviewLayout::new(900.0, 600.0, 8.0, 2.0 / 3.0, true);
        assert!(wide.preview.x <= wide.preview_column_width);
        assert!((wide.preview.y / wide.preview.x - 2.0 / 3.0).abs() < 1e-4);
        assert_eq!(wide.controls_max_height, 600.0);

        let tall = PreviewLayout::new(900.0, 300.0, 8.0, 1.5, true);
        assert!((tall.preview.y - 300.0).abs() < 1e-3);
        assert!(tall.preview.x < tall.preview_column_width);
    }

    #[test]
    fn a_portrait_preview_leaves_the_rest_to_the_controls() {
        let layout = PreviewLayout::new(360.0, 600.0, 8.0, 2.0 / 3.0, false);
        assert!(layout.preview.y <= 600.0 * PORTRAIT_PREVIEW_SHARE + 1e-3);
        assert!(
            (layout.preview.y + SPACE_SM + 8.0 + layout.controls_max_height - 600.0).abs() < 1e-3
        );
    }
}
