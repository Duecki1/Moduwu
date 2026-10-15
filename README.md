# Moduwu Design

A standalone Rust library for egui 0.35, extracted from CalibRaw. It provides
six theme presets (Obsidian Blue, Obsidian Red, Porcelain, Daylight, and
Plain Grey in dark and light),
custom palette support, spacing and sizing metrics, cards, toolbar rows,
buttons, form controls, numeric fields, sliders, menus, dialogs, animated
clips, and responsive layout helpers.

## Gallery

```sh
cargo run --example gallery
```

The gallery shows every reusable control in each theme. Desktop and Android
metrics can be switched at runtime, and the "Enabled" toggle shows disabled
states; Tab moves keyboard focus. `cargo test` renders it headlessly in every
combination.

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
