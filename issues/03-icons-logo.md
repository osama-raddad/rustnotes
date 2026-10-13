---
id: 03
title: Icons and logo: instanced Material Symbols subsets, SVG logo widgets, gallery sections
tag: AFK
priority: high
blocked-by: [02]
blocks: [05, 06, 07, 09]
estimate: medium
status: done
completed: 2026-10-13T22:41:10Z
commit: 9936cc621165c65d281506dafaa2bb7f8e9079f6
modules:
  - crates/ui-kit/tools/prepare_icons.py
  - crates/ui-kit/assets/fonts/
  - crates/ui-kit/assets/logos/
  - crates/ui-kit/src/icons.rs
  - crates/ui-kit/src/logo.rs
  - crates/ui-kit/examples/gallery.rs
---

# Icons and logo: instanced Material Symbols subsets, SVG logo widgets, gallery sections

## What changes for the user

Two new gallery sections. "Icons" shows the approved 162-icon catalogue at sizes
16/20/24/32/48 in the active theme, with filled variants for the icons that have
them, rendered from a small subsetted self-hosted font at the brand's weight 300.
"Logo" shows the V mark and wordmark coloured by role tokens on canvas, primary and
inverse bands, with minimum-size guidance in the caption. Everything re-skins on
theme switch; dark themes use the darker icon grade.

## Acceptance criteria

- `tools/prepare_icons.py` (python3 + fonttools + brotli, run manually, output
  committed): decompresses `.afk/design-system/fonts/MaterialSymbolsOutlined.woff2`
  to TTF, reads the catalogue
  `.afk/design-system/assets/Icons/catalogue.json`, resolves each icon name to its
  PUA codepoint via the font cmap (glyph names match; `location_on` has no glyph in
  font 0.47.5 — alias it to the visually closest catalogue sibling or exclude it,
  and emit a Rust-side documented alias), then instances and subsets four fonts:
  outline-grade-0 (wght 300, GRAD 0, FILL 0, opsz 24), outline-grade-neg25 (GRAD −25),
  fill-grade-0 (FILL 1, GRAD 0), fill-grade-neg25 (FILL 1, GRAD −25). Print sizes;
  if the four subsets exceed ~1.5 MB total, drop the grade distinction to two fonts
  and record that as a deviation.
- The script also generates `src/icons.rs` (or a generated submodule): an `Icon`
  enum with all catalogue names (snake_case variants), `as_char()` → the PUA char,
  plus name/fill metadata (which icons have `-fill.svg` per the assets dir), and a
  category list for the gallery. Generated header, committed.
- Font embedding: icon families registered additively alongside text families
  (`ui-kit` icon family per grade/fill variant); `icon_font(dark: bool)` /
  selection helpers. Icon widget: `fn icon(ui, icon, size, color)` painting the
  glyph via `FontId` at icon token sizes; sizes only from the icon token set
  (16/20/24/32/48); unit test asserts the enum covers exactly the catalogue names
  (test reads catalogue.json at compile time via include of a generated count const —
  no runtime fs).
- Logo: parse the single-path V mark and wordmark SVGs ONCE at build time via the
  python script (emit normalized path data as Rust constants — a tiny
  tessellation-free representation is fine if you paint with `epaint::Shape`,
  or re-use the SVG as egui `Vec` of points with `PathShape`). Supported inks are
  ROLE colours at paint time (primary, on-primary, inverse-on-surface, bronze via
  text-accent/indicator) — never bake hex into the shapes; the per-ink source files
  exist only as reference. `Logo::mark()` / `Logo::wordmark()` widgets take the
  colour from the current theme roles by default with an override. Minimum sizes
  enforced by debug assert/log (wordmark ≥ 96px, mark ≥ 24px in chrome).
- Gallery sections: "Icons" (grid grouped by catalogue category, filled variants
  shown for the 29 that have them, sizes row) and "Logo" (mark + wordmark on
  canvas/primary/inverse bands). Voice-clean captions.
- `just check/lint/test` green.

## Out of scope for this issue

- Component usage of icons (05-09). Icon colour routing rules ("primary for acting
  icons, muted for secondary") land with the components that use them.
- The Brand/Cover/Themes showcase pages (10).

## Implementation notes

- The PUA chars are private-use (U+E000..U+F8FF); Rust string literals are fine via
  `char::from_u32`.
- egui paints glyphs from installed fonts; the icon is just a one-char `RichText`
  in the icon family. Verify baseline/size behaviour at 16px (opsz pinned at 24 —
  minor deviation vs "opsz follows size", document it).
- Logo SVGs: `vynatix-mark-*.svg` is 1.8 KB, wordmark 7.7 KB — single paths,
  viewBox `0 0 …`. Normalizing to `epaint` points with a build-time sampled
  tessellation is acceptable (document tolerance), or use `PathShape` with
  beziers if you emit control points directly.

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
