//! Menus: context menus, dropdowns and their rows. On desktop layouts a
//! menu is a popup beside what opened it; on touch layouts it is a sheet that
//! rises from the bottom of the viewport ([`crate::menu_sheet`]). The rows
//! and submenus below lay themselves out for either.

use egui::{
    self, Align2, InnerResponse, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget,
    WidgetText,
};

use crate::menu_sheet::{self, Row, Trailing};
use crate::metrics::Metrics;
use crate::touch_hold::long_pressed;

/// A row of a menu: an action such as Rename or Delete, or one of several
/// choices. In a popup it is a compact text row; in a sheet it is a tall row
/// with its icon before the label. Add it with `ui.add`, or with
/// `ui.add_enabled` to disable it.
#[must_use = "add it to a menu with `ui.add`"]
pub struct MenuItem<'a> {
    label: WidgetText,
    icon: Option<&'a str>,
    value: Option<String>,
    selected: Option<bool>,
    destructive: bool,
}

impl<'a> MenuItem<'a> {
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            value: None,
            selected: None,
            destructive: false,
        }
    }

    /// The current value of what the row sets, such as the sort order, shown
    /// dimmed at the row's end. Mostly for [`submenu`] rows.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Makes the row one of several choices, checked when `selected`: a check
    /// mark before the label in a popup, after it in a sheet. The caller
    /// decides what a click selects, so a list can allow one choice or
    /// several, and whether it closes the menu.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// The glyph shown before the label in a sheet. The application supplies
    /// it, so the design system does not depend on an icon font.
    pub fn icon(mut self, glyph: &'a str) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Colours the row as an action that destroys data, such as Delete.
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    fn sheet_row<'v>(self, trailing: Trailing<'v>) -> Row<'a, 'v> {
        Row {
            label: self.label,
            icon: self.icon,
            icon_column: self.icon.is_some(),
            value: self.value,
            destructive: self.destructive,
            trailing,
        }
    }
}

impl Widget for MenuItem<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let text = self.label.text().to_owned();
        let enabled = ui.is_enabled();
        let selected = self.selected;
        let response = if menu_sheet::tag(ui).is_some() {
            let trailing = selected.map_or(Trailing::Nothing, Trailing::Check);
            self.sheet_row(trailing).show(ui)
        } else if let Some(mut selected) = selected {
            popup_check_row(ui, self.label, &mut selected, false)
        } else {
            let label = if self.destructive {
                self.label.color(ui.visuals().error_fg_color)
            } else {
                self.label
            };
            let mut button = egui::Button::new(label);
            if let Some(value) = self.value {
                button = button.right_text(egui::RichText::new(value).weak());
            }
            ui.add(button)
        };
        response.widget_info(|| match selected {
            Some(selected) => egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                enabled,
                selected,
                &text,
            ),
            None => egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &text),
        });
        response
    }
}

/// A row that opens a submenu: a cascading popup in a popup menu, a page with
/// a back row in a sheet. Returns the row's response and, while the submenu
/// shows, what `add_contents` returned. As in the menu itself, only a row
/// that calls `ui.close()` closes the menu, so a list of several choices
/// stays open while they are picked.
pub fn submenu<R>(
    ui: &mut Ui,
    item: MenuItem<'_>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    if let Some(tag) = menu_sheet::tag(ui) {
        return menu_sheet::submenu(ui, tag, item.sheet_row(Trailing::Chevron), add_contents);
    }
    let arrow = egui::menu::SubMenuButton::RIGHT_ARROW;
    let right_text = match &item.value {
        Some(value) => egui::RichText::new(format!("{value}  {arrow}")).weak(),
        None => egui::RichText::new(arrow),
    };
    let (response, inner) = egui::menu::SubMenuButton::from_button(
        egui::Button::new(item.label).right_text(right_text),
    )
    .config(
        egui::menu::MenuConfig::new()
            .style(menu_popup_style())
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
    )
    .ui(ui, add_contents);
    InnerResponse::new(inner.map(|inner| inner.inner), response)
}

/// The title of a menu, such as the name of the photo it acts on. A sheet
/// shows it above its rows, as the finger that opened the menu covers what it
/// pressed; a popup, which opens beside it, leaves it out.
pub fn menu_title(ui: &mut Ui, title: &str) {
    if menu_sheet::tag(ui).is_some() {
        menu_sheet::title(ui, title, false);
    }
}

/// The title of a menu whose options leave it open, such as filters. A sheet
/// shows it with a close button, as none of its options closes it; a popup
/// leaves it out and closes on a click outside.
pub fn menu_title_with_close(ui: &mut Ui, title: &str) {
    if menu_sheet::tag(ui).is_some() {
        menu_sheet::title(ui, title, true);
    }
}

/// A menu row for an on/off option. Clicking it, or Space and Enter while it
/// has focus, flips `value` and marks the response changed; assistive
/// technology sees a checkbox. In a popup a check mark before the label shows
/// the value; in a sheet a switch after it does. The menu stays open, so the
/// caller decides whether to close it.
pub fn menu_checkbox_item(
    ui: &mut Ui,
    value: &mut bool,
    label: impl Into<egui::WidgetText>,
) -> Response {
    let label = label.into();
    let text = label.text().to_owned();
    let response = if menu_sheet::tag(ui).is_some() {
        Row {
            label,
            icon: None,
            icon_column: true,
            value: None,
            destructive: false,
            trailing: Trailing::Switch(value),
        }
        .show(ui)
    } else {
        popup_check_row(ui, label, value, true)
    };
    let (enabled, checked) = (ui.is_enabled(), *value);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, checked, &text)
    });
    response
}

/// A popup's row with a check mark, when `checked` is set, before the label.
/// A click flips `checked` when the row `toggles`.
fn popup_check_row(ui: &mut Ui, label: WidgetText, checked: &mut bool, toggles: bool) -> Response {
    let spacing = ui.spacing();
    let (padding, check_edge, gap) = (
        spacing.button_padding,
        spacing.icon_width,
        spacing.icon_spacing,
    );
    let galley = label.into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let desired = egui::vec2(
        padding.x * 2.0 + check_edge + gap + galley.size().x,
        ui.spacing()
            .interact_size
            .y
            .max(galley.size().y + padding.y * 2.0),
    );
    let (rect, mut response) = ui.allocate_at_least(desired, Sense::click());
    if toggles && response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    let checked = *checked;
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let visuals = ui.style().interact(&response);
    let painter = ui.painter();
    if response.hovered() || response.has_focus() {
        painter.rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            StrokeKind::Inside,
        );
    }
    let check = egui::Rect::from_center_size(
        egui::pos2(rect.left() + padding.x + check_edge * 0.5, rect.center().y),
        egui::Vec2::splat(check_edge),
    );
    if checked {
        menu_sheet::paint_check(ui, check, visuals.text_color());
    }
    let text_pos = egui::pos2(check.right() + gap, rect.center().y - galley.size().y * 0.5);
    painter.galley(text_pos, galley, visuals.text_color());
    response
}

const OVERFLOW_INSET: f32 = 5.0;

fn overflow_button_rect(anchor_rect: egui::Rect, edge: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            anchor_rect.right() - edge - OVERFLOW_INSET,
            anchor_rect.top() + OVERFLOW_INSET,
        ),
        egui::vec2(edge, edge),
    )
}

fn overflow_button(
    ui: &mut Ui,
    anchor_rect: egui::Rect,
    id: egui::Id,
    edge: f32,
    glyph: &str,
) -> Response {
    let button_rect = overflow_button_rect(anchor_rect, edge);
    let touch_edge = ui
        .spacing()
        .interact_size
        .x
        .max(ui.spacing().interact_size.y)
        .max(edge);
    let hit_rect = egui::Rect::from_min_size(
        egui::pos2(anchor_rect.right() - touch_edge, anchor_rect.top()),
        egui::vec2(touch_edge, touch_edge),
    )
    .intersect(anchor_rect);
    let response = ui.interact(hit_rect, id, Sense::click());
    let visuals = ui.style().interact(&response);
    let radius = (edge * 0.22).clamp(3.0, 7.0);
    let painter = ui.painter_at(button_rect);
    painter.rect_filled(button_rect, radius, visuals.weak_bg_fill);
    painter.rect_stroke(button_rect, radius, visuals.bg_stroke, StrokeKind::Inside);
    painter.text(
        button_rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(edge * 0.52),
        visuals.fg_stroke.color,
    );
    response
}

/// A menu button in the top right corner of `anchor_rect`, such as a card:
/// touch layouts have no right-click, so it opens the card's menu.
///
/// The glyph is supplied by the application so the design system does not depend on a
/// particular icon font.
pub fn overflow_menu<R>(
    ui: &mut Ui,
    anchor_rect: egui::Rect,
    id: egui::Id,
    edge: f32,
    glyph: &str,
    tooltip: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Response {
    let response = overflow_button(ui, anchor_rect, id, edge, glyph);
    dropdown_menu(&response, add_contents);
    response.on_hover_text(tooltip)
}

/// The menu of `response`'s widget. A popup opens on a right-click; on touch
/// layouts a sheet opens on a right-click or a long press, timed with
/// [`crate::set_long_press_duration`]. Returns what `add_contents` returned
/// while the menu shows.
pub fn context_menu<R>(response: &Response, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let ctx = &response.ctx;
    if Metrics::of(ctx).touch_layout {
        if response.secondary_clicked() || long_pressed(response) {
            menu_sheet::open(ctx, response.id);
        }
        return menu_sheet::show(ctx, response.id, add_contents);
    }
    egui::Popup::context_menu(response)
        .style(menu_popup_style())
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(add_contents)
        .map(|shown| shown.inner)
}

/// The menu a click on `response`'s button opens: a popup below the button,
/// or a sheet on touch layouts. Returns what `add_contents` returned while the
/// menu shows.
pub fn dropdown_menu<R>(response: &Response, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    let ctx = &response.ctx;
    if Metrics::of(ctx).touch_layout {
        if response.clicked() {
            menu_sheet::open(ctx, response.id);
        }
        return menu_sheet::show(ctx, response.id, add_contents);
    }
    egui::Popup::menu(response)
        .style(menu_popup_style())
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(add_contents)
        .map(|shown| shown.inner)
}

/// The style of Moduwu menus and dropdown lists: rows are flat until hovered.
/// The idle border is removed, not just hidden, so rows that egui draws
/// frameless while idle (selectable rows) keep their label in place on hover.
pub(crate) fn menu_popup_style() -> egui::style::StyleModifier {
    egui::style::StyleModifier::new(|style: &mut egui::Style| {
        let idle = &mut style.visuals.widgets.inactive;
        idle.weak_bg_fill = egui::Color32::TRANSPARENT;
        idle.bg_stroke = Stroke::NONE;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_button_stays_inside_anchor() {
        let anchor = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(120.0, 80.0));
        let button = overflow_button_rect(anchor, 22.0);

        assert_eq!(button.size(), egui::vec2(22.0, 22.0));
        assert_eq!(button.right(), anchor.right() - OVERFLOW_INSET);
        assert_eq!(button.top(), anchor.top() + OVERFLOW_INSET);
        assert!(anchor.contains(button.min));
        assert!(anchor.contains(button.max));
    }

    #[test]
    fn overflow_button_does_not_rewind_vertical_layout() {
        egui::__run_test_ui(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let (card_rect, _) =
                ui.allocate_exact_size(egui::vec2(68.0, 72.0), egui::Sense::click());
            let next_position = ui.next_widget_position();

            let overflow =
                overflow_button(ui, card_rect, egui::Id::new("test-overflow"), 22.0, "⋮");

            assert_eq!(ui.next_widget_position(), next_position);
            assert!(card_rect.contains(overflow.rect.min));
            assert!(card_rect.contains(overflow.rect.max));

            let (next_rect, _) =
                ui.allocate_exact_size(egui::vec2(56.0, 62.0), egui::Sense::click());
            assert!(next_rect.top() >= card_rect.bottom());
        });
    }

    #[test]
    fn menu_checkbox_items_flip_and_report_a_checkbox() {
        let ctx = egui::Context::default();
        let mut value = false;
        let mut changed = false;
        let events = crate::test_support::click(
            &ctx,
            |ui| {
                let response = menu_checkbox_item(ui, &mut value, "Invert");
                changed |= response.changed();
                response.rect
            },
            |rect| rect.center(),
        );
        assert!(value);
        assert!(changed);
        let info = crate::test_support::last_widget_info(&events).expect("item reports its click");
        assert_eq!(info.typ, egui::WidgetType::Checkbox);
        assert_eq!(info.selected, Some(true));
        assert_eq!(info.label.as_deref(), Some("Invert"));
    }

    #[test]
    fn menu_checkbox_items_match_menu_item_height() {
        egui::__run_test_ui(|ui| {
            let mut value = true;
            let item = ui.add(MenuItem::new("Duplicate")).rect.height();
            let checkbox = menu_checkbox_item(ui, &mut value, "Invert").rect.height();
            assert_eq!(checkbox, item);
        });
    }

    #[test]
    fn selected_menu_items_report_their_state() {
        let ctx = egui::Context::default();
        let events = crate::test_support::click(
            &ctx,
            |ui| ui.add(MenuItem::new("Newest first").selected(true)).rect,
            |rect| rect.center(),
        );
        let info = crate::test_support::last_widget_info(&events).expect("item reports its click");
        assert_eq!(info.typ, egui::WidgetType::SelectableLabel);
        assert_eq!(info.selected, Some(true));
        assert_eq!(info.label.as_deref(), Some("Newest first"));
    }

    #[test]
    fn destructive_menu_items_keep_default_button_geometry() {
        egui::__run_test_ui(|ui| {
            let (normal, destructive) = ui
                .horizontal(|ui| {
                    let normal = ui.button("Delete");
                    let destructive = ui.add(MenuItem::new("Delete").destructive());
                    (normal.rect.size(), destructive.rect.size())
                })
                .inner;

            assert_eq!(normal, destructive);
        });
    }

    use crate::menu_sheet::{dismiss_menu_sheet, menu_sheet_open};
    use crate::test_support::touch;
    use egui::{Context, Event, Pos2, Rect, TouchPhase};
    use std::cell::Cell;

    const VIEWPORT: egui::Vec2 = egui::vec2(412.0, 860.0);

    fn touch_context() -> Context {
        let ctx = Context::default();
        Metrics::ANDROID.install(&ctx);
        ctx
    }

    fn frame(ctx: &Context, time: f64, events: Vec<Event>, show: &mut impl FnMut(&mut Ui)) {
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, VIEWPORT)),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| show(ui),
        );
    }

    /// Taps `at` starting at `time`, then draws one more frame, which a sheet
    /// or page the tap opened needs to lay out: egui sizes a new area unseen.
    fn tap(ctx: &Context, time: f64, at: Pos2, show: &mut impl FnMut(&mut Ui)) {
        frame(ctx, time, touch(TouchPhase::Start, at), show);
        frame(ctx, time + 0.05, touch(TouchPhase::End, at), show);
        frame(ctx, time + 0.1, Vec::new(), show);
    }

    #[test]
    fn a_long_press_opens_a_bottom_sheet_that_a_row_closes() {
        let ctx = touch_context();
        let row = Cell::new(Rect::NOTHING);
        let renamed = Cell::new(false);
        let mut show = |ui: &mut Ui| {
            let photo = ui.allocate_response(egui::vec2(200.0, 100.0), Sense::click());
            context_menu(&photo, |ui| {
                let rename = ui.add(MenuItem::new("Rename").icon("R"));
                row.set(rename.rect);
                if rename.clicked() {
                    renamed.set(true);
                    ui.close();
                }
            });
        };
        frame(&ctx, 0.0, Vec::new(), &mut show);
        frame(
            &ctx,
            0.1,
            touch(TouchPhase::Start, egui::pos2(50.0, 50.0)),
            &mut show,
        );
        frame(&ctx, 0.3, Vec::new(), &mut show);
        assert!(!menu_sheet_open(&ctx), "the press is not long yet");
        frame(&ctx, 0.6, Vec::new(), &mut show);
        assert!(menu_sheet_open(&ctx));
        // Lifting the finger that opened the sheet leaves it open.
        frame(
            &ctx,
            0.7,
            touch(TouchPhase::End, egui::pos2(50.0, 50.0)),
            &mut show,
        );
        assert!(menu_sheet_open(&ctx));
        let row = row.get();
        assert_eq!(row.height(), 48.0);
        assert!(
            row.width() > VIEWPORT.x * 0.8,
            "rows span the sheet: {row:?}"
        );
        assert!(
            row.bottom() > VIEWPORT.y - 100.0,
            "the sheet is at the bottom: {row:?}"
        );

        tap(&ctx, 1.0, row.center(), &mut show);
        assert!(renamed.get());
        assert!(!menu_sheet_open(&ctx), "`ui.close()` closes the sheet");
    }

    #[test]
    fn a_title_close_button_closes_a_sheet() {
        let ctx = touch_context();
        let (button, title_row) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
        let mut show = |ui: &mut Ui| {
            let menu = ui.button("Menu");
            button.set(menu.rect);
            dropdown_menu(&menu, |ui| {
                menu_title_with_close(ui, "Sort & filter");
                title_row.set(ui.min_rect());
                let mut checked = false;
                menu_checkbox_item(ui, &mut checked, "Show only the RAW");
            });
        };
        frame(&ctx, 0.0, Vec::new(), &mut show);
        tap(&ctx, 0.1, button.get().center(), &mut show);
        assert!(menu_sheet_open(&ctx));
        let row = title_row.get();
        let close = egui::pos2(row.right() - row.height() * 0.5, row.center().y);
        tap(&ctx, 0.3, close, &mut show);
        assert!(!menu_sheet_open(&ctx));
    }

    #[test]
    fn a_sheet_submenu_opens_a_page_that_back_leaves() {
        let ctx = touch_context();
        let (button, file_row, copy_row) = (
            Cell::new(Rect::NOTHING),
            Cell::new(Rect::NOTHING),
            Cell::new(Rect::NOTHING),
        );
        let rename_visible = Cell::new(false);
        let copied = Cell::new(false);
        let mut show = |ui: &mut Ui| {
            let menu = ui.button("Menu");
            button.set(menu.rect);
            dropdown_menu(&menu, |ui| {
                rename_visible.set(ui.is_visible());
                let _ = ui.add(MenuItem::new("Rename"));
                let file = submenu(ui, MenuItem::new("File"), |ui| {
                    let copy = ui.add(MenuItem::new("Copy"));
                    if copy.clicked() {
                        copied.set(true);
                        ui.close();
                    }
                    copy.rect
                });
                file_row.set(file.response.rect);
                if let Some(rect) = file.inner {
                    copy_row.set(rect);
                }
            });
        };
        frame(&ctx, 0.0, Vec::new(), &mut show);
        tap(&ctx, 0.1, button.get().center(), &mut show);
        assert!(menu_sheet_open(&ctx));
        assert!(rename_visible.get());

        tap(&ctx, 0.5, file_row.get().center(), &mut show);
        assert!(!rename_visible.get(), "the page replaces the menu");
        assert!(copy_row.get().is_positive());

        let escape = vec![crate::test_support::key_press(egui::Key::Escape)];
        frame(&ctx, 0.7, escape, &mut show);
        frame(&ctx, 0.8, Vec::new(), &mut show);
        assert!(menu_sheet_open(&ctx), "Escape goes back to the menu");
        assert!(rename_visible.get());

        tap(&ctx, 1.0, file_row.get().center(), &mut show);
        tap(&ctx, 1.2, copy_row.get().center(), &mut show);
        assert!(copied.get());
        assert!(!menu_sheet_open(&ctx));
    }

    #[test]
    fn back_closes_the_page_then_the_sheet_and_an_undrawn_menu_closes() {
        let ctx = touch_context();
        let (button, file_row) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
        let draw = Cell::new(true);
        let mut show = |ui: &mut Ui| {
            let menu = ui.button("Menu");
            button.set(menu.rect);
            if draw.get() {
                dropdown_menu(&menu, |ui| {
                    file_row.set(submenu(ui, MenuItem::new("File"), |_| ()).response.rect);
                });
            }
        };
        frame(&ctx, 0.0, Vec::new(), &mut show);
        tap(&ctx, 0.1, button.get().center(), &mut show);
        tap(&ctx, 0.3, file_row.get().center(), &mut show);
        assert!(dismiss_menu_sheet(&ctx), "back leaves the page");
        assert!(menu_sheet_open(&ctx));
        assert!(dismiss_menu_sheet(&ctx), "back closes the sheet");
        assert!(!dismiss_menu_sheet(&ctx), "nothing is left to close");

        // egui lets the closed sheet block input for one more frame.
        frame(&ctx, 0.4, Vec::new(), &mut show);
        tap(&ctx, 0.5, button.get().center(), &mut show);
        assert!(menu_sheet_open(&ctx));
        draw.set(false);
        frame(&ctx, 0.7, Vec::new(), &mut show);
        assert!(!menu_sheet_open(&ctx));
    }

    #[test]
    fn a_page_whose_submenu_is_gone_returns_to_the_menu() {
        let ctx = touch_context();
        let (button, file_row) = (Cell::new(Rect::NOTHING), Cell::new(Rect::NOTHING));
        let with_submenu = Cell::new(true);
        let menu_visible = Cell::new(false);
        let mut show = |ui: &mut Ui| {
            let menu = ui.button("Menu");
            button.set(menu.rect);
            dropdown_menu(&menu, |ui| {
                menu_visible.set(ui.is_visible());
                if with_submenu.get() {
                    file_row.set(submenu(ui, MenuItem::new("File"), |_| ()).response.rect);
                }
            });
        };
        frame(&ctx, 0.0, Vec::new(), &mut show);
        tap(&ctx, 0.1, button.get().center(), &mut show);
        tap(&ctx, 0.3, file_row.get().center(), &mut show);
        assert!(!menu_visible.get());
        with_submenu.set(false);
        frame(&ctx, 0.5, Vec::new(), &mut show);
        frame(&ctx, 0.6, Vec::new(), &mut show);
        assert!(menu_sheet_open(&ctx));
        assert!(menu_visible.get());
    }
}
