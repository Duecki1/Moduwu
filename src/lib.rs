//! Reusable egui design-system primitives shared across applications.
//!
//! The crate intentionally owns only presentation concerns: palette-driven
//! theme application, layout metrics, frames/cards, common buttons and form
//! controls, popup helpers, and responsive layout utilities, including
//! scrolling a newly revealed widget into view. Applications are
//! expected to keep their own theme selection, persistence, domain widgets,
//! and business logic outside this crate.

mod angle_dial;
mod buttons;
mod dialogs;
mod forms;
mod frames;
mod menus;
mod metrics;
mod number_field;
mod presets;
mod responsive;
mod scroll_into_view;
mod slider;
mod theme;

pub use angle_dial::{AngleDial, SNAP_DEGREES};
pub use buttons::{
    action_row, destructive_button, floating_action_button, floating_action_rect,
    full_width_button, icon_button, icon_button_enabled, icon_toggle_button,
    icon_toggle_button_enabled, interaction_visual_state, interaction_visuals,
    interaction_visuals_for_flags, navigation_row, primary_action_button, primary_button,
    secondary_button, secondary_button_enabled, segmented_button, tab_button, toggle,
    toggle_button, toolbar_button, InteractionVisualState, InteractionVisuals, PrimaryButton,
};
pub use dialogs::{
    dialog_button_row, dialog_confirmation_buttons, dialog_keyboard_action, dialog_text_field,
    dialog_window, request_initial_focus, DialogAction, DialogKeyboard, DialogWindow,
    DIALOG_MARGIN, DIALOG_TEXT_FIELD_WIDTH, DIALOG_WIDTH_DEFAULT, DIALOG_WIDTH_FORM,
    DIALOG_WIDTH_LARGE, DIALOG_WIDTH_NARROW, DIALOG_WIDTH_WIDE,
};
pub use forms::{
    checkbox_with_help, combo_box, form_combo, form_combo_with_help, form_row, form_row_with_help,
    heading_with_help, property_row, responsive_combo_box, singleline_text_edit, strong_with_help,
    toggle_with_help,
};
pub use frames::{
    card_frame, card_header, content_card, panel_frame, panel_title, prepare_toolbar,
    progress_card_header, section_card, section_card_with_help, section_separator,
    tool_rail_icon_size, toolbar_frame, toolbar_icon_size, toolbar_row, toolbar_title,
    workspace_frame, Card,
};
pub use menus::{
    context_menu, context_menu_item, destructive_menu_item, dropdown_menu, dropdown_submenu,
    menu_item, overflow_menu,
};
pub use metrics::{
    Metrics, ANDROID_CONTROL_HEIGHT, CARD_GAP, CARD_RADIUS, CONTENT_MARGIN, CONTROL_HEIGHT,
    DESKTOP_CONTROL_HEIGHT, FLOATING_ACTION_EDGE, FLOATING_ACTION_MARGIN, HELP_BUTTON_EDGE,
    METRICS, PANEL_TITLE_HEIGHT, PANEL_TITLE_TEXT_SIZE, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XS,
    SPACE_XXS, TOOLBAR_HEIGHT, TOOLBAR_ICON_EDGE, TOOL_RAIL_ICON_EDGE, WINDOW_MARGIN,
};
pub use number_field::{arrow_step, step_focused, NumberField};
pub use presets::Design;
pub use responsive::{
    card_gap, compact_portrait_for_platform, content_margin, is_compact_portrait,
    persisted_panel_width, ResponsiveWidth, ScreenLayout,
};
pub use scroll_into_view::{request_scroll_into_view, scroll_into_view_on_draw};
pub use slider::{
    lock_slider_scroll, slider_scroll_locked, Slider, SliderLayout, SliderMetrics, SliderResponse,
};
pub use theme::{Palette, Theme, ThemeMode};
