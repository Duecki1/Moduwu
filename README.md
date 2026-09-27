# Moduwu Design

A standalone Rust library for egui 0.35, extracted from CalibRaw. It provides
four theme presets (Obsidian Blue, Obsidian Red, Porcelain, and Daylight),
custom palette support, spacing and sizing metrics, cards, toolbar rows,
buttons, form controls, menus, dialogs, and responsive layout helpers.

It builds independently of CalibRaw. Photo editing, application state,
settings persistence, and domain-specific widgets stay in the application.

## Use locally

Keep the repositories beside each other:

```text
GitProjects/
├── CalibRaw/
└── moduwu-design/
```

In the consuming project's `Cargo.toml`:

```toml
[dependencies]
egui = "0.35"
moduwu-design = { path = "../moduwu-design" }
```

```rust
use moduwu_design::{Design, primary_action_button, section_card};

fn draw(ctx: &egui::Context, ui: &mut egui::Ui) {
    Design::ObsidianBlue.theme().apply(ctx);
    section_card(ui, "Example", |ui| {
        if primary_action_button(ui, "Save").clicked() {
            // Handle the application action here.
        }
    });
}
```

Apply the theme at startup and when the user selects a new theme. Keep the
application and library on the same egui version.

## Publish to GitHub and import it

Create an empty GitHub repository, then run from this folder (replace the URL
with your actual repository):

```sh
git add .
git commit -m "Extract reusable egui design system"
git remote add origin git@github.com:YOUR_ACCOUNT/moduwu-design.git
git push -u origin main
git rev-parse HEAD
```

CalibRaw currently imports this sibling folder through
`[workspace.dependencies]` in its root `Cargo.toml`. After pushing, replace
that dependency with your actual URL and the commit printed above:

```toml
moduwu-design = { git = "https://github.com/YOUR_ACCOUNT/moduwu-design.git", rev = "FULL_COMMIT_SHA" }
```

Run `cargo check -p calibraw-ui` in CalibRaw to update its `Cargo.lock`, then
commit the manifest and lockfile together. Cargo fetches the pinned library
for fresh clones and CI; those machines no longer need the sibling folder.
Until that switch, every build needs this folder beside CalibRaw.

## Development

Requires Rust 1.92 or newer. No CalibRaw native libraries are needed.

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Licensed under GPL-3.0-or-later; see [COPYING](COPYING).
