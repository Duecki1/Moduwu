use egui::{self, Color32, InnerResponse, Response, RichText, Stroke, Ui, Vec2, Widget};

use crate::metrics::{
    control_height, CARD_RADIUS, FLOATING_ACTION_EDGE, FLOATING_ACTION_MARGIN, SPACE_SM,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionVisualState {
    Disabled,
    Active,
    Selected,
    Focused,
    Hovered,
    Inactive,
}

#[derive(Clone, Copy, Debug)]
pub struct InteractionVisuals {
    pub state: InteractionVisualState,
    pub fill: Color32,
    pub weak_fill: Color32,
    pub stroke: Stroke,
    pub foreground: Color32,
}

pub const fn interaction_visual_state(
    enabled: bool,
    selected: bool,
    active: bool,
    hovered: bool,
    focused: bool,
) -> InteractionVisualState {
    if !enabled {
        InteractionVisualState::Disabled
    } else if active {
        InteractionVisualState::Active
    } else if selected {
        InteractionVisualState::Selected
    } else if focused {
        InteractionVisualState::Focused
    } else if hovered {
        InteractionVisualState::Hovered
    } else {
        InteractionVisualState::Inactive
    }
}

pub fn interaction_visuals_for_flags(
    ui: &Ui,
    enabled: bool,
    selected: bool,
    active: bool,
    hovered: bool,
    focused: bool,
) -> InteractionVisuals {
    let state = interaction_visual_state(enabled, selected, active, hovered, focused);
    let visuals = ui.visuals();
    if state == InteractionVisualState::Selected {
        return InteractionVisuals {
            state,
            fill: visuals.selection.bg_fill,
            weak_fill: visuals.selection.bg_fill,
            stroke: visuals.selection.stroke,
            foreground: visuals.selection.stroke.color,
        };
    }

    let widget = match state {
        InteractionVisualState::Disabled => &visuals.widgets.noninteractive,
        InteractionVisualState::Active => &visuals.widgets.active,
        InteractionVisualState::Focused | InteractionVisualState::Hovered => {
            &visuals.widgets.hovered
        }
        InteractionVisualState::Inactive => &visuals.widgets.inactive,
        InteractionVisualState::Selected => unreachable!(),
    };
    InteractionVisuals {
        state,
        fill: widget.bg_fill,
        weak_fill: widget.weak_bg_fill,
        stroke: widget.bg_stroke,
        foreground: widget.fg_stroke.color,
    }
}

pub fn interaction_visuals(ui: &Ui, response: &Response, selected: bool) -> InteractionVisuals {
    interaction_visuals_for_flags(
        ui,
        response.enabled(),
        selected,
        response.is_pointer_button_down_on(),
        response.hovered() || response.highlighted(),
        response.has_focus(),
    )
}

/// Semantic primary action button using the active widget visuals.
pub struct PrimaryButton {
    label: egui::WidgetText,
    width: Option<f32>,
}

impl PrimaryButton {
    pub fn new(label: impl Into<egui::WidgetText>) -> Self {
        Self {
            label: label.into(),
            width: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl Widget for PrimaryButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let visuals = &ui.visuals().widgets.active;
        let button = egui::Button::new(self.label.color(Color32::WHITE))
            .fill(visuals.weak_bg_fill)
            .stroke(visuals.bg_stroke)
            .corner_radius(CARD_RADIUS);
        if let Some(width) = self.width {
            ui.add_sized([width, control_height(ui)], button)
        } else {
            ui.add(button.min_size(egui::vec2(0.0, control_height(ui))))
        }
    }
}

pub fn full_width_button(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> Response {
    ui.add_sized(
        [ui.available_width().max(1.0), control_height(ui)],
        egui::Button::new(label.into()),
    )
}

pub fn tab_button(ui: &mut Ui, label: &str, selected: bool, width: f32) -> Response {
    segmented_button(ui, RichText::new(label).strong(), selected, width)
}

pub fn segmented_button(
    ui: &mut Ui,
    label: impl Into<egui::WidgetText>,
    selected: bool,
    width: f32,
) -> Response {
    ui.add_sized(
        [width, control_height(ui)],
        egui::Button::new(label.into())
            .selected(selected)
            .frame(true)
            .truncate()
            .corner_radius(CARD_RADIUS),
    )
}

pub fn toolbar_button(ui: &mut Ui, label: impl Into<egui::WidgetText>, width: f32) -> Response {
    ui.add_sized([width, control_height(ui)], egui::Button::new(label.into()))
}

/// Square or rectangular glyph button that preserves the themed minimum control height.
pub fn icon_button(ui: &mut Ui, glyph: &str, size: Vec2, tooltip: &str) -> Response {
    let button_size = egui::vec2(size.x, size.y.max(ui.spacing().interact_size.y));
    ui.add_sized(
        button_size,
        egui::Button::new(RichText::new(glyph).size(size.y * 0.55)).frame(true),
    )
    .on_hover_text(tooltip)
}

/// Toggle variant of [`icon_button`].
pub fn icon_toggle_button(
    ui: &mut Ui,
    glyph: &str,
    selected: bool,
    size: Vec2,
    tooltip: &str,
) -> Response {
    let button_size = egui::vec2(size.x, size.y.max(ui.spacing().interact_size.y));
    ui.add_sized(
        button_size,
        egui::Button::new(RichText::new(glyph).size(size.y * 0.55)).selected(selected),
    )
    .on_hover_text(tooltip)
}

pub fn icon_button_enabled(
    ui: &mut Ui,
    enabled: bool,
    glyph: &str,
    size: Vec2,
    tooltip: &str,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| icon_button(ui, glyph, size, tooltip))
        .inner
}

pub fn icon_toggle_button_enabled(
    ui: &mut Ui,
    enabled: bool,
    glyph: &str,
    selected: bool,
    size: Vec2,
    tooltip: &str,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        icon_toggle_button(ui, glyph, selected, size, tooltip)
    })
    .inner
}

pub fn primary_button(ui: &mut Ui, label: impl Into<egui::WidgetText>, width: f32) -> Response {
    ui.add(PrimaryButton::new(label).width(width))
}

pub fn primary_action_button(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> Response {
    ui.add(PrimaryButton::new(label))
}

pub fn secondary_button(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> Response {
    ui.add(
        egui::Button::new(label.into())
            .corner_radius(CARD_RADIUS)
            .min_size(egui::vec2(0.0, control_height(ui))),
    )
}

pub fn secondary_button_enabled(
    ui: &mut Ui,
    enabled: bool,
    label: impl Into<egui::WidgetText>,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| secondary_button(ui, label))
        .inner
}

pub fn destructive_button(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> Response {
    let color = ui.visuals().error_fg_color;
    ui.add(
        egui::Button::new(label.into().color(color))
            .corner_radius(CARD_RADIUS)
            .min_size(egui::vec2(0.0, control_height(ui))),
    )
}

pub fn toggle_button(ui: &mut Ui, label: impl Into<egui::WidgetText>, selected: bool) -> Response {
    ui.add(
        egui::Button::new(label.into())
            .selected(selected)
            .frame(true)
            .corner_radius(CARD_RADIUS),
    )
}

pub fn navigation_row(
    ui: &mut Ui,
    label: impl Into<egui::WidgetText>,
    selected: bool,
    sense: egui::Sense,
) -> Response {
    ui.add_sized(
        [ui.available_width().max(1.0), control_height(ui)],
        egui::Button::selectable(selected, ())
            .left_text(label)
            .truncate()
            .sense(sense),
    )
}

pub fn action_row<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().interact_size.y = control_height(ui);
        ui.spacing_mut().item_spacing = egui::vec2(SPACE_SM, SPACE_SM);
        add_contents(ui)
    })
}

pub fn floating_action_rect(bounds: egui::Rect) -> egui::Rect {
    let size = Vec2::splat(FLOATING_ACTION_EDGE);
    let inset = Vec2::splat(FLOATING_ACTION_MARGIN);
    egui::Rect::from_min_size(bounds.right_bottom() - inset - size, size)
}

pub fn floating_action_button(
    ui: &mut Ui,
    rect: egui::Rect,
    glyph: &str,
    tooltip: &str,
) -> Response {
    let active = &ui.visuals().widgets.active;
    let fill = active.weak_bg_fill;
    let stroke = active.bg_stroke;
    let corner_radius = active.corner_radius;
    let icon_color = active.fg_stroke.color;
    ui.put(
        rect,
        egui::Button::new(
            RichText::new(glyph)
                .size(FLOATING_ACTION_EDGE * 0.42)
                .color(icon_color),
        )
        .min_size(rect.size())
        .corner_radius(corner_radius)
        .fill(fill)
        .stroke(stroke),
    )
    .on_hover_text(tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::CONTROL_HEIGHT;

    #[test]
    fn disabled_interaction_state_overrides_other_flags() {
        assert_eq!(
            interaction_visual_state(false, true, true, true, true),
            InteractionVisualState::Disabled
        );
    }

    #[test]
    fn active_selected_and_focus_states_have_stable_priority() {
        assert_eq!(
            interaction_visual_state(true, true, true, true, true),
            InteractionVisualState::Active
        );
        assert_eq!(
            interaction_visual_state(true, true, false, true, true),
            InteractionVisualState::Selected
        );
        assert_eq!(
            interaction_visual_state(true, false, false, false, true),
            InteractionVisualState::Focused
        );
        assert_eq!(
            interaction_visual_state(true, false, false, true, false),
            InteractionVisualState::Hovered
        );
    }

    #[test]
    fn segmented_buttons_honor_their_assigned_width() {
        egui::__run_test_ui(|ui| {
            let width = 42.0;
            let response = segmented_button(ui, "Long segment label", false, width);
            assert_eq!(response.rect.width(), width);
            assert_eq!(response.rect.height(), CONTROL_HEIGHT);
        });
    }

    #[test]
    fn full_width_buttons_use_standard_control_height() {
        egui::__run_test_ui(|ui| {
            ui.set_width(280.0);
            let response = full_width_button(ui, "Continue");
            assert_eq!(response.rect.height(), CONTROL_HEIGHT);
            assert!((response.rect.width() - 280.0).abs() < 0.001);
        });
    }

    #[test]
    fn icon_buttons_preserve_requested_width_and_themed_minimum_height() {
        egui::__run_test_ui(|ui| {
            ui.spacing_mut().interact_size.y = CONTROL_HEIGHT;
            let size = egui::vec2(26.0, 20.0);
            let response = icon_button(ui, "×", size, "Close");
            assert_eq!(response.rect.width(), size.x);
            assert_eq!(response.rect.height(), CONTROL_HEIGHT);
        });
    }

    #[test]
    fn floating_actions_use_platform_size_and_standard_inset() {
        let bounds = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 400.0));
        let rect = floating_action_rect(bounds);
        assert_eq!(rect.size(), egui::Vec2::splat(FLOATING_ACTION_EDGE));
        assert_eq!(
            rect.right_bottom(),
            bounds.right_bottom() - egui::Vec2::splat(FLOATING_ACTION_MARGIN)
        );
    }
}
