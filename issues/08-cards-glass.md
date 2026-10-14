---
id: 08
title: Components: Card, MetricCard, EditorialCard, LiquidGlass with gallery section
tag: AFK
priority: high
blocked-by: [04]
blocks: [09, 10, 11]
estimate: medium
status: done
completed: 2026-10-14T01:00:34+02:00
commit: b5a69d1dcb55ea75a1c2891a5416977725fb0d7b
modules:
  - crates/ui-kit/src/components/card.rs
  - crates/ui-kit/src/components/metric_card.rs
  - crates/ui-kit/src/components/editorial_card.rs
  - crates/ui-kit/src/components/liquid_glass.rs
  - crates/ui-kit/examples/gallery.rs
---

# Components: Card, MetricCard, EditorialCard, LiquidGlass with gallery section

## What changes for the user

A "Components: surfaces" gallery section: the four card variants from the Card spec
(rest/hover elevation lift, outlined variant), a MetricCard trio with real
falsifiable numbers ("47 minutes to 6" style, per the brand voice), an EditorialCard
with placeholder image tile (muted palette tint per the photography rules, no
invented illustration), eyebrow, Fraunces headline and summary; and LiquidGlass
panes in all five thicknesses with the specular edge, plus the
reduced-transparency fallback.

## Acceptance criteria

- Card per `components/Card/README.md`: surface-container-low at rest →
  surface-container + shadow L1→L2 on hover for interactive cards, radius-lg,
  1px outline-variant border on the outlined variant, paddings 24/16/32 from the
  spacing tokens. Content-agnostic container widget.
- MetricCard per `components/MetricCard/README.md`: label (11px w600 uppercase
  tracked +18% in text-accent — reuse the eyebrow machinery from 02 with a
  +0.18em variant), figure (display face 56px w700 tr −4% in text-display — note:
  Fraunces 700 not in the 02 instance set; instance a `wght=700,opsz=144` Fraunces
  in this issue's prep script run and document it), unit span smaller, context
  line 13px text-muted. Real numbers only.
- EditorialCard per `components/EditorialCard/README.md`: image area as a muted
  placeholder tile tinted to the palette (per README photography rules), eyebrow,
  display headline (respecting ≥2.5× body pairing), summary, optional action.
- LiquidGlass per `components/LiquidGlass/README.md` + README §"Transparency,
  blur, Liquid Glass": five thicknesses (ultra-thin/thin/regular/thick/chrome) as
  tint+opacity recipes from the glass tint tokens, MANDATORY 1px specular edge
  (inset top highlight per glass-specular tokens), shadow L2, chrome-and-overlays
  usage note in docs; **documented deviation: no backdrop blur in egui** — semi-
  transparent tint approximation, and under `reduced_transparency` the pane turns
  solid surface-container with no edge. Never stack two same-thickness panes
  (doc note).
- Unit tests: card padding/radius/elevation lookups per variant; glass recipes
  resolve per theme with the specular alpha from tokens; metric label style
  (+0.18em, 11px) and figure style per theme faces; editorial card body/display
  ratio ≥ 2.5 check with the chosen styles.
- Gallery section per the above, voice-clean copy, real-ish Vynatix-flavoured
  content (Android Automotive, Gothenburg, middle-dot metadata separators).
- `just check/lint/test` green.

## Out of scope for this issue

- TopAppBar (09) — it consumes glass from here. Real photography (none exists).
- Cover/Themes showcases (10).

## Implementation notes

- Placeholder image tile: flat tinted rect from palette constants (allowed for
  mocks per README) — clearly a placeholder, no invented SVG art.
- If Fraunces 700 at 56px measures much larger than sibling type, that's expected:
  it's the display voice.

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
