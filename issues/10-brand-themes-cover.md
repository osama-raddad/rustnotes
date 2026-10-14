---
id: 10
title: Brand moments: Themes, Brand, Cover gallery sections
tag: AFK
priority: medium
blocked-by: [02, 05]
blocks: [11]
estimate: medium
status: done
completed: 2026-10-14T14:12:00+0200
commit: 089c732a4113ec248258b8ab8f529b5dda53aefd
modules:
  - crates/ui-kit/src/components/themes_showcase.rs
  - crates/ui-kit/src/components/brand_showcase.rs
  - crates/ui-kit/src/components/cover.rs
  - crates/ui-kit/examples/gallery.rs
---

# Brand moments: Themes, Brand, Cover gallery sections

## What changes for the user

The gallery gains its three brand-book pages. A Cover: the gallery's opening hero
(brand-book style: eyebrow, display headline, standfirst) per
`components/Cover/README.md`. A Themes section: the six themes side by side as
miniature re-skinned specimens (roles swatch, a button, a card each — proving the
one-system-six-themes claim). A Brand section: the governing principles, the four
tone positions with one example sentence each, the three approved headline modes,
and the photography direction as tinted placeholder tiles with captions.

## Acceptance criteria

- Cover per `components/Cover/README.md`: the brand-book cover composition adapted
  as the gallery's first screen (eyebrow + display headline + standfirst + mark,
  correct type pairing ≥2.5×, canvas-led, accent as guest).
- Themes showcase per `components/Themes/README.md`: six theme cards (each card
  paints ITS theme's tokens regardless of the active theme — this requires
  painting with an explicit theme override, not ctx visuals; the tokens are plain
  data so this is a matter of passing the right token set into section painting);
  each shows name, anchors, and a live specimen row (primary button, card, chip).
- Brand showcase per `components/Brand/README.md`: principles, tone spectrum,
  headline modes, photography direction; copy lifted/adapted from the spec pages,
  passing the voice checklist (no em dashes, no exclamation, no emoji; middle dots
  for metadata).
- Unit tests: theme card coverage = ThemeId::ALL exactly once; cover type pairing
  uses display ≥2.5× body helper; brand copy constants pass the voice-lint test
  shape prepared here IF trivial, else in 11 (coordinate: the copy-lint test lands
  in 11 over the whole gallery).
- `just check/lint/test` green.

## Out of scope for this issue

- Logo rendering (03) — reused, not changed. Final copy-lint and docs (11).

## Implementation notes

- These three "components" are showcase pages in the source system; implementing
  them as gallery sections is the faithful port (per PRD module shape).
- Explicit-theme painting: a `ThemeTokens` value passed down instead of ambient
  ctx lookup inside these sections; the kit's component API should therefore
  accept tokens-or-ctx (prefer tokens-as-data everywhere; ctx is only for input).

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
