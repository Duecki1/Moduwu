//! Menus on touch layouts: a sheet that rises from the bottom of the
//! viewport, with tall rows, where a submenu opens as a page with a back row.
//! [`crate::context_menu`] and the other menu functions show menus this way
//! on touch layouts and as popups elsewhere.
//!
//! One sheet is open at a time. As with a popup, the code that opened it adds
//! the menu's contents every frame, and a sheet whose menu is not drawn for a
//! frame closes. While a submenu's page shows, the rest of the menu runs
//! unseen, so that the submenu, which it contains, can add its page.

use egui::{
    self, Align, Align2, Color32, Context, FontId, Id, InnerResponse, Key, Layout, Modifiers,
    Response, RichText, Sense, Stroke, StrokeKind, TextWrapMode, Ui, UiBuilder, UiStackInfo,
    WidgetText,
};

use crate::buttons::ghost_icon_button;
use crate::keyboard::unobscured_content_rect;
use crate::metrics::{control_height, Metrics, SPACE_SM};
use crate::modal_dialog::{backdrop_color, sheet_frame, SHEET_MAX_HEIGHT_FRACTION};
use crate::switch::{paint_switch, track_size};

/// A row's height: a comfortable touch target.
const ROW_HEIGHT: f32 = 48.0;
/// From a row's edges to its icon and to its trailing chevron or switch. A
/// sheet insets its contents by as much, and rows reach out over the inset,
/// so headings, chips and other controls line up with the rows' icons.
const ROW_PADDING: f32 = 12.0;
/// The column at the start of a row that holds its icon.
const ICON_COLUMN: f32 = 40.0;
const ICON_SIZE: f32 = 20.0;
/// Half the height of the chevrons on submenu and back rows.
const CHEVRON_EXTENT: f32 = 5.0;
/// The edge of the check mark on a selected choice.
const CHECK_EDGE: f32 = 16.0;
/// Sheets are no wider than this, so they stay compact on tablets.
const MAX_WIDTH: f32 = 640.0;

const TAG: &str = "moduwu-menu-sheet";

fn sheet_id() -> Id {
    Id::new("moduwu-menu-sheet")
}

fn area_id() -> Id {
    sheet_id().with("area")
}

/// The frame a submenu's page was last shown in.
fn page_shown_id() -> Id {
    sheet_id().with("page-shown")
}

/// The open sheet.
#[derive(Clone, Debug)]
struct Sheet {
    /// The menu it shows: the id of the widget that opened it.
    menu: Id,
    /// The submenus opened in it, outermost first. The last one's page shows.
    pages: Vec<Id>,
    /// The frame the sheet was last shown in.
    shown_frame: u64,
}

/// Marks the `Ui`s a sheet's contents are added to, so that rows lay
/// themselves out for touch and submenus open pages.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tag {
    menu: Id,
    /// The menu or submenu the contents belong to. Submenu ids derive from
    /// it, so they match whether the contents show or run unseen.
    page: Id,
}

/// The sheet `ui`'s contents are added to, if any.
pub(crate) fn tag(ui: &Ui) -> Option<Tag> {
    ui.stack()
        .iter()
        .find_map(|frame| frame.tags().get_downcast::<Tag>(TAG).copied())
}

fn tagged(tag: Tag) -> UiBuilder {
    UiBuilder::new().ui_stack_info(UiStackInfo::default().with_tag_value(TAG, tag))
}

/// The open sheet, unless its menu was not drawn in the previous frame.
fn current(ctx: &Context) -> Option<Sheet> {
    let sheet = ctx.data(|data| data.get_temp::<Sheet>(sheet_id()))?;
    if sheet.shown_frame + 1 < ctx.cumulative_frame_nr() {
        ctx.data_mut(|data| data.remove::<Sheet>(sheet_id()));
        return None;
    }
    Some(sheet)
}

fn store(ctx: &Context, sheet: Sheet) {
    ctx.data_mut(|data| data.insert_temp(sheet_id(), sheet));
    ctx.request_repaint();
}

fn close(ctx: &Context) {
    ctx.data_mut(|data| data.remove::<Sheet>(sheet_id()));
    ctx.request_repaint();
}

/// Opens `menu`'s sheet in place of any other.
pub(crate) fn open(ctx: &Context, menu: Id) {
    store(
        ctx,
        Sheet {
            menu,
            pages: Vec::new(),
            shown_frame: ctx.cumulative_frame_nr(),
        },
    );
}

/// Whether a menu is open as a sheet, which only touch layouts show. The
/// platform layer asks it to learn whether the system back button has
/// something to close.
pub fn menu_sheet_open(ctx: &Context) -> bool {
    current(ctx).is_some()
}

/// Goes back from the open sheet's submenu page, or closes the sheet when it
/// shows its menu, as the system back button does. Returns whether a sheet
/// was open.
pub fn dismiss_menu_sheet(ctx: &Context) -> bool {
    let Some(mut sheet) = current(ctx) else {
        return false;
    };
    if sheet.pages.pop().is_some() {
        store(ctx, sheet);
    } else {
        close(ctx);
    }
    true
}

/// Shows `menu`'s sheet while it is open, with `add_contents` as the menu.
pub(crate) fn show<R>(
    ctx: &Context,
    menu: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let mut sheet = current(ctx).filter(|sheet| sheet.menu == menu)?;
    sheet.shown_frame = ctx.cumulative_frame_nr();
    let showing_page = !sheet.pages.is_empty();
    ctx.data_mut(|data| data.insert_temp(sheet_id(), sheet));
    let tag = Tag { menu, page: menu };
    if showing_page {
        let mut unseen = Ui::new(
            ctx.clone(),
            area_id().with("unseen"),
            tagged(tag).invisible(),
        );
        let inner = add_contents(&mut unseen);
        let frame = ctx.cumulative_frame_nr();
        if ctx.data(|data| data.get_temp::<u64>(page_shown_id())) != Some(frame) {
            // The open page's submenu is gone, or its label changed: show
            // the menu again.
            if let Some(mut sheet) = current(ctx) {
                sheet.pages.clear();
                store(ctx, sheet);
            }
        }
        return Some(inner);
    }
    let (inner, outcome) = show_page(ctx, tag, None, add_contents);
    outcome.apply(ctx);
    Some(inner)
}

/// A submenu in a sheet: a row with a chevron, and the submenu's page once
/// the row is tapped.
pub(crate) fn submenu<R>(
    ui: &mut Ui,
    tag: Tag,
    row: Row<'_, '_>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    let title = row.label.text().to_owned();
    let id = tag.page.with(&title);
    let response = row.show(ui);
    let enabled = ui.is_enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &title));
    let Some(mut sheet) = current(ui.ctx()).filter(|sheet| sheet.menu == tag.menu) else {
        return InnerResponse::new(None, response);
    };
    let depth = sheet.pages.iter().position(|page| *page == id);
    let page = Tag {
        menu: tag.menu,
        page: id,
    };
    let inner = match depth {
        // Only rows on the page that shows can be tapped.
        None => {
            if response.clicked() {
                sheet.pages.push(id);
                store(ui.ctx(), sheet);
            }
            None
        }
        // A page above this one shows; this submenu contains it.
        Some(depth) if depth + 1 < sheet.pages.len() => {
            Some(ui.scope_builder(tagged(page), add_contents).inner)
        }
        Some(_) => {
            let frame = ui.ctx().cumulative_frame_nr();
            ui.ctx()
                .data_mut(|data| data.insert_temp(page_shown_id(), frame));
            let (inner, outcome) = show_page(ui.ctx(), page, Some(&title), add_contents);
            outcome.apply(ui.ctx());
            Some(inner)
        }
    };
    InnerResponse::new(inner, response)
}

/// What the sheet does after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Stay,
    /// The back row or Escape: back to the previous page, or closed.
    Back,
    /// A row called `ui.close()`, or the backdrop was tapped.
    Close,
}

impl Outcome {
    fn apply(self, ctx: &Context) {
        match self {
            Self::Stay => {}
            Self::Back => {
                dismiss_menu_sheet(ctx);
            }
            Self::Close => close(ctx),
        }
    }
}

/// The sheet showing a menu, or a submenu's page under a back row with its
/// `title`.
fn show_page<R>(
    ctx: &Context,
    tag: Tag,
    title: Option<&str>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (R, Outcome) {
    let viewport = unobscured_content_rect(ctx);
    let style = ctx.global_style();
    let frame = sheet_frame(&style);
    let chrome = frame.total_margin().sum();
    let inner_width = (viewport.width().min(MAX_WIDTH) - chrome.x).max(1.0);
    let max_height = viewport.height() * SHEET_MAX_HEIGHT_FRACTION - chrome.y;
    let modal = egui::Modal::new(area_id())
        .area(
            egui::Modal::default_area(area_id())
                .anchor(Align2::CENTER_BOTTOM, egui::Vec2::ZERO)
                .constrain_to(viewport),
        )
        .frame(frame)
        .backdrop_color(backdrop_color(&style))
        .show(ctx, |ui| {
            ui.set_width(inner_width);
            ui.scope_builder(tagged(tag), |ui| {
                let inset =
                    egui::Frame::NONE.inner_margin(egui::Margin::symmetric(ROW_PADDING as i8, 0));
                let back = title
                    .is_some_and(|title| inset.show(ui, |ui| back_row(ui, title)).inner.clicked());
                let rows_height =
                    (max_height - if title.is_some() { ROW_HEIGHT } else { 0.0 }).max(ROW_HEIGHT);
                // The minimum lets the rows grow past the sheet's height in
                // the previous frame, which caps the space egui offers: the
                // sheet fits its rows, up to `rows_height`, on every page.
                let inner = egui::ScrollArea::vertical()
                    .id_salt(tag.page)
                    .max_height(rows_height)
                    .min_scrolled_height(rows_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| inset.show(ui, add_contents).inner)
                    .inner;
                (inner, back)
            })
            .inner
        });
    let (inner, back) = modal.inner;
    let escape = || ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    let outcome = if modal.backdrop_response.clicked() || modal.response.should_close() {
        Outcome::Close
    } else if back || (modal.is_top_modal && !modal.any_popup_open && escape()) {
        Outcome::Back
    } else {
        Outcome::Stay
    };
    (inner, outcome)
}

/// A page's first row: a chevron pointing back, then the submenu's title.
fn back_row(ui: &mut Ui, title: &str) -> Response {
    let response = Row {
        label: RichText::new(title).strong().into(),
        icon: None,
        icon_column: true,
        value: None,
        destructive: false,
        trailing: Trailing::Nothing,
    }
    .show(ui);
    let enabled = ui.is_enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Back"));
    let center = egui::pos2(
        response.rect.left() + ROW_PADDING + ICON_SIZE * 0.5,
        response.rect.center().y,
    );
    paint_chevron(ui, center, -1.0, ui.visuals().text_color());
    ui.separator();
    response
}

/// The title above a sheet's contents, styled as a dialog's header, with a
/// close button at its end when `close_button` is set.
pub(crate) fn title(ui: &mut Ui, title: &str, close_button: bool) {
    let metrics = Metrics::of(ui.ctx());
    let height = metrics.control_height;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width().max(1.0), height),
        Layout::right_to_left(Align::Center),
        |ui| {
            if close_button
                && ghost_icon_button(ui, "×", egui::Vec2::splat(height), "Close").clicked()
            {
                ui.close();
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let title = RichText::new(title)
                    .strong()
                    .size(metrics.panel_title_text_size);
                ui.add(egui::Label::new(title).truncate());
            });
        },
    );
}

/// What a row shows at its end.
pub(crate) enum Trailing<'v> {
    Nothing,
    /// The row opens a submenu.
    Chevron,
    /// One of several choices, with a check mark while selected.
    Check(bool),
    /// An on/off option that a tap flips.
    Switch(&'v mut bool),
}

/// A tall row of a sheet. The caller reports its accessibility information.
pub(crate) struct Row<'a, 'v> {
    pub(crate) label: WidgetText,
    pub(crate) icon: Option<&'a str>,
    /// Keeps the icon column without an icon, so that the label lines up
    /// with the rows around it that have one.
    pub(crate) icon_column: bool,
    /// Dimmed text before the trailing mark, such as a submenu's current
    /// choice.
    pub(crate) value: Option<String>,
    pub(crate) destructive: bool,
    pub(crate) trailing: Trailing<'v>,
}

impl Row<'_, '_> {
    pub(crate) fn show(self, ui: &mut Ui) -> Response {
        let Self {
            label,
            icon,
            icon_column,
            value,
            destructive,
            trailing,
        } = self;
        let width = ui.available_width().max(1.0) + 2.0 * ROW_PADDING;
        let track = track_size(control_height(ui));
        let label_left = ROW_PADDING + if icon_column { ICON_COLUMN } else { 0.0 };
        let trailing_width = match trailing {
            Trailing::Nothing => 0.0,
            Trailing::Chevron => CHEVRON_EXTENT + SPACE_SM,
            Trailing::Check(_) => CHECK_EDGE + SPACE_SM,
            Trailing::Switch(_) => track.x + SPACE_SM,
        };
        let text_room = (width - label_left - ROW_PADDING - trailing_width).max(1.0);
        // The value gives way to the label: it may take at most half the room.
        let value = value.map(|value| {
            WidgetText::from(value).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                text_room * 0.5,
                egui::TextStyle::Button,
            )
        });
        let value_width = value
            .as_ref()
            .map_or(0.0, |value| value.size().x + SPACE_SM);
        let galley = label.into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (text_room - value_width).max(1.0),
            egui::TextStyle::Button,
        );
        // Rows reach over the sheet's inset and into the spacing around them,
        // so that neighbouring rows meet like a list's while other contents
        // keep their spacing.
        let gap = ui.spacing().item_spacing.y;
        let (allocated, base) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().max(1.0), (ROW_HEIGHT - gap).max(1.0)),
            Sense::hover(),
        );
        let rect = allocated.expand2(egui::vec2(ROW_PADDING, gap * 0.5));
        let mut response = ui.interact(rect, base.id.with("row"), Sense::click());
        let mark = match trailing {
            Trailing::Nothing => Mark::Nothing,
            Trailing::Chevron => Mark::Chevron,
            Trailing::Check(selected) => Mark::Check(selected),
            Trailing::Switch(value) => {
                if response.clicked() {
                    *value = !*value;
                    response.mark_changed();
                }
                Mark::Switch(*value)
            }
        };
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let visuals = ui.style().interact(&response);
        let painter = ui.painter();
        if response.hovered() || response.is_pointer_button_down_on() || response.has_focus() {
            painter.rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                visuals.corner_radius,
                Stroke::new(1.5, ui.visuals().widgets.active.bg_stroke.color),
                StrokeKind::Inside,
            );
        }
        let color = if destructive {
            ui.visuals().error_fg_color
        } else {
            visuals.text_color()
        };
        if let Some(icon) = icon {
            painter.text(
                egui::pos2(rect.left() + ROW_PADDING + ICON_SIZE * 0.5, rect.center().y),
                Align2::CENTER_CENTER,
                icon,
                FontId::proportional(ICON_SIZE),
                color,
            );
        }
        let text_position = egui::pos2(
            rect.left() + label_left,
            rect.center().y - galley.size().y * 0.5,
        );
        painter.galley(text_position, galley, color);
        let trailing_right = rect.right() - ROW_PADDING;
        if let Some(value) = value {
            let position = egui::pos2(
                trailing_right - trailing_width - value.size().x,
                rect.center().y - value.size().y * 0.5,
            );
            painter.galley(position, value, ui.visuals().weak_text_color());
        }
        let center_y = rect.center().y;
        match mark {
            Mark::Nothing | Mark::Check(false) => {}
            Mark::Chevron => {
                let center = egui::pos2(trailing_right - CHEVRON_EXTENT * 0.5, center_y);
                paint_chevron(ui, center, 1.0, ui.visuals().weak_text_color());
            }
            Mark::Check(true) => {
                let check = egui::Rect::from_center_size(
                    egui::pos2(trailing_right - CHECK_EDGE * 0.5, center_y),
                    egui::Vec2::splat(CHECK_EDGE),
                );
                paint_check(ui, check, ui.visuals().selection.stroke.color);
            }
            Mark::Switch(checked) => {
                let track_rect = egui::Rect::from_min_size(
                    egui::pos2(trailing_right - track.x, center_y - track.y * 0.5),
                    track,
                );
                paint_switch(ui, &response, track_rect, checked);
            }
        }
        response
    }
}

/// What a row paints at its end, decided once a switch has taken its click.
enum Mark {
    Nothing,
    Chevron,
    Check(bool),
    Switch(bool),
}

/// A check mark filling `rect`.
pub(crate) fn paint_check(ui: &Ui, rect: egui::Rect, color: Color32) {
    let point = |x: f32, y: f32| {
        egui::pos2(
            egui::lerp(rect.left()..=rect.right(), x),
            egui::lerp(rect.top()..=rect.bottom(), y),
        )
    };
    ui.painter().add(egui::Shape::line(
        vec![point(0.15, 0.55), point(0.4, 0.8), point(0.85, 0.25)],
        Stroke::new(2.0, color),
    ));
}

/// A chevron centred on `center`, pointing right for a `direction` of 1 and
/// left for -1.
fn paint_chevron(ui: &Ui, center: egui::Pos2, direction: f32, color: Color32) {
    let half = CHEVRON_EXTENT;
    let tip = center + egui::vec2(direction * half * 0.5, 0.0);
    let back = center - egui::vec2(direction * half * 0.5, 0.0);
    ui.painter().add(egui::Shape::line(
        vec![
            back - egui::vec2(0.0, half),
            tip,
            back + egui::vec2(0.0, half),
        ],
        Stroke::new(1.5, color),
    ));
}
