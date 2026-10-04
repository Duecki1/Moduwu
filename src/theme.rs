use egui::{self, Color32, Margin, Stroke};

use crate::metrics::{Metrics, CARD_RADIUS, METRICS, SPACE_LG, SPACE_SM};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub accent: Color32,
    pub accent_bright: Color32,
    pub hyperlink: Color32,
    pub border: Color32,
    pub panel: Color32,
    pub window: Color32,
    pub faint: Color32,
    pub extreme: Color32,
    pub inactive: Color32,
    pub inactive_stroke: Color32,
    pub hovered: Color32,
    pub hovered_stroke: Color32,
    pub open: Color32,
    pub open_stroke: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub mode: ThemeMode,
    pub palette: Palette,
}

impl Theme {
    pub const fn new(mode: ThemeMode, palette: Palette) -> Self {
        Self { mode, palette }
    }

    pub const fn dark(palette: Palette) -> Self {
        Self::new(ThemeMode::Dark, palette)
    }

    pub const fn light(palette: Palette) -> Self {
        Self::new(ThemeMode::Light, palette)
    }

    /// Applies the theme with the build platform's [`METRICS`].
    pub fn apply(self, ctx: &egui::Context) {
        self.apply_with_metrics(ctx, METRICS);
    }

    /// Applies the theme with `metrics`, which Moduwu controls then use in
    /// place of the build platform's. Galleries use this to preview another
    /// platform's layout.
    pub fn apply_with_metrics(self, ctx: &egui::Context, metrics: Metrics) {
        metrics.install(ctx);
        let palette = self.palette;
        let egui_theme = if self.mode.is_dark() {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };

        let mut visuals = if self.mode.is_dark() {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.panel_fill = palette.panel;
        visuals.window_fill = palette.window;
        visuals.faint_bg_color = palette.faint;
        visuals.extreme_bg_color = palette.extreme;
        visuals.code_bg_color = palette.faint;
        visuals.selection.bg_fill = palette.accent;
        let active_text = if self.mode.is_dark() {
            Color32::WHITE
        } else {
            Color32::from_rgb(30, 32, 37)
        };
        visuals.selection.stroke = Stroke::new(1.0, active_text);
        visuals.hyperlink_color = palette.hyperlink;
        visuals.window_stroke = Stroke::new(1.0, palette.border);
        visuals.window_corner_radius = 10.0.into();
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(if self.mode.is_dark() { 72 } else { 30 }),
        };
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: Color32::from_black_alpha(if self.mode.is_dark() { 82 } else { 34 }),
        };
        visuals.menu_corner_radius = CARD_RADIUS.into();

        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
        visuals.widgets.noninteractive.corner_radius = CARD_RADIUS.into();
        visuals.widgets.inactive.bg_fill = palette.inactive;
        visuals.widgets.inactive.weak_bg_fill = palette.inactive;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.inactive_stroke);
        visuals.widgets.inactive.corner_radius = CARD_RADIUS.into();
        visuals.widgets.hovered.bg_fill = palette.hovered;
        visuals.widgets.hovered.weak_bg_fill = palette.hovered;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.hovered_stroke);
        visuals.widgets.hovered.corner_radius = CARD_RADIUS.into();
        visuals.widgets.active.bg_fill = palette.accent;
        visuals.widgets.active.weak_bg_fill = palette.accent;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent_bright);
        visuals.widgets.active.fg_stroke.color = active_text;
        visuals.widgets.active.corner_radius = CARD_RADIUS.into();
        visuals.widgets.open.bg_fill = palette.open;
        visuals.widgets.open.weak_bg_fill = palette.open;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette.open_stroke);
        visuals.widgets.open.corner_radius = CARD_RADIUS.into();
        visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

        let mut style = (*ctx.style_of(egui_theme)).clone();
        style.visuals = visuals;
        #[cfg(all(target_os = "android", debug_assertions))]
        {
            style.debug.warn_if_rect_changes_id = false;
            style.debug.show_unaligned = false;
        }
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(20.0));
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
        style.spacing.slider_width = 220.0;
        style.spacing.item_spacing = egui::vec2(SPACE_SM, SPACE_SM);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = metrics.control_height;
        style.spacing.window_margin = Margin::same(metrics.window_margin);
        style.spacing.menu_margin = Margin::same(SPACE_SM as i8);
        style.spacing.indent = SPACE_LG;
        ctx.set_style_of(egui_theme, style);
        ctx.set_theme(egui_theme);
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: Palette = Palette {
        accent: Color32::from_rgb(79, 132, 185),
        accent_bright: Color32::from_rgb(126, 170, 213),
        hyperlink: Color32::from_rgb(111, 162, 210),
        border: Color32::from_rgb(48, 52, 60),
        panel: Color32::from_rgb(24, 26, 31),
        window: Color32::from_rgb(18, 20, 24),
        faint: Color32::from_rgb(31, 34, 40),
        extreme: Color32::from_rgb(12, 14, 17),
        inactive: Color32::from_rgb(34, 37, 43),
        inactive_stroke: Color32::from_rgb(58, 63, 72),
        hovered: Color32::from_rgb(43, 47, 55),
        hovered_stroke: Color32::from_rgb(79, 86, 98),
        open: Color32::from_rgb(39, 43, 50),
        open_stroke: Color32::from_rgb(69, 76, 88),
    };

    #[test]
    fn applying_theme_updates_global_style_metrics() {
        let ctx = egui::Context::default();
        Theme::dark(DARK).apply(&ctx);
        let style = ctx.style_of(ctx.theme());
        assert_eq!(style.spacing.interact_size.y, METRICS.control_height);
        assert_eq!(
            style.spacing.window_margin,
            Margin::same(METRICS.window_margin)
        );
        assert_eq!(style.visuals.selection.bg_fill, DARK.accent);
    }

    #[test]
    fn explicit_metrics_size_controls_for_another_platform() {
        let ctx = egui::Context::default();
        let other = Metrics::for_platform(!METRICS.touch_layout);
        Theme::dark(DARK).apply_with_metrics(&ctx, other);
        let style = ctx.style_of(ctx.theme());
        assert_eq!(style.spacing.interact_size.y, other.control_height);
        assert_eq!(Metrics::of(&ctx), other);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = crate::secondary_button(ui, "Button");
            assert_eq!(response.rect.height(), other.control_height);
        });
    }
}
