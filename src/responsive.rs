use egui::{Ui, Vec2};

use crate::metrics::{Metrics, CARD_GAP, CONTENT_MARGIN, SPACE_SM};

const COMPACT_PORTRAIT_CARD_GAP: f32 = SPACE_SM;
const COMPACT_PORTRAIT_CONTENT_MARGIN: i8 = SPACE_SM as i8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponsiveWidth {
    Compact,
    Medium,
    Wide,
}

impl ResponsiveWidth {
    pub const COMPACT_MAX_WIDTH: f32 = 520.0;
    pub const WIDE_MIN_WIDTH: f32 = 820.0;

    pub fn from_width(width: f32) -> Self {
        if width < Self::COMPACT_MAX_WIDTH {
            Self::Compact
        } else if width < Self::WIDE_MIN_WIDTH {
            Self::Medium
        } else {
            Self::Wide
        }
    }

    pub const fn is_compact(self) -> bool {
        matches!(self, Self::Compact)
    }

    pub const fn is_wide(self) -> bool {
        matches!(self, Self::Wide)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenLayout {
    Horizontal,
    Vertical,
}

impl ScreenLayout {
    pub const MIN_HORIZONTAL_SIDEBAR_WIDTH: f32 = 320.0;
    pub const MAX_HORIZONTAL_SIDEBAR_WIDTH: f32 = ResponsiveWidth::COMPACT_MAX_WIDTH;
    pub const MIN_VERTICAL_SIDEBAR_HEIGHT: f32 = 240.0;

    pub fn from_size(size: Vec2) -> Self {
        if size.x >= size.y {
            Self::Horizontal
        } else {
            Self::Vertical
        }
    }

    pub fn sidebar_default_size(self, viewport: Vec2) -> f32 {
        self.sidebar_default_size_for_platform(viewport, cfg!(target_os = "android"))
    }

    pub fn sidebar_default_size_for_platform(self, viewport: Vec2, android: bool) -> f32 {
        match self {
            Self::Horizontal => {
                (viewport.x * 0.28).clamp(Self::MIN_HORIZONTAL_SIDEBAR_WIDTH, 460.0)
            }
            Self::Vertical if android => {
                (viewport.y * 0.42).clamp(Self::MIN_VERTICAL_SIDEBAR_HEIGHT, 480.0)
            }
            Self::Vertical => (viewport.y * 0.40).clamp(Self::MIN_VERTICAL_SIDEBAR_HEIGHT, 480.0),
        }
    }
}

/// Returns a user-resized panel width when egui has persisted one, otherwise the default.
///
/// Keeping this separate from content-driven panel sizing prevents newly revealed controls
/// from silently replacing an explicit resize choice.
pub fn persisted_panel_width(
    ctx: &egui::Context,
    panel_id: egui::Id,
    default_width: f32,
    min_width: f32,
    max_width: f32,
) -> f32 {
    ctx.data_mut(|data| {
        data.get_persisted::<f32>(panel_id.with("user-width"))
            .or_else(|| {
                data.get_persisted::<egui::PanelState>(panel_id)
                    .map(|state| state.size().x)
            })
            .unwrap_or(default_width)
            .clamp(min_width, max_width)
    })
}

pub fn compact_portrait_for_platform(viewport: Vec2, android: bool) -> bool {
    android && viewport.x < viewport.y
}

pub fn is_compact_portrait(ui: &Ui) -> bool {
    compact_portrait_for_platform(
        ui.ctx().content_rect().size(),
        Metrics::of(ui.ctx()).touch_layout,
    )
}

pub fn content_margin(ui: &Ui) -> i8 {
    if is_compact_portrait(ui) {
        COMPACT_PORTRAIT_CONTENT_MARGIN
    } else {
        CONTENT_MARGIN
    }
}

pub fn card_gap(ui: &mut Ui) {
    let gap = if is_compact_portrait(ui) {
        COMPACT_PORTRAIT_CARD_GAP
    } else {
        CARD_GAP
    };
    let explicit_space = (gap - ui.spacing().item_spacing.y).max(0.0);
    ui.add_space(explicit_space);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsive_width_uses_shared_breakpoints() {
        assert_eq!(ResponsiveWidth::from_width(319.0), ResponsiveWidth::Compact);
        assert_eq!(ResponsiveWidth::from_width(519.0), ResponsiveWidth::Compact);
        assert_eq!(ResponsiveWidth::from_width(520.0), ResponsiveWidth::Medium);
        assert_eq!(ResponsiveWidth::from_width(819.0), ResponsiveWidth::Medium);
        assert_eq!(ResponsiveWidth::from_width(820.0), ResponsiveWidth::Wide);
        assert_eq!(ResponsiveWidth::from_width(1200.0), ResponsiveWidth::Wide);
    }

    #[test]
    fn compact_density_is_limited_to_android_portrait() {
        let portrait = egui::vec2(411.0, 891.0);
        let landscape = egui::vec2(891.0, 411.0);
        assert!(compact_portrait_for_platform(portrait, true));
        assert!(!compact_portrait_for_platform(landscape, true));
        assert!(!compact_portrait_for_platform(portrait, false));
    }

    #[test]
    fn android_portrait_sidebar_preserves_preview_room() {
        let viewport = egui::vec2(411.0, 891.0);
        let size = ScreenLayout::Vertical.sidebar_default_size_for_platform(viewport, true);
        assert_eq!(size, viewport.y * 0.42);
        assert!(size > ScreenLayout::Vertical.sidebar_default_size_for_platform(viewport, false));
        assert!(size < viewport.y * 0.5);
    }

    #[test]
    fn landscape_sidebar_size_is_platform_neutral() {
        let viewport = egui::vec2(891.0, 411.0);
        assert_eq!(
            ScreenLayout::Horizontal.sidebar_default_size_for_platform(viewport, true),
            ScreenLayout::Horizontal.sidebar_default_size_for_platform(viewport, false)
        );
    }
}
