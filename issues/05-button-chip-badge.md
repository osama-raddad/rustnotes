---
id: 05
title: Components: Button, Chip, Badge with gallery section
tag: AFK
priority: high
blocked-by: [03, 04]
blocks: [09, 11]
estimate: medium
status: done
completed: 2026-10-14T01:17:06+02:00
commit: 7defe1a9706b42589702f67f78a41c3834e55aad
modules:
  - crates/ui-kit/src/components/button.rs
  - crates/ui-kit/src/components/chip.rs
  - crates/ui-kit/src/components/badge.rs
  - crates/ui-kit/examples/gallery.rs
---

# Components: Button, Chip, Badge with gallery section

## What changes for the user

A "Components: actions and labels" gallery section shows the Button in four
emphases (primary, secondary, tertiary, ghost) and three sizes (36/44/52px), with
icon-leading/icon-trailing and icon-only variants, in default, hover, pressed,
focus and disabled states; Chips as filter pills (selected/unselected, leading
icon, dismiss where spec'd); Badges as state/count pills. All re-skin per theme
and honour the reduced-motion/transparency flags.

## Acceptance criteria

- Button: egui widget (custom-painted, not egui::Button, to control states per
  spec — but reuse egui interaction sensing via `ui.interact`). Emphasis variants
  and colours exactly per `components/Button/README.md` (primary fill + on-primary
  label; secondary transparent + 1px outline + primary label; tertiary fill +
  on-tertiary; ghost transparent + primary label). Sizes: sm 36px/text-label,
  default 44px/body-sm, lg 52px/body-md; radius-md; body face weight 600; space-2
  icon gap. Hover 8% overlay (states helper), pressed 12% + 0.985 scale, focus ring
  via 04 helper, disabled 38%. Touch target ≥ 44px assert. Click returns a
  typed response. Icon-only variant requires an accessibility label (egui: set
  on the response via `response.widget_info` if available, else documented).
- Chip: pill (radius-full) per `components/Chip/README.md` — selected/unselected
  looks, optional leading icon, filter/dismiss interactions returning typed
  events; states per 04 helpers.
- Badge: `components/Badge/README.md` — status/kind pill (1–2 words, tinted) with
  tone variants (neutral/success/warning/error/info per status roles) and a count
  badge on an icon (e.g. 9+ cap logic, unit-tested pure function).
- Gallery section renders every variant × state matrix for the active theme with
  real Tab-focusable instances (not static tiles) so states are live; plus the
  disabled and icon-only variants with aria-style labels.
- Unit tests: size math per size class; badge count cap (99 → "99+", 100 → "100"?
  — follow the spec's rule or the preview, your call documented); chip selected
  state colour resolution per theme; tertiary reserved-accent rule exposed as a
  doc note (not enforceable in code).
- Copy: labels verb-first sentence case per voice ("Start a project", "Read the
  case study"), no emoji/exclamation/em dash.
- `just check/lint/test` green.

## Out of scope for this issue

- Button groups/toolbars; other components (06+).

## Implementation notes

- Spec sources: `components/{Button,Chip,Badge}/README.md` + their `preview.html`
  for exact look details. Tokens from generated modules only.
- House pattern: plain functions or small structs with `fn ui(self, ui: &mut Ui) -> Response`
  like rustnotes-core layout widgets.

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
