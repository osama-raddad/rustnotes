---
id: 06
title: Components: Checkbox, Radio, Toggle with gallery section
tag: AFK
priority: high
blocked-by: [04, 03]
blocks: [11]
estimate: medium
status: done
completed: 2026-10-14T02:33:33+0200
commit: b47cc83516f82fdabc61078065541bcef28a31a5
modules:
  - crates/ui-kit/src/components/checkbox.rs
  - crates/ui-kit/src/components/radio.rs
  - crates/ui-kit/src/components/toggle.rs
  - crates/ui-kit/examples/gallery.rs
---

# Components: Checkbox, Radio, Toggle with gallery section

## What changes for the user

A "Components: choice controls" gallery section: checkboxes (single and group,
unchecked/checked/indeterminate where the spec has it, disabled), a radio group
(e.g. region picker per the spec's example), and toggles (on/off/disabled, with
label) — all focusable, all showing the brand focus ring, all re-skinning across
the six themes.

## Acceptance criteria

- Checkbox per `components/Checkbox/README.md`: box size/shape per spec (radius per
  token), checked mark via the `check` icon glyph, label beside in body face,
  hover/press/focus/disabled per 04 helpers; tri-state support if the spec defines
  indeterminate (read the preview); returns typed state.
- Radio per `components/Radio/README.md`: circular mark, group semantics (one
  selected), label, states per 04; typed selection response.
- Toggle per `components/Toggle/README.md`: pill track + knob, on/off colours from
  roles, motion within egui's per-frame model (animate knob over ~200ms using ctx
  time; reduced-motion flag snaps instantly — the spec's example use IS reduced
  motion, show that variant in the gallery via the flag), disabled at 38%/12%.
- Unit tests: toggle state machine (off→on→off, disabled no-op), radio group
  selection exclusivity, checkbox tri-state transitions, knob travel math at t=0,
  0.5, 1 for a known duration.
- Gallery section with live controls and voice-clean copy (e.g. a "Reduced motion"
  toggle that actually flips the kit flags — the spec's own example).
- `just check/lint/test` green.

## Out of scope for this issue

- TextField/Select (07); form layout helpers.

## Implementation notes

- Spec sources: `components/{Checkbox,Radio,Toggle}/README.md` + previews.
- egui gives `ui.interact` + `Response.clicked`; keyboard activation for
  checkboxes/radios via egui's button sense or explicit key handling.

## Definition of done

- [ ] All acceptance tests pass (red → green observed).
- [ ] `just check`, `just lint`, `just test` green.
- [ ] One commit, one issue. Commit message references this file by id.
- [ ] Issue file updated with `status: done` in frontmatter.
