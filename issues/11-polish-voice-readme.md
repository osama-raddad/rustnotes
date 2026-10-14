---
id: 11
title: Gallery polish, voice lint, README and docs
status: done
completed: 2026-10-14
commit: aa9e82c
tag: AFK
priority: medium
blocked-by: [05, 06, 07, 08, 09, 10]
blocks: []
estimate: medium
modules:
  - crates/ui-kit/README.md
  - crates/ui-kit/examples/gallery.rs
  - crates/ui-kit/src/gallery_copy.rs
  - README.md
---

# Gallery polish, voice lint, README and docs

## What changes for the user

The gallery is a finished product: opens on the Cover, navigable sections in a
sensible order, a theme switcher that persists for the session, no rough edges.
The crate carries a README that tells the next engineer what the kit is, how to
use it, which fonts/licences are vendored, and exactly where the implementation
deviates from the brand book. The workspace README points to it.

## Acceptance criteria

- Gallery polish pass: section order (Cover, Themes, Foundations, Type, Icons,
  Logo, Brand, Buttons, Inputs, Cards, Navigation/feedback), the six-theme
  switcher reachable from the TopAppBar (a Themes-style menu), empty states or
  scroll misbehaviours fixed, Tab order sane, focus ring visible on every
  interactive element.
- Voice lint test: all gallery copy routed through a `gallery_copy` module (string
  constants); a unit test scans them for forbidden content: emoji (any codepoint
  outside BMP-text ranges / explicit emoji blocks), em dash (—, U+2014), ellipsis
  in OUR copy per rules (…, U+2026 — allowed only where the spec itself uses it
  in an operational "Sending…" pattern), exclamation marks, forbidden vocabulary
  list (leverage, empower, unlock, world-class, best-in-class, cutting-edge,
  next-generation, disruptive, transformative, supercharge, revolutionise,
  game-changing); title-case headings rejected (sentence case enforced by a
  heuristic: no heading string may be fully title-cased for >3 words). Failing
  strings are fixed in the copy, not the test.
- Crate README (`crates/ui-kit/README.md`): what it is; how to run the gallery
  (`cargo run -p ui-kit --example gallery`); how to apply a theme in an egui app
  (minimal snippet); the fonts table (which faces at which instances, why);
  licence notes (vendored OFL texts + Aptos restriction + Fraunces/Geist upstream
  OFL note); THE DEVIATIONS LIST (glass without blur, reduced-motion/transparency
  as flags, eyebrow tracking approach, H2 opsz 144 vs 96, lede weight 400 vs 350,
  icon opsz pinned 24, icon grade fallback if applied, `location_on` aliasing,
  variable-font axes instanced static, no media queries); generator usage
  (tools/*.py, when to rerun, output is committed).
- Workspace `README.md`: short pointer section for the new crate (match the
  existing README's structure and tone).
- `just check/lint/test` green; `cargo run -p ui-kit --example gallery` compiles
  with no warnings.
- Deviations cross-checked against `.afk/review-notes.md` if review already ran
  (ordering may put review after this issue; then the review phase handles it).

## Out of scope for this issue

- Visual pixel-QA against the HTML previews (user's manual QA after the PR).
- Any rustnotes app integration.

## Implementation notes

- This is the catch-all closure issue; if earlier issues left small TODOs that are
  documentation-shaped, they land here.

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
