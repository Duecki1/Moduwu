//! Rows of a collapsible tree, such as a folder hierarchy.
//!
//! A [`tree`] stacks [`TreeRow`]s closely so they read as one list. Each row
//! is indented by its depth: a disclosure arrow when it has children, an
//! optional icon, then its label. A child's arrow sits under its parent's
//! icon, and guide lines hang from each ancestor's arrow. Hover and selection
//! fill the whole row, so selected rows share one left edge at every depth.
//!
//! The caller owns the hierarchy: it shows the children of expanded rows one
//! level deeper and flips a row's expanded state when
//! [`TreeRowResponse::toggled`] reports it.

use egui::{
    self, InnerResponse, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui,
    WidgetText,
};

use crate::metrics::{control_height, CARD_RADIUS, SPACE_XS, SPACE_XXS};

/// Gap between the rows of a [`tree`].
const ROW_GAP: f32 = SPACE_XXS;

/// Gap between the row's edge and its disclosure, icon and label columns.
const COLUMN_GAP: f32 = SPACE_XS;

/// Opacity of the guide lines. They take the text colour, so they stay as
/// faint on hovered and selected rows as on the card behind the tree.
const GUIDE_OPACITY: f32 = 0.15;

/// Lays out `add_rows`, a sequence of [`TreeRow`]s, as one tree: one below
/// another, closer together than other controls.
pub fn tree<R>(ui: &mut Ui, add_rows: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        add_rows(ui)
    })
}

/// One row of a [`tree`], as wide as the space available and a control high.
///
/// Clicking the row, or Space and Enter while it has focus, clicks
/// [`TreeRowResponse::row`]; the arrow does the same for
/// [`TreeRowResponse::disclosure`]. Assistive technology sees the row as a
/// selectable label and the arrow as a collapsing header, toggled while the
/// row is expanded.
#[must_use = "call `show` to render the row"]
pub struct TreeRow<'a> {
    id_salt: egui::IdSalt,
    label: WidgetText,
    depth: usize,
    expanded: Option<bool>,
    icon: Option<&'a str>,
    selected: bool,
    sense: Sense,
}

impl<'a> TreeRow<'a> {
    /// `id_salt` identifies the row among the rows of its tree, which may
    /// show one label at several depths.
    pub fn new(id_salt: impl egui::AsIdSalt, label: impl Into<WidgetText>) -> Self {
        Self {
            id_salt: egui::IdSalt::new(id_salt),
            label: label.into(),
            depth: 0,
            expanded: None,
            icon: None,
            selected: false,
            sense: Sense::click(),
        }
    }

    /// Levels above the row: 0 for the tree's top rows, 1 for their
    /// children.
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// `Some(expanded)` gives the row a disclosure arrow, pointing down while
    /// `expanded`. `None`, the default, is a row without children, which keeps
    /// the arrow's space so its icon lines up with its siblings'.
    pub fn expanded(mut self, expanded: Option<bool>) -> Self {
        self.expanded = expanded;
        self
    }

    /// A glyph drawn before the label in the label's font, such as a folder.
    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// What the row responds to, `Sense::click()` by default; for example
    /// `Sense::click_and_drag()` for rows that can be dragged.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    pub fn show(self, ui: &mut Ui) -> TreeRowResponse {
        let height = control_height(ui);
        let column = column_width(height);
        let indent = column + COLUMN_GAP;
        let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width().max(1.0), height));
        let id = ui.make_persistent_id(self.id_salt);
        let enabled = ui.is_enabled();
        let text = self.label.text().to_owned();
        let selected = self.selected;

        let row = ui.interact(rect, id, self.sense);
        row.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, enabled, selected, &text)
        });

        // The arrow is a column wide plus a gap on either side, at the left
        // of the row's indented content.
        let content_left = rect.left() + self.depth as f32 * indent;
        let disclosure_rect = Rect::from_x_y_ranges(
            content_left..=content_left + column + 2.0 * COLUMN_GAP,
            rect.y_range(),
        );
        // Registered after the row, so the arrow is on top and takes its
        // clicks. It senses what the row senses: otherwise a row that can be
        // dragged would still be hovered, and show its tooltip, through it.
        let disclosure = self.expanded.map(|expanded| {
            let response = ui.interact(
                disclosure_rect,
                id.with("disclosure"),
                self.sense | Sense::click(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::CollapsingHeader,
                    enabled,
                    expanded,
                    &text,
                )
            });
            response
        });

        if row.hovered() || disclosure.as_ref().is_some_and(Response::hovered) {
            if let Some(cursor) = ui.visuals().interact_cursor {
                ui.ctx().set_cursor_icon(cursor);
            }
        }
        if !ui.is_rect_visible(rect) {
            return TreeRowResponse { row, disclosure };
        }

        let visuals = ui.visuals();
        let painter = ui.painter();
        let disclosure_highlighted = disclosure
            .as_ref()
            .is_some_and(|disclosure| disclosure.hovered() || disclosure.has_focus());
        let highlighted = row.hovered()
            || row.is_pointer_button_down_on()
            || row.has_focus()
            || disclosure_highlighted;
        // A disabled selected row keeps its fill, faded with the rest of the
        // disabled UI, as a selected button does.
        let (fill, text_color) = if selected {
            (
                Some(visuals.selection.bg_fill),
                visuals.selection.stroke.color,
            )
        } else if enabled && highlighted {
            (
                Some(visuals.widgets.hovered.weak_bg_fill),
                visuals.widgets.hovered.text_color(),
            )
        } else {
            (None, visuals.widgets.inactive.text_color())
        };
        let glyph_color = if selected {
            text_color
        } else {
            visuals.weak_text_color()
        };
        if let Some(fill) = fill {
            painter.rect_filled(rect, CARD_RADIUS, fill);
        }

        let gap = ui.spacing().item_spacing.y;
        let guide = Stroke::new(1.0, visuals.text_color().gamma_multiply(GUIDE_OPACITY));
        for level in 0..self.depth {
            let x = rect.left() + level as f32 * indent + COLUMN_GAP + column * 0.5;
            // Half the gap above and below meets the next row's guide.
            painter.vline(
                painter.round_to_pixel_center(x),
                rect.top() - gap * 0.5..=rect.bottom() + gap * 0.5,
                guide,
            );
        }

        let disclosure_center =
            egui::pos2(content_left + COLUMN_GAP + column * 0.5, rect.center().y);
        if let (Some(expanded), Some(response)) = (self.expanded, &disclosure) {
            let color = if disclosure_highlighted && !selected {
                text_color
            } else {
                glyph_color
            };
            paint_arrow(
                painter,
                disclosure_center,
                column,
                expanded,
                Stroke::new(1.5, color),
            );
            if response.has_focus() {
                paint_focus_ring(ui, disclosure_rect);
            }
        }

        let mut label_left = disclosure_rect.right();
        if let Some(icon) = self.icon {
            let galley = WidgetText::from(icon).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
            let center = disclosure_center + egui::vec2(indent, 0.0);
            painter.galley(center - galley.size() * 0.5, galley, glyph_color);
            label_left += indent;
        }
        let label_width = rect.right() - ui.spacing().button_padding.x - label_left;
        let galley = self.label.into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            label_width.max(1.0),
            egui::TextStyle::Button,
        );
        painter.galley(
            egui::pos2(label_left, rect.center().y - galley.size().y * 0.5),
            galley,
            text_color,
        );
        if row.has_focus() {
            paint_focus_ring(ui, rect);
        }

        TreeRowResponse { row, disclosure }
    }
}

/// What happened to a [`TreeRow`] during one frame.
#[derive(Clone, Debug)]
pub struct TreeRowResponse {
    /// The whole row; a click usually selects it.
    pub row: Response,
    /// The disclosure arrow of a row given [`TreeRow::expanded`].
    pub disclosure: Option<Response>,
}

impl TreeRowResponse {
    /// The arrow was clicked: the caller flips the row's expanded state.
    pub fn toggled(&self) -> bool {
        self.disclosure.as_ref().is_some_and(Response::clicked)
    }
}

/// Width of the arrow and icon columns, for a row `row_height` high: 20 px on
/// desktop and 25 px on Android.
fn column_width(row_height: f32) -> f32 {
    (row_height * 0.625).round()
}

/// A chevron centred on `center`, pointing right, or down when `expanded`,
/// sized for a column `column` wide.
fn paint_arrow(painter: &egui::Painter, center: Pos2, column: f32, expanded: bool, stroke: Stroke) {
    let half_length = column * 0.2;
    let half_depth = half_length * 0.5;
    let points = if expanded {
        [
            egui::vec2(-half_length, -half_depth),
            egui::vec2(0.0, half_depth),
            egui::vec2(half_length, -half_depth),
        ]
    } else {
        [
            egui::vec2(-half_depth, -half_length),
            egui::vec2(half_depth, 0.0),
            egui::vec2(-half_depth, half_length),
        ]
    };
    painter.line(points.map(|offset| center + offset).to_vec(), stroke);
}

/// The focus ring of a segmented control's segments and a switch.
fn paint_focus_ring(ui: &Ui, rect: Rect) {
    ui.painter().rect_stroke(
        rect,
        CARD_RADIUS,
        Stroke::new(1.5, ui.visuals().widgets.active.bg_stroke.color),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{click, last_widget_info, painted_text_rect, run_frame};

    /// What a collapsed, draggable row with children reported over the
    /// frames of a test.
    #[derive(Default)]
    struct Observed {
        toggled: bool,
        clicked: bool,
        row_hovered: bool,
        disclosure_hovered: bool,
    }

    /// Shows the row and returns the rect of the `part` under test.
    fn show_row(ui: &mut Ui, observed: &mut Observed, part: fn(&TreeRowResponse) -> Rect) -> Rect {
        tree(ui, |ui| {
            let response = TreeRow::new("photos", "Photos")
                .expanded(Some(false))
                .sense(Sense::click_and_drag())
                .show(ui);
            observed.toggled |= response.toggled();
            observed.clicked |= response.row.clicked();
            observed.row_hovered = response.row.hovered();
            observed.disclosure_hovered =
                response.disclosure.as_ref().is_some_and(Response::hovered);
            part(&response)
        })
        .inner
    }

    fn disclosure_rect(response: &TreeRowResponse) -> Rect {
        response
            .disclosure
            .as_ref()
            .expect("the row has an arrow")
            .rect
    }

    #[test]
    fn the_arrow_toggles_and_the_rest_of_the_row_clicks() {
        let ctx = egui::Context::default();
        let mut observed = Observed::default();
        let events = click(
            &ctx,
            |ui| show_row(ui, &mut observed, disclosure_rect),
            |rect| rect.center(),
        );
        assert!(observed.toggled);
        assert!(!observed.clicked);
        let info = last_widget_info(&events).expect("the arrow reports its click");
        assert_eq!(info.typ, egui::WidgetType::CollapsingHeader);
        assert_eq!(info.selected, Some(false));
        assert_eq!(info.label.as_deref(), Some("Photos"));

        let ctx = egui::Context::default();
        let mut observed = Observed::default();
        let events = click(
            &ctx,
            |ui| show_row(ui, &mut observed, |response| response.row.rect),
            |rect| rect.right_center() - egui::vec2(10.0, 0.0),
        );
        assert!(observed.clicked);
        assert!(!observed.toggled);
        let info = last_widget_info(&events).expect("the row reports its click");
        assert_eq!(info.typ, egui::WidgetType::SelectableLabel);
        assert_eq!(info.label.as_deref(), Some("Photos"));
    }

    #[test]
    fn hovering_the_arrow_of_a_draggable_row_hovers_only_the_arrow() {
        let ctx = egui::Context::default();
        let mut observed = Observed::default();
        let mut show = |ui: &mut Ui| show_row(ui, &mut observed, disclosure_rect);
        let (rect, _) = run_frame(&ctx, Vec::new(), &mut show);
        run_frame(
            &ctx,
            vec![egui::Event::PointerMoved(rect.center())],
            &mut show,
        );
        run_frame(&ctx, Vec::new(), &mut show);
        assert!(observed.disclosure_hovered);
        assert!(!observed.row_hovered);
    }

    #[test]
    fn a_childs_arrow_sits_under_its_parents_icon() {
        let ctx = egui::Context::default();
        crate::Design::ObsidianBlue.theme().apply(&ctx);
        let mut child_arrow = Rect::NOTHING;
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            tree(ui, |ui| {
                TreeRow::new("parent", "Parent")
                    .icon("P")
                    .expanded(Some(true))
                    .show(ui);
                child_arrow = disclosure_rect(
                    &TreeRow::new("child", "Child")
                        .depth(1)
                        .expanded(Some(false))
                        .show(ui),
                );
            });
        });
        let icon = painted_text_rect(&output.shapes, "P").expect("the icon is painted");
        assert!(
            (child_arrow.center().x - icon.center().x).abs() < 1.0,
            "{child_arrow:?} {icon:?}"
        );
    }
}
