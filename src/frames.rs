use egui::{
    self, Align, Color32, Frame, InnerResponse, Layout, Margin, Response, RichText, Stroke, Ui,
    Vec2,
};

use crate::forms::strong_with_help;
use crate::metrics::{
    CARD_RADIUS, CONTENT_MARGIN, CONTROL_HEIGHT, PANEL_TITLE_HEIGHT, PANEL_TITLE_TEXT_SIZE,
    SPACE_SM, SPACE_XS, TOOLBAR_HEIGHT, TOOLBAR_ICON_EDGE, TOOL_RAIL_ICON_EDGE,
};
use crate::responsive::{content_margin, is_compact_portrait};

pub fn toolbar_icon_size() -> Vec2 {
    Vec2::splat(TOOLBAR_ICON_EDGE)
}

pub fn tool_rail_icon_size() -> Vec2 {
    Vec2::splat(TOOL_RAIL_ICON_EDGE)
}

pub fn prepare_toolbar(ui: &mut Ui) {
    ui.set_min_height(TOOLBAR_HEIGHT);
    ui.spacing_mut().interact_size.y = CONTROL_HEIGHT;
    ui.spacing_mut().item_spacing = egui::vec2(SPACE_SM, SPACE_XS);
}

pub fn toolbar_row<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let size = egui::vec2(ui.available_width().max(1.0), TOOLBAR_HEIGHT);
    ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
        prepare_toolbar(ui);
        add_contents(ui)
    })
}

pub fn toolbar_title(ui: &mut Ui, title: impl Into<RichText>) -> Response {
    ui.label(title.into().strong().size(PANEL_TITLE_TEXT_SIZE))
}

pub fn panel_title(ui: &mut Ui, title: impl Into<RichText>) -> InnerResponse<Response> {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let title = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width().max(1.0), PANEL_TITLE_HEIGHT),
                Layout::left_to_right(Align::Center),
                |ui| ui.label(title.into().strong().size(PANEL_TITLE_TEXT_SIZE)),
            )
            .inner;
        ui.separator();
        title
    })
}

pub fn toolbar_frame(ui: &Ui) -> Frame {
    let compact = is_compact_portrait(ui);
    let stroke = if cfg!(target_os = "android") {
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    } else {
        Stroke::NONE
    };
    Frame::new()
        .fill(ui.visuals().panel_fill)
        .inner_margin(Margin::symmetric(
            if compact { 10 } else { CONTENT_MARGIN },
            if compact { 4 } else { 6 },
        ))
        .stroke(stroke)
        .corner_radius(0.0)
}

pub fn panel_frame(ui: &Ui) -> Frame {
    Frame::new()
        .fill(ui.visuals().panel_fill)
        .inner_margin(Margin::same(content_margin(ui)))
        .stroke(Stroke::NONE)
}

pub fn workspace_frame(ui: &Ui) -> Frame {
    Frame::new()
        .fill(ui.visuals().window_fill)
        .inner_margin(Margin::same(content_margin(ui)))
        .stroke(Stroke::NONE)
}

pub fn card_frame(ui: &Ui) -> Frame {
    Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(Margin::same(content_margin(ui) + 2))
        .corner_radius(CARD_RADIUS)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
}

pub struct Card;

impl Card {
    pub fn frame(ui: &Ui) -> Frame {
        card_frame(ui)
    }

    pub fn show<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        content_card(ui, add_contents)
    }

    pub fn section<R>(
        ui: &mut Ui,
        title: impl Into<RichText>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        section_card(ui, title, add_contents)
    }
}

pub fn content_card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let frame_width = f32::from(content_margin(ui)) * 2.0 + 6.0;
    let inner_width = (ui.available_width() - frame_width).max(1.0);
    card_frame(ui).show(ui, |ui| {
        ui.set_width(inner_width);
        ui.set_max_width(inner_width);
        add_contents(ui)
    })
}

pub fn card_header<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let inner_width = card_header_inner_width(ui);
    card_header_frame(ui, ui.visuals().faint_bg_color).show(ui, |ui| {
        size_card_header_content(ui, inner_width);
        add_contents(ui)
    })
}

fn card_header_frame(ui: &Ui, fill: Color32) -> Frame {
    let horizontal_margin = content_margin(ui) + 2;
    Frame::new()
        .fill(fill)
        .inner_margin(Margin::symmetric(horizontal_margin, 10))
        .corner_radius(CARD_RADIUS)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
}

fn card_header_inner_width(ui: &Ui) -> f32 {
    let frame_width = f32::from(content_margin(ui)) * 2.0 + 6.0;
    (ui.available_width() - frame_width).max(1.0)
}

fn size_card_header_content(ui: &mut Ui, inner_width: f32) {
    ui.set_width(inner_width);
    ui.set_max_width(inner_width);
}

pub fn progress_card_header<R>(
    ui: &mut Ui,
    fraction: f32,
    label: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let inner_width = card_header_inner_width(ui);
    let (track, fill) = if ui.visuals().dark_mode {
        (
            Color32::from_rgb(31, 43, 57),
            Color32::from_rgb(54, 88, 123),
        )
    } else {
        (
            Color32::from_rgb(221, 234, 247),
            Color32::from_rgb(166, 200, 231),
        )
    };
    let frame = card_header_frame(ui, track);
    let mut prepared = frame.begin(ui);
    let fill_index = ui.painter().add(egui::Shape::Noop);
    let text_index = ui.painter().add(egui::Shape::Noop);
    size_card_header_content(&mut prepared.content_ui, inner_width);
    let inner = add_contents(&mut prepared.content_ui);

    let rect = frame
        .widget_rect(prepared.content_ui.min_rect())
        .shrink(frame.stroke.width);
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction > 0.0 {
        let fill_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + rect.width() * fraction, rect.bottom()),
        );
        let radius = if fraction >= 1.0 { CARD_RADIUS } else { 0.0 };
        let corners = egui::CornerRadius {
            nw: CARD_RADIUS as u8,
            ne: radius as u8,
            sw: CARD_RADIUS as u8,
            se: radius as u8,
        };
        ui.painter().set(
            fill_index,
            egui::Shape::rect_filled(fill_rect, corners, fill),
        );
    }
    let text_color = ui.visuals().text_color();
    let text = ui.ctx().fonts_mut(|fonts| {
        egui::Shape::text(
            fonts,
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            text_color,
        )
    });
    ui.painter().set(text_index, text);
    let response = prepared.end(ui);
    InnerResponse::new(inner, response)
}

pub fn section_card<R>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    content_card(ui, |ui| {
        ui.strong(title);
        add_contents(ui)
    })
}

pub fn section_card_with_help<R>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    help: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    content_card(ui, |ui| {
        strong_with_help(ui, title, help);
        add_contents(ui)
    })
}

pub fn section_separator(ui: &mut Ui) -> Response {
    let extra_space = (SPACE_SM - ui.spacing().item_spacing.y).max(0.0);
    ui.add_space(extra_space);
    let response = ui.separator();
    ui.add_space(extra_space);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_title_text_is_vertically_centered() {
        egui::__run_test_ui(|ui| {
            ui.set_width(320.0);
            let row_top = ui.cursor().top();
            let title = panel_title(ui, "Edit").inner;
            let expected_center = row_top + PANEL_TITLE_HEIGHT * 0.5;
            assert!((title.rect.center().y - expected_center).abs() < 0.001);
        });
    }

    #[test]
    fn toolbar_row_centers_labels_and_actions_on_the_same_axis() {
        egui::__run_test_ui(|ui| {
            ui.set_width(320.0);
            let (label, action) = toolbar_row(ui, |ui| {
                let label = ui.strong("Section");
                let action = ui
                    .with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_sized(toolbar_icon_size(), egui::Button::new("R"))
                    })
                    .inner;
                (label.rect, action.rect)
            })
            .inner;

            assert!((label.center().y - action.center().y).abs() < 0.001);
            assert_eq!(action.size(), toolbar_icon_size());
        });
    }

    #[test]
    fn panel_and_workspace_content_share_the_same_inset() {
        egui::__run_test_ui(|ui| {
            assert_eq!(
                panel_frame(ui).inner_margin,
                workspace_frame(ui).inner_margin
            );
        });
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn desktop_toolbar_removes_border_without_removing_padding() {
        egui::__run_test_ui(|ui| {
            let frame = toolbar_frame(ui);
            assert_eq!(frame.inner_margin.left, CONTENT_MARGIN);
            assert_eq!(frame.inner_margin.right, CONTENT_MARGIN);
            assert_eq!(frame.stroke, Stroke::NONE);
        });
    }
}
