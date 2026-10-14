---
id: 13
title: ui-kit: one-call theme go-live and the single colour-resolution path
tag: human-in-loop
priority: medium
blocked-by: []
blocks: []
estimate: medium
modules:
  - crates/ui-kit/src/theme.rs
  - crates/ui-kit/src/typography.rs
  - crates/ui-kit/src/icon.rs
---

# ui-kit: one-call theme go-live and the single colour-resolution path

From the architecture audit (candidates 3+5, to be designed together). "Make a
theme live" is a three-call protocol (`typography::install` + `icon::install` +
`ThemeId::apply`) whose completeness lives in each caller's head and fails
silently as font fallback; the gallery re-implements it twice and ~22 test
modules once each. Meanwhile colour resolution has two paths: explicit
`ThemeId` parameters on components vs context-reads (`ThemeId::current` for the
logo with `unwrap_or(Light)` hiding mistakes; `ui.visuals().dark_mode` for icon
grade) that silently disagree when `apply` never ran.

Design note for the grilling pass: either promote "the live theme" to a named
concept owning the go-live protocol, or fold the readers into the explicit
interface. Decide before the rustnotes app migration starts (see audit
candidate 7's decision note about `Palette::of(ctx)` vs the 54-role model).
