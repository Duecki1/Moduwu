# Moduwu Design agent rules

Moduwu is a reusable egui design library: themes, tokens, metrics and
controls shared by applications such as CalibRaw.

- Presentation only. No application business logic, persistence, photo
  processing or dependency on an application crate.
- Every preset defines every palette and token role. A new role is added to all
  presets in the same change.
- Preserve appearance and geometry when extracting code from an application;
  standardize sizes, radii or strokes only in deliberate visual changes.
- Custom-painted widgets must expose accessibility information through egui's
  `widget_info`: role, label, value where applicable, enabled state, and focus.
  They must respond to the actions they advertise (activation, set value,
  increment/decrement). Wrappers around built-in egui widgets keep the
  built-in information instead of replacing it.
- Tests cover behavior callers rely on (interaction, keyboard, metrics, theme
  application, accessibility), not just construction.
- Public API changes follow semantic versioning. Note breaking changes and
  coordinate them with dependent applications before release.
- `rust-version` stays at or below CalibRaw's (1.92).
- Small helpers can be plain functions; add a builder only when a control has
  several optional settings.

## Verification

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```
