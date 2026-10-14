---
id: 01
title: Crate scaffold, generated tokens for all six themes, minimal runnable gallery
tag: AFK
priority: critical
blocked-by: []
blocks: [02, 03, 04]
estimate: large
status: done
completed: 2026-10-13T21:37:48Z
commit: 73fd0684bf6bfec5fcef9dc7d0f6f6a88c707b18
modules:
  - crates/ui-kit/Cargo.toml
  - crates/ui-kit/src/lib.rs
  - crates/ui-kit/src/tokens/
  - crates/ui-kit/src/theme.rs
  - crates/ui-kit/tools/generate_tokens.py
  - crates/ui-kit/examples/gallery.rs
  - Cargo.toml
  - justfile
  - .gitignore
---

# Crate scaffold, generated tokens for all six themes, minimal runnable gallery

## What changes for the user

`cargo run -p ui-kit --example gallery` opens a desktop window titled "Vynatix UI kit"
showing the Forest Heritage light canvas, a Fraunces-less placeholder heading (system
font until issue 02), a row of six theme buttons (Forest/Teal/Monochrome, light/dark
each) and a palette swatch grid for the active theme. Clicking a theme button re-skins
the whole window immediately. `just check`, `just lint`, `just test` all include the
new crate and stay green.

## Acceptance criteria

- `crates/ui-kit` is a workspace member; its Cargo.toml mirrors `crates/rustnotes-core`:
  workspace deps only, `egui` with `default-features = false`, edition 2021,
  `rust-version = "1.95"`, plus `[dev-dependencies] eframe` (workspace, default-features
  false + the features the example needs, e.g. `winit/default` per how `crates/rustnotes`
  hosts eframe — check what minimal set compiles on macOS).
- `.gitignore` gains `/.afk/` in this, the first commit on the branch.
- `tools/generate_tokens.py` (python3 + stdlib only; fonttools NOT needed here) reads
  `../../../.afk/design-system/tokens.json` and writes generated Rust modules under
  `src/tokens/` (colour roles, spacing, radii, shadows, type scale, leading, tracking,
  breakpoints, containers, icon tokens, motion constants). Generated files carry a
  "generated, do not edit" header. The generated code is COMMITTED (builds never run
  the generator). The script also resolves `{palette-ref}` role values and per-theme
  shadow strings into typed structs.
- Six themes: `ThemeId` enum (`Light, Dark, TealLight, TealDark, MonoLight, MonoDark`)
  with names ("Forest light" etc. per tokens.json `color.themes`). A `Vynatix`/theme
  struct holds all 54 resolved per-theme role tokens (`Color32`), including rgba()
  values. `ThemeId::ALL` iterates in tokens.json order.
- Theme application: each theme maps to egui `Visuals` (panel fill = surface, text
  colours, selection, hyperlink = primary-hover, etc.) following the pattern of
  `crates/rustnotes-core/src/layout/palette.rs` (read it first; do not modify it).
- Unit tests, all in-crate: (a) generated role values equal the JSON-resolved values
  for a sample of roles across all six themes (test reads tokens.json via
  `include_str!`? No — tests validate against a small committed snapshot fixture the
  generator also emits, plus hand-checked anchors: primary light = #1E3A2F etc.);
  (b) WCAG contrast: for every theme, `on-primary`/`primary`, `on-surface`/`surface`,
  `on-surface-variant`/`surface-variant`, `text-accent` on `surface-container-low`
  ≥ 3:1 (the brand book's stated floor for accent text; body pairs ≥ 4.5:1) — reuse
  the luminance/contrast helpers pattern from palette.rs; (c) spacing/radii lookups
  return only approved values.
- justfile: `check` adds `cargo check -p ui-kit` (host only — no mobile targets needed,
  the crate is pure egui), `test` runs `cargo test -p ui-kit` in addition to
  rustnotes-core, `lint` adds `-p ui-kit` to the clippy invocation. Existing recipes
  and their app-parametric shape stay untouched.
- Example `examples/gallery.rs`: eframe app, theme state, six buttons from ThemeId::ALL,
  swatch grid showing the role tokens of the active theme with their names in
  sentence case, no emoji, no exclamation marks, no em dashes.

## Out of scope for this issue

- Fonts and the real type system (issue 02). Gallery text uses egui defaults.
- Icons (03), state helpers (04), components (05-10).
- README/docs beyond a crate-level stub (11).

## Implementation notes

- Read `.afk/design-system/README.md` sections "Themes", "Colors", "Visual foundations"
  and `.afk/design-system/tokens.json` first. Role tokens are the 54 dict-valued
  entries in `color.tokens`; palette refs like `{forest-30}` resolve against the
  flat 126 constants; some values are raw hex or `rgba()`.
- The gallery is the demo surface for everything downstream; keep its module
  structure (`examples/gallery.rs` + `src/` sections) ready to grow per issue.
- Commit style: conventional commits, reference the issue id (`closes #01` style
  isn't available — mention `issue 01` in the body).

## Definition of done

- [ ] All acceptance tests pass (red → green observed).
- [ ] `just check`, `just lint`, `just test` green including ui-kit.
- [ ] One commit, one issue. Commit message references this file by id.
- [ ] Issue file updated with `status: done` in frontmatter.
