---
id: 02
title: Typography: instanced self-hosted fonts, type-style system, gallery type section
tag: AFK
priority: high
blocked-by: [01]
blocks: [03, 04, 05, 08, 10]
estimate: medium
status: done
completed: 2026-10-13T21:52:34Z
commit: 85cf7e415323bd6b6e6dd7f5b300fa010d6073cf
modules:
  - crates/ui-kit/tools/prepare_fonts.py
  - crates/ui-kit/assets/fonts/
  - crates/ui-kit/src/typography.rs
  - crates/ui-kit/examples/gallery.rs
---

# Typography: instanced self-hosted fonts, type-style system, gallery type section

## What changes for the user

The gallery's text renders in the real Vynatix faces (Fraunces display, Aptos body,
Geist system sans, JetBrains Mono for code), and a new "Type" gallery section shows
the type specimen (display, headlines, lede, body, editorial italic, eyebrow, mono)
in the active theme. Switching to a Teal theme swaps display/editorial to Instrument
Serif and body to Geist; Monochrome swaps display to Geist. Fonts come from
`assets/fonts/` inside the crate, self-hosted via `include_bytes!`.

## Acceptance criteria

- `tools/prepare_fonts.py` (python3 + fonttools, run manually by the developer, output
  committed) produces the instanced/subset TTFs listed below from
  `.afk/design-system/fonts/`, writing them to `crates/ui-kit/assets/fonts/` with a
  printed size report. The script is deterministic and documented in the crate README
  stub (full README in issue 11).
- Font set (smallest sufficient, per PRD): Fraunces instanced upright
  `wght=500,opsz=144,SOFT=30,WONK=0` (display/H1/H2 — H2's opsz 96 reuses 144,
  documented deviation) and `wght=600,opsz=36,SOFT=0,WONK=0` (H3), Fraunces Italic
  instanced `wght=400,opsz=22,SOFT=30,WONK=0` (editorial); Geist instanced at
  wght 400, 500 and 600; Aptos statics Regular + SemiBold (+ Bold only if a later
  component needs it — then add here); Instrument Serif Regular + Italic as-is;
  JetBrains Mono Regular as-is. Source TTFs for Aptos/Instrument/JetBrains copy
  verbatim. Everything embedded with `include_bytes!`.
- Licence files: copy `InstrumentSerif-OFL.txt` and `JetBrainsMono-OFL.txt` from
  `.afk/design-system/assets/notes/licences/` next to the fonts; write
  `assets/fonts/THIRD-PARTY-NOTES.md` covering Aptos (Microsoft typeface, Vynatix
  products only), Fraunces and Geist (SIL OFL upstream; no licence text shipped in
  the source zip).
- `src/typography.rs`: per-theme face mapping (Forest: display/editorial Fraunces,
  body Aptos; Teal: display/editorial Instrument Serif, body Geist, "H3 and smaller
  in Geist"; Monochrome: display Geist, editorial Fraunces italic, body Geist);
  type-style constructors returning egui `FontId`s + letter-spacing-aware layout
  helpers for: display 96/1.02 tr −4%, headline-1 64/1.08 −3.5%, headline-2 48/1.12
  −3%, headline-3 32/1.18 −2% (600), headline-4 22/1.3 −1% (body face, 600), lede
  22/1.5 (Aptos 350 unavailable as static → Regular 400, documented deviation), body
  17/1.65, editorial italic 22/1.55, eyebrow 11/16 w600 +0.24em uppercase (tracked
  via per-character advance or a galley trick — egui has no letter-spacing; implement
  by painting per glyph with added advance, or the closest faithful approach you can
  test; document the approach), caption 12/1.5, mono 14/1.6. All values from
  `tokens.json` `type.groups` / `typeScale` / `leading` / `tracking`.
- Font installation API: one call installs all families for a given theme into an
  egui `Context` (FontDefinitions with named custom families) without losing the
  icon family installed by issue 03 later (design the install to be additive/merged).
- Gallery "Type" section renders the specimen with per-theme faces, including an
  eyebrow line ("Materials · Type · 2026" style metadata with middle dots), a
  Fraunces italic pull-quote, and a mono code line. Copy passes the voice rules.
- Unit tests: type-style table matches the token values (sizes, weights, families
  per theme); every theme maps every style to an installed family name; eyebrow
  tracking constant = 0.24em; display-to-body ratio invariant helper exposes
  96/17 ≥ 2.5 check for the pairing rule.
- `just check/lint/test` green.

## Out of scope for this issue

- Icons (03). Display minimum-size/2.5x enforcement in components (later issues).
- README final polish (11).

## Implementation notes

- Verify instances with `fontTools.varLib.instancer` (e.g.
  `fonttools varLib.instancer Fraunces-VariableFont.ttf wght=500 opsz=144 SOFT=30 WONK=0 -o out.ttf`).
  The Fraunces default instance is wght 900/opsz 9 — never ship the raw variable file.
- egui weights: there is no weight axis; each weight is its own named family
  (`ui-kit` family enum or `FontFamily::Name` constants).
- Letter spacing: egui `Layout` has no tracking; the eyebrow needs it (+0.24em is
  the signature). Simplest faithful approach: custom widget that advances each
  glyph manually. Keep it small and tested (total advance = sum advances + n·0.24em·size).

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
