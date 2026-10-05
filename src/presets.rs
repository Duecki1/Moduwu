//! Built-in themes shared by applications using this design system.
use egui::Color32;

use crate::{Palette, Theme};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Design {
    #[default]
    ObsidianBlue,
    ObsidianRed,
    PlainGreyDark,
    Porcelain,
    DaylightBlue,
    PlainGreyLight,
}

impl Design {
    pub const ALL: [Self; 6] = [
        Self::ObsidianBlue,
        Self::ObsidianRed,
        Self::PlainGreyDark,
        Self::Porcelain,
        Self::DaylightBlue,
        Self::PlainGreyLight,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::ObsidianBlue => "Obsidian Blue · Dark",
            Self::ObsidianRed => "Obsidian Red · Dark",
            Self::PlainGreyDark => "Plain Grey · Dark",
            Self::Porcelain => "Porcelain · Light",
            Self::DaylightBlue => "Daylight · Light",
            Self::PlainGreyLight => "Plain Grey · Light",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::ObsidianBlue => "Deep neutral surfaces with a restrained blue accent.",
            Self::ObsidianRed => "Warm near-black surfaces with a restrained ruby accent.",
            Self::Porcelain => "Warm paper-like surfaces with a restrained coral accent.",
            Self::DaylightBlue => "Clean cool surfaces with a focused blue accent.",
            Self::PlainGreyDark | Self::PlainGreyLight => {
                "Neutral grey surfaces and accent, with no colour tint."
            }
        }
    }

    pub const fn is_dark(self) -> bool {
        matches!(
            self,
            Self::ObsidianBlue | Self::ObsidianRed | Self::PlainGreyDark
        )
    }

    pub const fn palette(self) -> Palette {
        match self {
            Self::ObsidianBlue => Palette {
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
            },
            Self::ObsidianRed => Palette {
                accent: Color32::from_rgb(166, 70, 86),
                accent_bright: Color32::from_rgb(211, 111, 125),
                hyperlink: Color32::from_rgb(222, 125, 137),
                border: Color32::from_rgb(55, 45, 49),
                panel: Color32::from_rgb(25, 22, 24),
                window: Color32::from_rgb(18, 16, 18),
                faint: Color32::from_rgb(34, 29, 31),
                extreme: Color32::from_rgb(11, 10, 11),
                inactive: Color32::from_rgb(39, 33, 35),
                inactive_stroke: Color32::from_rgb(67, 52, 56),
                hovered: Color32::from_rgb(49, 40, 43),
                hovered_stroke: Color32::from_rgb(99, 69, 76),
                open: Color32::from_rgb(44, 36, 39),
                open_stroke: Color32::from_rgb(88, 61, 67),
            },
            // Accent text is white in dark themes, so the accent stays mid-dark.
            Self::PlainGreyDark => Palette {
                accent: Color32::from_gray(112),
                accent_bright: Color32::from_gray(160),
                hyperlink: Color32::from_gray(190),
                border: Color32::from_gray(52),
                panel: Color32::from_gray(27),
                window: Color32::from_gray(20),
                faint: Color32::from_gray(35),
                extreme: Color32::from_gray(13),
                inactive: Color32::from_gray(38),
                inactive_stroke: Color32::from_gray(64),
                hovered: Color32::from_gray(48),
                hovered_stroke: Color32::from_gray(88),
                open: Color32::from_gray(43),
                open_stroke: Color32::from_gray(77),
            },
            Self::Porcelain => Palette {
                accent: Color32::from_rgb(232, 132, 169),
                accent_bright: Color32::from_rgb(242, 166, 194),
                hyperlink: Color32::from_rgb(173, 36, 92),
                border: Color32::from_rgb(205, 195, 199),
                panel: Color32::from_rgb(247, 243, 244),
                window: Color32::from_rgb(255, 251, 252),
                faint: Color32::from_rgb(241, 235, 237),
                extreme: Color32::from_rgb(225, 216, 219),
                inactive: Color32::from_rgb(237, 229, 232),
                inactive_stroke: Color32::from_rgb(196, 183, 188),
                hovered: Color32::from_rgb(230, 218, 223),
                hovered_stroke: Color32::from_rgb(176, 157, 165),
                open: Color32::from_rgb(226, 213, 218),
                open_stroke: Color32::from_rgb(168, 149, 157),
            },
            Self::DaylightBlue => Palette {
                accent: Color32::from_rgb(116, 170, 242),
                accent_bright: Color32::from_rgb(151, 195, 250),
                hyperlink: Color32::from_rgb(28, 91, 193),
                border: Color32::from_rgb(190, 199, 211),
                panel: Color32::from_rgb(243, 247, 252),
                window: Color32::from_rgb(251, 253, 255),
                faint: Color32::from_rgb(235, 241, 248),
                extreme: Color32::from_rgb(216, 225, 236),
                inactive: Color32::from_rgb(229, 236, 245),
                inactive_stroke: Color32::from_rgb(178, 190, 205),
                hovered: Color32::from_rgb(217, 227, 239),
                hovered_stroke: Color32::from_rgb(151, 170, 193),
                open: Color32::from_rgb(211, 223, 237),
                open_stroke: Color32::from_rgb(143, 163, 187),
            },
            // Accent text is near-black in light themes, so the accent stays mid-light.
            Self::PlainGreyLight => Palette {
                accent: Color32::from_gray(170),
                accent_bright: Color32::from_gray(198),
                hyperlink: Color32::from_gray(40),
                border: Color32::from_gray(196),
                panel: Color32::from_gray(241),
                window: Color32::from_gray(250),
                faint: Color32::from_gray(234),
                extreme: Color32::from_gray(218),
                inactive: Color32::from_gray(230),
                inactive_stroke: Color32::from_gray(184),
                hovered: Color32::from_gray(219),
                hovered_stroke: Color32::from_gray(160),
                open: Color32::from_gray(213),
                open_stroke: Color32::from_gray(152),
            },
        }
    }

    pub const fn theme(self) -> Theme {
        let palette = self.palette();
        if self.is_dark() {
            Theme::dark(palette)
        } else {
            Theme::light(palette)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_apply_their_palette_and_mode() {
        let ctx = egui::Context::default();
        for design in Design::ALL {
            design.theme().apply(&ctx);
            let style = ctx.style_of(ctx.theme());
            let palette = design.palette();
            assert_eq!(style.visuals.dark_mode, design.is_dark());
            assert_eq!(style.visuals.panel_fill, palette.panel);
            assert_eq!(style.visuals.window_fill, palette.window);
            assert_eq!(style.visuals.widgets.active.bg_fill, palette.accent);
        }
    }

    #[test]
    fn plain_grey_presets_are_neutral_in_both_modes() {
        assert!(Design::PlainGreyDark.is_dark());
        assert!(!Design::PlainGreyLight.is_dark());
        for design in [Design::PlainGreyDark, Design::PlainGreyLight] {
            let p = design.palette();
            for color in [
                p.accent,
                p.accent_bright,
                p.hyperlink,
                p.border,
                p.panel,
                p.window,
                p.faint,
                p.extreme,
                p.inactive,
                p.inactive_stroke,
                p.hovered,
                p.hovered_stroke,
                p.open,
                p.open_stroke,
            ] {
                assert!(
                    color.r() == color.g() && color.g() == color.b(),
                    "{design:?} {color:?}"
                );
            }
        }
    }
}
