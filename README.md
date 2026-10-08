# Moduwu Design

A standalone Rust library for egui 0.35, extracted from CalibRaw. It provides
six theme presets (Obsidian Blue, Obsidian Red, Porcelain, Daylight, and
Plain Grey in dark and light),
custom palette support, spacing and sizing metrics, cards, toolbar rows,
buttons, form controls, numeric fields, sliders, menus, dialogs, and
responsive layout helpers.

## Gallery

```sh
cargo run --example gallery
```

The gallery shows every reusable control in each theme. Desktop and Android
metrics can be switched at runtime, and the "Enabled" toggle shows disabled
states; Tab moves keyboard focus. `cargo test` renders it headlessly in every
combination.

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
