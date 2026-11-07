//! Icon tiles, framed buttons that show an icon above a name, and the popup
//! that holds a grid of them for picking one.

use egui::{self, Response, Sense, StrokeKind, TextWrapMode, Ui, Widget};

use crate::buttons::interaction_visuals;
use crate::menu_sheet;
use crate::metrics::{Metrics, CARD_RADIUS, CONTROL_HEIGHT, SPACE_XS, SPACE_XXS};

/// The height and text sizes of an [`IconTile`]. Heights scale with the
/// platform's control height, so tiles stay touch-sized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileSize {
    pub height: f32,
    /// Size of the icon glyph, in points.
    pub icon: f32,
    /// Size of the name and of the badge, in points.
    pub label: f32,
}

impl TileSize {
    /// For the few choices a picker leads with, two to a row.
    pub const LARGE: Self = Self {
        height: CONTROL_HEIGHT * 2.4,
        icon: 30.0,
        label: 13.0,
    };
    /// The usual tile, three to a row.
    pub const MEDIUM: Self = Self {
        height: CONTROL_HEIGHT * 1.6,
        icon: 20.0,
        label: 11.5,
    };
    /// For the rarer choices, four to a row.
    pub const SMALL: Self = Self {
        height: CONTROL_HEIGHT * 1.3,
        icon: 16.0,
        label: 10.5,
    };
}

/// A button showing an icon above its name, framed like a card. The icon and
/// the name are centred as one block; a name wider than the tile is
/// truncated. An optional badge glyph sits in the top-right corner, such as a
/// check mark on a choice that is already taken.
///
/// It is a button for assistive technology, reporting the name, the enabled
/// state and the selected state. Disable it with [`Ui::add_enabled`].
#[must_use = "add it with `ui.add(tile)`"]
pub struct IconTile<'a> {
    icon: &'a str,
    label: &'a str,
    size: TileSize,
    width: Option<f32>,
    selected: bool,
    badge: Option<&'a str>,
}

impl<'a> IconTile<'a> {
    /// A [`TileSize::MEDIUM`] tile as wide as the available width.
    pub fn new(icon: &'a str, label: &'a str) -> Self {
        Self {
            icon,
            label,
            size: TileSize::MEDIUM,
            width: None,
            selected: false,
            badge: None,
        }
    }

    pub fn size(mut self, size: TileSize) -> Self {
        self.size = size;
        self
    }

    /// The tile's width; [`tile_width`] fits a number of tiles to a row.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Reports the tile as selected to assistive technology. It does not
    /// change how the tile is drawn; pair it with a badge to show it.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// A glyph drawn in the top-right corner at the name's size.
    pub fn badge(mut self, badge: &'a str) -> Self {
        self.badge = Some(badge);
        self
    }
}

impl Widget for IconTile<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, self.size.height), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                ui.is_enabled(),
                self.selected,
                self.label,
            )
        });
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let visuals = interaction_visuals(ui, &response, false);
        let painter = ui.painter();
        painter.rect(
            rect,
            CARD_RADIUS,
            visuals.weak_fill,
            visuals.stroke,
            StrokeKind::Inside,
        );

        let padding = ui.spacing().button_padding.x;
        let label = egui::WidgetText::from(egui::RichText::new(self.label).size(self.size.label))
            .into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                rect.width() - 2.0 * padding,
                egui::TextStyle::Button,
            );
        let icon_gap = SPACE_XXS;
        let top = rect.center().y - (self.size.icon + icon_gap + label.size().y) * 0.5;
        painter.text(
            egui::pos2(rect.center().x, top + self.size.icon * 0.5),
            egui::Align2::CENTER_CENTER,
            self.icon,
            egui::FontId::proportional(self.size.icon),
            visuals.foreground,
        );
        painter.galley(
            egui::pos2(
                rect.center().x - label.size().x * 0.5,
                top + self.size.icon + icon_gap,
            ),
            label,
            visuals.foreground,
        );
        if let Some(badge) = self.badge {
            painter.text(
                rect.right_top() + egui::vec2(-padding, padding),
                egui::Align2::RIGHT_TOP,
                badge,
                egui::FontId::proportional(self.size.label),
                visuals.foreground,
            );
        }
        response
    }
}

/// The width of each of `per_row` tiles filling the available width, with
/// the horizontal item spacing between them.
pub fn tile_width(ui: &Ui, per_row: usize) -> f32 {
    let per_row = per_row.max(1) as f32;
    (ui.available_width() - ui.spacing().item_spacing.x * (per_row - 1.0)) / per_row
}

/// Where a [`picker_popup`] opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerPlacement {
    /// Below or above the button, on the side with more room, and at least
    /// as wide as the button.
    BelowOrAbove,
    /// Left of the button, from its top: for buttons in a panel along the
    /// right edge.
    Left,
}

/// Opens a picker when `button` is clicked: a popup at least `min_width`
/// wide that scrolls when its contents are taller than the room on its side.
/// `add_contents` draws the choices and returns the one clicked, which
/// closes the popup. Returns that choice.
pub fn picker_popup<R>(
    button: &Response,
    min_width: f32,
    placement: PickerPlacement,
    add_contents: impl FnOnce(&mut Ui) -> Option<R>,
) -> Option<R> {
    let screen = button.ctx.content_rect();
    let gap = SPACE_XS;
    // Chosen here rather than by egui, which learns the popup's height only
    // after showing it once and would move it on the second frame.
    let (align, size) = match placement {
        PickerPlacement::BelowOrAbove => {
            let room_above = button.rect.top() - screen.top();
            let room_below = screen.bottom() - button.rect.bottom();
            let align = if room_below >= room_above {
                egui::RectAlign::BOTTOM_START
            } else {
                egui::RectAlign::TOP_START
            };
            let size = egui::vec2(
                button.rect.width().max(min_width),
                room_above.max(room_below) - 2.0 * gap,
            );
            (align, size)
        }
        PickerPlacement::Left => (
            egui::RectAlign::LEFT_START,
            egui::vec2(min_width, screen.bottom() - button.rect.top() - 2.0 * gap),
        ),
    };
    // Not a menu: menus draw idle buttons flat, and tiles keep their frame.
    egui::Popup::from_toggle_button_response(button)
        .align(align)
        .align_alternatives(&[])
        .gap(gap)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| picker_body(ui, size, add_contents))
        .and_then(|response| response.inner)
}

/// A picker that opens as menus do: [`picker_popup`] on pointer layouts, and
/// on touch layouts a sheet rising from the bottom of the viewport, as wide
/// as the viewport up to the sheets' limit. A choice closes either. Returns
/// what `add_contents` picked.
pub fn picker_menu<R>(
    button: &Response,
    min_width: f32,
    placement: PickerPlacement,
    add_contents: impl FnOnce(&mut Ui) -> Option<R>,
) -> Option<R> {
    let ctx = &button.ctx;
    if !Metrics::of(ctx).touch_layout {
        return picker_popup(button, min_width, placement, add_contents);
    }
    if button.clicked() {
        menu_sheet::open(ctx, button.id);
    }
    menu_sheet::show(ctx, button.id, |ui| {
        let picked = add_contents(ui);
        if picked.is_some() {
            ui.close();
        }
        picked
    })
    .flatten()
}

/// Fills a popup of at most `size` with a scroll area around `add_contents`,
/// and closes the popup once it returns a choice.
fn picker_body<R>(
    ui: &mut Ui,
    size: egui::Vec2,
    add_contents: impl FnOnce(&mut Ui) -> Option<R>,
) -> Option<R> {
    // The popup frame's margin and border lie outside the contents.
    let size = size
        - ui.spacing().menu_margin.sum()
        - egui::Vec2::splat(2.0 * ui.visuals().window_stroke.width);
    ui.set_width(size.x);
    // A solid scrollbar, so a picker taller than its room shows that more
    // choices follow.
    let mut scroll = egui::style::ScrollStyle::solid();
    scroll.bar_width = 7.0;
    scroll.bar_inner_margin = 7.0;
    ui.spacing_mut().scroll = scroll;
    // A popup offers its contents only the height it had the frame before,
    // which would keep the scroll area at its first height. The minimum lets
    // it grow to its contents, up to `size.y`.
    let picked = egui::ScrollArea::vertical()
        .max_height(size.y)
        .min_scrolled_height(size.y)
        .show(ui, add_contents)
        .inner;
    if picked.is_some() {
        ui.close();
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{click, last_widget_info};

    #[test]
    fn clicking_a_tile_reports_a_selected_button_with_its_name() {
        let ctx = egui::Context::default();
        let mut clicked = false;
        let events = click(
            &ctx,
            |ui| {
                let response = ui.add(IconTile::new("+", "Subject").selected(true).width(120.0));
                clicked |= response.clicked();
                response.rect
            },
            |rect| rect.center(),
        );
        assert!(clicked);
        let info = last_widget_info(&events).expect("the tile reports its click");
        assert_eq!(info.typ, egui::WidgetType::Button);
        assert_eq!(info.label.as_deref(), Some("Subject"));
        assert_eq!(info.selected, Some(true));
    }

    #[test]
    fn tile_width_fills_the_row_with_the_spacing_between_tiles() {
        egui::__run_test_ui(|ui| {
            let available = ui.available_width();
            let spacing = ui.spacing().item_spacing.x;
            let width = tile_width(ui, 3);
            assert!((width * 3.0 + spacing * 2.0 - available).abs() < 1e-3);
        });
    }
}
