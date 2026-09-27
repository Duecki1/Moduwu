/// Shared layout and control metrics for the design system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub control_height: f32,
    pub toolbar_height: f32,
    pub toolbar_icon_edge: f32,
    pub tool_rail_icon_edge: f32,
    pub space_xxs: f32,
    pub space_xs: f32,
    pub space_sm: f32,
    pub space_md: f32,
    pub space_lg: f32,
    pub card_gap: f32,
    pub content_margin: i8,
    pub card_radius: f32,
    pub help_button_edge: f32,
    pub panel_title_height: f32,
    pub panel_title_text_size: f32,
    pub floating_action_edge: f32,
    pub floating_action_margin: f32,
    pub window_margin: i8,
}

pub const DESKTOP_CONTROL_HEIGHT: f32 = 32.0;
pub const ANDROID_CONTROL_HEIGHT: f32 = 40.0;

impl Metrics {
    pub const DESKTOP: Self = Self::for_platform(false);
    pub const ANDROID: Self = Self::for_platform(true);

    pub const fn for_platform(android: bool) -> Self {
        let control_height = if android {
            ANDROID_CONTROL_HEIGHT
        } else {
            DESKTOP_CONTROL_HEIGHT
        };
        Self {
            control_height,
            toolbar_height: control_height,
            toolbar_icon_edge: control_height,
            tool_rail_icon_edge: 40.0,
            space_xxs: 2.0,
            space_xs: 4.0,
            space_sm: 8.0,
            space_md: 12.0,
            space_lg: 16.0,
            card_gap: 8.0,
            content_margin: 12,
            card_radius: 8.0,
            help_button_edge: if android { control_height } else { 28.0 },
            panel_title_height: 40.0,
            panel_title_text_size: 16.0,
            floating_action_edge: if android { 52.0 } else { 46.0 },
            floating_action_margin: 12.0,
            window_margin: if android { 16 } else { 12 },
        }
    }
}

pub const METRICS: Metrics = Metrics::for_platform(cfg!(target_os = "android"));
pub const CONTROL_HEIGHT: f32 = METRICS.control_height;
pub const TOOLBAR_HEIGHT: f32 = METRICS.toolbar_height;
pub const TOOLBAR_ICON_EDGE: f32 = METRICS.toolbar_icon_edge;
pub const TOOL_RAIL_ICON_EDGE: f32 = METRICS.tool_rail_icon_edge;
pub const SPACE_XXS: f32 = METRICS.space_xxs;
pub const SPACE_XS: f32 = METRICS.space_xs;
pub const SPACE_SM: f32 = METRICS.space_sm;
pub const SPACE_MD: f32 = METRICS.space_md;
pub const SPACE_LG: f32 = METRICS.space_lg;
pub const CARD_GAP: f32 = METRICS.card_gap;
pub const CONTENT_MARGIN: i8 = METRICS.content_margin;
pub const CARD_RADIUS: f32 = METRICS.card_radius;
pub const HELP_BUTTON_EDGE: f32 = METRICS.help_button_edge;
pub const PANEL_TITLE_HEIGHT: f32 = METRICS.panel_title_height;
pub const PANEL_TITLE_TEXT_SIZE: f32 = METRICS.panel_title_text_size;
pub const FLOATING_ACTION_EDGE: f32 = METRICS.floating_action_edge;
pub const FLOATING_ACTION_MARGIN: f32 = METRICS.floating_action_margin;
pub const WINDOW_MARGIN: i8 = METRICS.window_margin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_metrics_preserve_touch_targets() {
        assert_eq!(Metrics::ANDROID.control_height, ANDROID_CONTROL_HEIGHT);
        assert_eq!(Metrics::DESKTOP.control_height, DESKTOP_CONTROL_HEIGHT);
        const {
            assert!(Metrics::ANDROID.floating_action_edge > ANDROID_CONTROL_HEIGHT);
            assert!(Metrics::DESKTOP.floating_action_edge > DESKTOP_CONTROL_HEIGHT);
        }
    }
}
