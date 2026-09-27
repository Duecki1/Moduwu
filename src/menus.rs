use egui::{self, Align2, InnerResponse, Response, Sense, StrokeKind, Ui};

pub fn menu_item(ui: &mut Ui, enabled: bool, label: impl Into<egui::WidgetText>) -> Response {
    ui.add_enabled(enabled, egui::Button::new(label))
}

pub fn destructive_menu_item(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> Response {
    let color = ui.visuals().error_fg_color;
    menu_item(ui, true, label.into().color(color))
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

/// Anchored overflow button with a menu popup.
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
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(add_contents);

    response.on_hover_text(tooltip)
}

pub fn context_menu<R>(
    response: &Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<InnerResponse<R>> {
    egui::Popup::context_menu(response)
        .style(egui::style::StyleModifier::default())
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(add_contents)
}

pub fn dropdown_menu<R>(
    response: &Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<InnerResponse<R>> {
    egui::Popup::menu(response)
        .style(egui::style::StyleModifier::default())
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(add_contents)
}

pub fn dropdown_submenu<'a, R>(
    ui: &mut Ui,
    label: impl egui::IntoAtoms<'a>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Response {
    let (response, _) = egui::menu::SubMenuButton::new(label)
        .config(egui::menu::MenuConfig::new().style(egui::style::StyleModifier::default()))
        .ui(ui, add_contents);
    response
}

pub fn context_menu_item<'a>(
    ui: &mut Ui,
    enabled: bool,
    label: impl egui::IntoAtoms<'a>,
) -> Response {
    ui.add_enabled(enabled, egui::Button::selectable(false, label))
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
    fn destructive_menu_items_keep_default_button_geometry() {
        egui::__run_test_ui(|ui| {
            let (normal, destructive) = ui
                .horizontal(|ui| {
                    let normal = ui.button("Delete");
                    let destructive = destructive_menu_item(ui, "Delete");
                    (normal.rect.size(), destructive.rect.size())
                })
                .inner;

            assert_eq!(normal, destructive);
        });
    }
}
