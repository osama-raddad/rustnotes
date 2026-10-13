---
id: 04
title: Foundations: interaction states, FocusRing widget, gallery states section
tag: AFK
priority: high
blocked-by: [02]
blocks: [05, 06, 07, 08, 09]
estimate: medium
status: done
completed: 2026-10-13T22:09:19Z
commit: 66f2f4dea0fbcac6c7271ac6feb0abb2e4492be7
modules:
  - crates/ui-kit/src/states.rs
  - crates/ui-kit/src/components/focus_ring.rs
  - crates/ui-kit/examples/gallery.rs
---

# Foundations: interaction states, FocusRing widget, gallery states section

## What changes for the user

A new "States" gallery section shows a focus demo: buttons-as-rectangles cycling
through hover (8% on-colour overlay), pressed (12% overlay + 98.5% scale), keyboard
focus (2px accent ring, 2px offset, canvas halo) and disabled (38% content / 12%
container opacity), next to a control that really takes focus via Tab so the ring
is observable live. Shadow levels L1–L5 and the radii scale render as specimen tiles
beside the spacing scale.

## Acceptance criteria

- `src/states.rs` with typed, unit-tested helpers:
  - `hover_overlay(on: Color32) -> Color32` — 8% on-colour overlay (per M3: composite
    `on` at 8% alpha over the fill);
  - `pressed_overlay(on: Color32)` — 12%;
  - `disabled(content, container)` — content at 38% opacity, container at 12%;
  - press scale 0.985 as a const with the spec citation;
  - a `WidgetState`/`Interaction` query helper that maps an egui `Response` to
    (hovered, pressed/active, has_focus, disabled) consuming egui's built-in
    interaction sensing.
- Focus ring painting: `focus_ring(painter, rect, theme)` draws the 2px `focus-ring`
  role outline at 2px offset plus the 4px canvas-coloured halo (`surface` role)
  under it, per spec; FocusRing component (one of the 22) exposes it as a wrapper
  that paints around any widget's response. Ring animates in ~120ms where egui
  timing data is available (ctx.input time), static otherwise (documented).
- Focus visuals must also work for egui's own widgets: a theme-level install that
  sets egui focus styling to the brand ring where egui exposes it, and the kit's
  own widgets all use the helper (enforced by later component issues).
- Reduced-motion/transparency kit flags: `UiKitFlags { reduced_motion, reduced_transparency }`
  stored as egui pass-through (e.g. via `Context` extension or a plain struct the
  gallery hosts); helpers consult them: reduced motion swaps spatial transitions for
  opacity fades ≤ 100ms, reduced transparency swaps glass/tints for solid tokens
  (used by 08/09). Default false. Documented deviation for the missing media query.
- Gallery "States" section + "Foundations" specimens: spacing scale steps, all 8
  radii, shadow L1–L5 tiles per theme (tinted shadows from the generated tokens),
  focus/hover/press/disabled demo tiles. Voice-clean captions.
- Unit tests: overlay math exactness for known inputs (8% of cream over forest =
    expected composite), disabled alphas, press-scale const, ring geometry
  (outer rect = widget rect expanded by 4px total, stroke width 2), flags default.
- `just check/lint/test` green.

## Out of scope for this issue

- Real components using the helpers (05+).
- Motion curves beyond what egui exposes (documented; no animation loop added).

## Implementation notes

- Spec source: `.afk/design-system/README.md` §"Hover, press, focus", §"Elevation",
  §"Shadows", §"Spacing", §"Radii"; `components/FocusRing/README.md`.
- egui alpha compositing: use `Color32::linear_multiply` carefully — the spec's
  percentages are in sRGB space; pick the composite that visually matches the web
  behaviour (rgba over) and test the math, not the gamma.
- egui has `response.has_focus()`; Tab-order focus works for buttons/text edits.

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
