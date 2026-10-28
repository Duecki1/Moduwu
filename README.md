# Moduwu Design

A standalone Rust library for egui 0.35, extracted from CalibRaw. It provides
six theme presets (Obsidian Blue, Obsidian Red, Porcelain, Daylight, and
Plain Grey in dark and light),
custom palette support, spacing and sizing metrics, cards, toolbar rows,
buttons, form controls, numeric fields, sliders, tree rows, menus, dialogs,
animated clips, and responsive layout helpers.

## Gallery

```sh
cargo run --example gallery
```

The gallery shows every reusable control in each theme. Desktop and Android
metrics can be switched at runtime, and the "Enabled" toggle shows disabled
states; Tab moves keyboard focus. `cargo test` renders it headlessly in every
combination.

## Changes in 4.0

- Menus open as bottom sheets on touch layouts (`Metrics::touch_layout`):
  `context_menu`, `dropdown_menu` and `overflow_menu` keep their arguments
  and show a popup on desktop. A sheet rises from the bottom of the viewport
  with the dialog sheet's frame and backdrop, at most 640 points wide, and
  closes when a row calls `ui.close()`, the backdrop is tapped, or Escape is
  pressed. A context menu's sheet opens on a right-click or a long press.
  Its contents are inset so that headings, chips and other controls line up
  with the icons of its rows.
- Breaking: `context_menu` and `dropdown_menu` return `Option<R>`, what the
  contents returned while the menu shows, instead of the popup's
  `InnerResponse`.
- Breaking: `MenuItem` replaces `menu_item`, `context_menu_item` and
  `destructive_menu_item`. Add it with `ui.add` or `ui.add_enabled`;
  `MenuItem::icon` sets the glyph a sheet shows before the label, and
  `MenuItem::destructive` colours actions such as Delete. In a popup it is a
  compact text row.
- Breaking: `submenu` replaces `dropdown_submenu`. It takes a `MenuItem` and
  returns the row's response with what the contents returned while they show.
  In a sheet the submenu opens as a page with a back row. Like the menu
  itself, a submenu closes only when a row calls `ui.close()`, so a list of
  several choices stays open while they are picked.
- `MenuItem::value`: the current value of what a row sets, shown dimmed at
  its end, such as a sort order on the row that opens the list of them.
- `MenuItem::selected`: a row that is one of several choices, with a check
  mark while selected: before the label in a popup, after it in a sheet.
- `menu_checkbox_item` shows a switch after its label in a sheet.
- `menu_title`: the title a sheet shows above its rows, styled as a dialog's
  header, such as the name of the photo a menu acts on. Popups leave it out.
  `menu_title_with_close` adds a close button for menus whose options leave
  them open, such as filters.
- `menu_sheet_open` and `dismiss_menu_sheet`, for the system back button:
  going back leaves a submenu's page, then closes the sheet.
- `touch_hold`, `set_long_press_duration` and `DEFAULT_LONG_PRESS`: long
  presses timed with the platform's Touch & hold delay rather than egui's,
  which is twice Android's default. A press fires once, on the frame its
  delay ends, unless the touch moved; lifting it afterwards is no tap.

## Changes in 3.4

- `tree` and `TreeRow`: the rows of a collapsible tree, such as a folder
  hierarchy. Each row is indented by its depth, with a disclosure arrow when it
  has children, an optional icon and its label; a child's arrow sits under its
  parent's icon, and guide lines hang from each ancestor's arrow. Hover and
  selection fill the whole row. The caller shows the children of expanded rows
  and flips the expanded state when `TreeRowResponse::toggled` reports it.

## Changes in 3.3

- `Slider::scale` with `SliderScale::Reciprocal`: places values along the
  track by their reciprocal, such as a colour temperature spaced in mired
  while it is shown and typed in kelvin. The value field, keyboard steps and
  accessibility values keep the value's units.

## Changes in 3.2

- `toggle_row`: a switch on its own full-width line, like `toggle_with_help`
  without the tooltip.
- A focused switch draws the same focus ring as a segmented control.

## Changes in 3.1

- `Dialog::show_with_preview` and `DialogSize::Preview`: the modal dialog
  with an image preview beside (landscape) or above (portrait and bottom
  sheets) its scrolling controls, as `DialogWindow::show_with_preview` does.
- `Dialog::area_id`: the id of a dialog's area, for finding it in egui's
  memory.

## Changes in 3.0

- Breaking: `Slider::hover_text` and `AngleDial::hover_text` are removed, and
  sliders and dials no longer show a "Double-click to reset" tooltip. A hint
  must not cover a control while it is dragged; put help on a static label or
  heading instead. Double-click still resets.
- Frameless controls (`ghost_icon_button`, `ghost_icon_toggle_button`,
  unselected `navigation_row`, `context_menu_item`, `chip`) and the rows of
  Moduwu menus and combo boxes no longer shift their label by a pixel when
  hovered. Menu and combo-box rows are flat until hovered.
- Icon buttons (`icon_button`, `icon_toggle_button` and the ghost variants)
  are exactly the requested width with the glyph centred. Button padding made
  them about 4 px wider, which overflowed wrapping rows.

## Changes in 2.4

- `toggle` and `toggle_with_help` draw a label and a sliding switch instead of
  a toggle button. `toggle` hugs its label; `toggle_with_help` fills the row
  and puts the switch at its end. Signatures and checkbox semantics are
  unchanged. `toggle_button` stays for tool modes.
- `ghost_icon_button` and `ghost_icon_toggle_button`: icon buttons whose frame
  appears only on hover, press, focus or selection, for dense rows.
- `segmented_control`: one choice among a few options in a single track with a
  sliding highlight; arrow keys move the selection.
- `chip`: a pill-shaped selectable option for filters and quick choices.
- `menu_checkbox_item`: a menu row with a check mark for on/off options.
- `Dialog` (with `DialogSize` and `DialogResponse`): a modal dialog with a
  dimmed backdrop, a header with an optional close button, a scrolling body and
  a fixed footer. It becomes a bottom sheet on narrow viewports and touch
  layouts in portrait. `dialog_confirmation_actions` adds confirm and cancel
  actions to its footer.

## Changes in 2.3

- `AnimatedImage`: a short looping clip, such as a bundled help clip, decoded
  once from GIF or WebP bytes (animated or still) and painted into a rect, for
  example a preview dialog's preview. It plays on show, and a click, Space or
  Enter pauses and resumes it. Adds the `image` crate (GIF and WebP only).

## Changes in 2.2

- `DialogWindow::show_with_preview` (with `DialogPreview` and
  `DIALOG_WIDTH_PREVIEW`): a dialog that pairs an image preview with its
  controls, side by side on landscape screens and stacked on portrait ones. The
  preview always stays in view; only the controls scroll, above the action row.

## Changes in 2.1

- `request_scroll_into_view` and `scroll_into_view_on_draw`: an action asks for
  a widget that may first be drawn a frame later, and the enclosing scroll
  areas bring it into view once it is. A request lapses after a few passes.

## Changes in 2.0

- `NumberField`: a drag-value field with focus-aware arrow stepping and repeat,
  display precision, suffixes, explicit ids and commit-after-edit updates.
- `Slider`: a labelled slider with a value field, scroll-friendly dragging
  (`slider_scroll_locked`), double-click reset, keyboard stepping, gradients and
  AccessKit slider semantics.
- Runtime metrics: `Theme::apply_with_metrics` installs `Metrics` that controls
  read through `Metrics::of`; `Theme::apply` keeps the build platform's.
- Breaking: `Metrics` has a new `touch_layout` field.

Licensed under GPL-3.0-or-later; see [COPYING](COPYING).
