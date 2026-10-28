//! Every reusable Moduwu control in each theme, with desktop or Android
//! metrics selectable at runtime.
//!
//! Run with `cargo run --example gallery`. Tab moves keyboard focus through
//! the controls; the "Disabled" toggle shows every control's disabled state.
//! With Android metrics, menus open as bottom sheets.

use eframe::egui::{self, Color32, Ui};
use moduwu_design::{
    action_row, checkbox_with_help, chip, content_card, destructive_button,
    dialog_confirmation_actions, dialog_confirmation_buttons, dialog_window, dropdown_menu,
    form_combo, form_row, form_row_with_help, full_width_button, ghost_icon_button,
    ghost_icon_toggle_button, icon_button, icon_toggle_button, menu_checkbox_item, menu_title,
    navigation_row, panel_title, primary_button, secondary_button, section_card,
    section_card_with_help, segmented_button, segmented_control, singleline_text_edit, submenu,
    tab_button, toggle, toggle_button, toggle_row, toggle_with_help, toolbar_button, toolbar_row,
    tree, AngleDial, AnimatedImage, Design, Dialog, DialogAction, DialogKeyboard, DialogPreview,
    DialogSize, MenuItem, Metrics, NumberField, Slider, SliderLayout, SliderScale, TreeRow,
    DIALOG_WIDTH_FORM, DIALOG_WIDTH_PREVIEW,
};

fn main() -> eframe::Result {
    eframe::run_native(
        "Moduwu gallery",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<Gallery>::default())),
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Platform {
    #[default]
    Desktop,
    Android,
}

impl Platform {
    const fn metrics(self) -> Metrics {
        match self {
            Self::Desktop => Metrics::DESKTOP,
            Self::Android => Metrics::ANDROID,
        }
    }
}

struct Gallery {
    design: Design,
    platform: Platform,
    /// The design and platform last applied to the context.
    applied: Option<(Design, Platform)>,
    enabled: bool,
    compact_sliders: bool,
    toggled: bool,
    tab: usize,
    /// The selected row of `FOLDERS`, and whether its first row is expanded.
    folder: usize,
    folders_expanded: bool,
    checked: bool,
    text: String,
    count: u32,
    gamma: f32,
    exposure: f32,
    hue: f32,
    opacity: f32,
    temperature: f32,
    angle: f32,
    choice: usize,
    /// Which `CHOICES` chips are selected.
    chips: [bool; 3],
    dialog_open: bool,
    modal_open: bool,
    preview_dialog_open: bool,
    /// A clip for the preview dialog, decoded on first use.
    clip: Option<AnimatedImage>,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            design: Design::default(),
            platform: Platform::default(),
            applied: None,
            enabled: true,
            compact_sliders: false,
            toggled: false,
            tab: 0,
            folder: 1,
            folders_expanded: true,
            checked: true,
            text: "Editable text".to_owned(),
            count: 12,
            gamma: 2.2,
            exposure: 0.35,
            hue: 210.0,
            opacity: 80.0,
            temperature: 5_500.0,
            angle: -30.0,
            choice: 0,
            chips: [true, false, false],
            dialog_open: false,
            modal_open: false,
            preview_dialog_open: false,
            clip: None,
        }
    }
}

const CHOICES: [&str; 3] = ["Balanced", "Detailed", "Fast"];

/// A tree's rows and their depths; the rows at depth 1 are children of the
/// first.
const FOLDERS: [(&str, usize); 4] = [("Photos", 0), ("2025", 1), ("2026", 1), ("Exports", 0)];

impl eframe::App for Gallery {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl Gallery {
    fn show(&mut self, ui: &mut Ui) {
        if self.applied != Some((self.design, self.platform)) {
            self.design
                .theme()
                .apply_with_metrics(ui.ctx(), self.platform.metrics());
            self.applied = Some((self.design, self.platform));
        }
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            self.settings(ui);
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_enabled_ui(self.enabled, |ui| self.controls(ui));
            });
        });
        self.dialog(ui.ctx());
        self.preview_dialog(ui.ctx());
        self.modal(ui.ctx());
    }

    fn settings(&mut self, ui: &mut Ui) {
        panel_title(ui, "Moduwu gallery");
        ui.horizontal_wrapped(|ui| {
            for design in Design::ALL {
                if toggle_button(ui, design.label(), self.design == design).clicked() {
                    self.design = design;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (platform, label) in [
                (Platform::Desktop, "Desktop metrics"),
                (Platform::Android, "Android metrics"),
            ] {
                if segmented_button(ui, label, self.platform == platform, 150.0).clicked() {
                    self.platform = platform;
                }
            }
            ui.checkbox(&mut self.enabled, "Enabled");
            ui.checkbox(&mut self.compact_sliders, "Compact sliders");
        });
    }

    fn controls(&mut self, ui: &mut Ui) {
        section_card(ui, "Buttons", |ui| {
            action_row(ui, |ui| {
                primary_button(ui, "Primary", 120.0);
                secondary_button(ui, "Secondary");
                destructive_button(ui, "Delete");
                if toggle_button(ui, "Toggle", self.toggled).clicked() {
                    self.toggled = !self.toggled;
                }
            });
            ui.horizontal(|ui| {
                for (index, label) in ["Edit", "Masks", "Export"].into_iter().enumerate() {
                    if tab_button(ui, label, self.tab == index, 96.0).clicked() {
                        self.tab = index;
                    }
                }
            });
            let icon = egui::vec2(32.0, 32.0);
            ui.horizontal(|ui| {
                icon_button(ui, "+", icon, "Add");
                if icon_toggle_button(ui, "*", self.toggled, icon, "Toggle").clicked() {
                    self.toggled = !self.toggled;
                }
                toolbar_button(ui, "Toolbar", 96.0);
            });
            full_width_button(ui, "Full width");
        });

        section_card(ui, "Choices", |ui| {
            let modes = [(0, "Library"), (1, "Develop"), (2, "Settings")];
            segmented_control(ui, &mut self.tab, &modes, ui.available_width().min(300.0));
            action_row(ui, |ui| {
                for (selected, label) in self.chips.iter_mut().zip(CHOICES) {
                    if chip(ui, label, *selected).clicked() {
                        *selected = !*selected;
                    }
                }
            });
            let icon = egui::vec2(32.0, 32.0);
            ui.horizontal(|ui| {
                ghost_icon_button(ui, "+", icon, "Add");
                if ghost_icon_toggle_button(ui, "*", self.toggled, icon, "Toggle").clicked() {
                    self.toggled = !self.toggled;
                }
                toggle(ui, &mut self.toggled, "Switch");
            });
        });

        section_card_with_help(
            ui,
            "Forms",
            "Rows stack below the compact breakpoint.",
            |ui| {
                checkbox_with_help(ui, &mut self.checked, "Checkbox", "A checkbox with help.");
                toggle_with_help(ui, &mut self.checked, "Toggle", "A toggle with help.");
                toggle_row(ui, &mut self.toggled, "Toggle row");
                form_combo(
                    ui,
                    "Choice",
                    "gallery-choice",
                    CHOICES[self.choice],
                    160.0,
                    |ui| {
                        for (index, choice) in CHOICES.into_iter().enumerate() {
                            ui.selectable_value(&mut self.choice, index, choice);
                        }
                    },
                );
                form_row(ui, "Text", 160.0, |ui, width| {
                    ui.add_sized(
                        [width, ui.spacing().interact_size.y],
                        singleline_text_edit(&mut self.text),
                    );
                });
                form_row_with_help(
                    ui,
                    "Count",
                    120.0,
                    "Integer field with a suffix.",
                    |ui, _| {
                        ui.add(NumberField::new(&mut self.count, 0..=64).suffix(" px"));
                    },
                );
                form_row(ui, "Gamma", 120.0, |ui, _| {
                    ui.add(
                        NumberField::new(&mut self.gamma, 0.5..=4.0)
                            .speed(0.01)
                            .decimals(2)
                            .commit_on_finish(true),
                    );
                });
            },
        );

        section_card(ui, "Sliders", |ui| {
            let layout = if self.compact_sliders {
                SliderLayout::Compact
            } else {
                SliderLayout::Stacked
            };
            Slider::new("Exposure", &mut self.exposure, -5.0..=5.0)
                .decimals(2)
                .step(0.05)
                .layout(layout)
                .show(ui);
            Slider::new("Opacity", &mut self.opacity, 0.0..=100.0)
                .accent(Color32::from_rgb(232, 129, 74))
                .reset_to(100.0)
                .layout(layout)
                .show(ui);
            Slider::new("Hue", &mut self.hue, 0.0..=360.0)
                .gradient(|t| egui::ecolor::Hsva::new(t, 0.9, 0.92, 1.0).into())
                .layout(layout)
                .show(ui);
            Slider::new("Temperature", &mut self.temperature, 2_000.0..=50_000.0)
                .scale(SliderScale::Reciprocal)
                .step(10.0)
                .reset_to(5_500.0)
                .layout(layout)
                .show(ui);
            AngleDial::new("Angle", &mut self.angle).show(ui);
        });

        content_card(ui, |ui| {
            toolbar_row(ui, |ui| {
                ui.strong("Navigation and menus");
                let menu = secondary_button(ui, "Menu");
                dropdown_menu(&menu, |ui| {
                    menu_title(ui, "Menu");
                    let _ = ui.add(MenuItem::new("Enabled item").icon("📋"));
                    let _ = ui.add_enabled(false, MenuItem::new("Disabled item").icon("📋"));
                    menu_checkbox_item(ui, &mut self.checked, "Checkable item");
                    submenu(ui, MenuItem::new("Submenu").icon("🗀"), |ui| {
                        let _ = ui.add(MenuItem::new("Nested item"));
                    });
                    ui.separator();
                    if ui
                        .add(MenuItem::new("Delete").icon("🗑").destructive())
                        .clicked()
                    {
                        ui.close();
                    }
                });
                if secondary_button(ui, "Dialog").clicked() {
                    self.dialog_open = true;
                }
                if secondary_button(ui, "Modal dialog").clicked() {
                    self.modal_open = true;
                }
                if secondary_button(ui, "Preview dialog").clicked() {
                    self.preview_dialog_open = true;
                }
            });
            for (index, label) in ["Library", "Develop"].into_iter().enumerate() {
                if navigation_row(ui, label, self.tab == index, egui::Sense::click()).clicked() {
                    self.tab = index;
                }
            }
            ui.separator();
            tree(ui, |ui| {
                for (index, (label, depth)) in FOLDERS.into_iter().enumerate() {
                    if depth > 0 && !self.folders_expanded {
                        continue;
                    }
                    let row = TreeRow::new(index, label)
                        .depth(depth)
                        .expanded((index == 0).then_some(self.folders_expanded))
                        .icon("🗀")
                        .selected(self.folder == index)
                        .show(ui);
                    if row.toggled() {
                        self.folders_expanded = !self.folders_expanded;
                    }
                    if row.row.clicked() {
                        self.folder = index;
                    }
                }
            });
        });
    }

    fn dialog(&mut self, ctx: &egui::Context) {
        if !self.dialog_open {
            return;
        }
        dialog_window("Dialog", ctx, DIALOG_WIDTH_FORM).show(ctx, |ui| {
            ui.label("Enter confirms and Escape cancels.");
            let action = dialog_confirmation_buttons(
                ui,
                "Cancel",
                "Confirm",
                true,
                false,
                DialogKeyboard::CONFIRM_ON_ENTER,
            );
            if action != DialogAction::None {
                self.dialog_open = false;
            }
        });
    }

    /// A modal dialog; a bottom sheet in narrow windows and Android portrait.
    fn modal(&mut self, ctx: &egui::Context) {
        if !self.modal_open {
            return;
        }
        let mut action = DialogAction::None;
        let response = Dialog::new("gallery-modal", "Modal dialog")
            .size(DialogSize::Wide)
            .close_button(true)
            .dismiss_on_backdrop(true)
            .show_with_footer(
                ctx,
                |ui| {
                    ui.label("Enter confirms, Escape cancels and a backdrop click dismisses.");
                    toggle_with_help(ui, &mut self.checked, "Switch", "A switch with help.");
                },
                |ui| {
                    action = dialog_confirmation_actions(
                        ui,
                        "Cancel",
                        "Confirm",
                        true,
                        false,
                        DialogKeyboard::CONFIRM_ON_ENTER,
                    );
                },
            );
        if response.dismissed || action != DialogAction::None {
            self.modal_open = false;
        }
    }

    /// The preview stays in view beside or above the controls, which scroll.
    /// It plays a clip, as a help dialog would.
    fn preview_dialog(&mut self, ctx: &egui::Context) {
        if !self.preview_dialog_open {
            return;
        }
        let clip = self.clip.get_or_insert_with(|| {
            AnimatedImage::from_bytes(ctx, "gallery-clip", &gallery_clip())
                .expect("the generated clip is a valid GIF")
        });
        let mut action = DialogAction::None;
        dialog_window("Preview dialog", ctx, DIALOG_WIDTH_PREVIEW).show_with_preview(
            ctx,
            DialogPreview {
                label: "Help clip",
                aspect: clip.aspect(),
            },
            |ui, rect| {
                clip.show(ui, rect, "Help clip");
            },
            |ui| {
                Slider::new("Exposure", &mut self.exposure, -1.0..=1.0)
                    .decimals(2)
                    .show(ui);
                for row in 1..=12 {
                    ui.label(format!("Scrolling content {row}"));
                }
            },
            |ui| {
                action = dialog_confirmation_buttons(
                    ui,
                    "Cancel",
                    "Apply",
                    true,
                    false,
                    DialogKeyboard::CLOSE_ONLY,
                );
            },
        );
        if action != DialogAction::None {
            self.preview_dialog_open = false;
        }
    }
}

/// A 1.5 s GIF of a bar sweeping across a 3:2 frame.
fn gallery_clip() -> Vec<u8> {
    const FRAMES: u32 = 12;
    let (width, height) = (96, 64);
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
    for frame in 0..FRAMES {
        let bar = frame * width / FRAMES;
        let pixels = image::RgbaImage::from_fn(width, height, |x, _| {
            if (bar..bar + width / FRAMES).contains(&x) {
                image::Rgba([240, 240, 240, 255])
            } else {
                image::Rgba([40, 60, 90, 255])
            }
        });
        encoder
            .encode_frame(image::Frame::from_parts(
                pixels,
                0,
                0,
                image::Delay::from_numer_denom_ms(125, 1),
            ))
            .expect("encoding to memory cannot fail");
    }
    drop(encoder);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, gallery: &mut Gallery, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(420.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| gallery.show(ui));
    }

    #[test]
    fn every_theme_and_platform_renders_enabled_disabled_and_focused() {
        let tab = egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        for design in Design::ALL {
            for platform in [Platform::Desktop, Platform::Android] {
                for (enabled, compact_sliders) in [(true, false), (false, true)] {
                    let ctx = egui::Context::default();
                    let mut gallery = Gallery {
                        design,
                        platform,
                        enabled,
                        compact_sliders,
                        dialog_open: true,
                        modal_open: true,
                        preview_dialog_open: true,
                        ..Gallery::default()
                    };
                    frame(&ctx, &mut gallery, Vec::new());
                    assert_eq!(Metrics::of(&ctx), platform.metrics());
                    assert_eq!(
                        ctx.style_of(ctx.theme()).spacing.interact_size.y,
                        platform.metrics().control_height
                    );
                    for _ in 0..3 {
                        frame(&ctx, &mut gallery, vec![tab.clone()]);
                    }
                    assert!(ctx.memory(|memory| memory.focused().is_some()));
                }
            }
        }
    }
}
