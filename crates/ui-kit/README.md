# ui-kit

The Vynatix design system as pure egui: generated tokens for all six
themes, the themes, and the typography stack. The gallery example
(`cargo run -p ui-kit --example gallery`) is the demo surface; each issue
grows a section there.

A full README lands with the polish issue. Until then, this stub documents
the one workflow that is easy to get wrong: the fonts.

## Fonts

`src/typography.rs` embeds the Vynatix faces from `assets/fonts/` with
`include_bytes!`. Those files are prepared, never hand-edited:

```
python3 tools/prepare_fonts.py
```

Run it from this directory (python3 with `fonttools` installed). It
instances the variable sources from `.afk/design-system/fonts/` at the
coordinates the type styles need (the Fraunces variable file defaults to
wght 900 at opsz 9 and is never shipped raw), copies the static sources
verbatim, and writes a size report. The output is byte-deterministic and
committed; builds never run the script. Re-run it only after the
design-system font sources change, and commit the result.

The instanced faces, and what sets them:

| Face | Instance | Set by |
|---|---|---|
| Fraunces-Display | wght 500, opsz 144, SOFT 30 | display, H1, H2 (Forest) |
| Fraunces-Headline | wght 600, opsz 36, SOFT 0 | H3 (Forest) |
| Fraunces-Metric | wght 700, opsz 144, SOFT 30 | the metric figure (56px) and the editorial headline (24px), issue 08 |
| Fraunces-EditorialItalic | wght 400, opsz 22, SOFT 30 | editorial pull-quotes |
| Geist-Regular | wght 400 | body in Teal and Monochrome |
| Geist-Medium | wght 500 | display, H1, H2 in Monochrome |
| Geist-SemiBold | wght 600 | eyebrows; H3, H4 in Teal and Monochrome; the bold display voice in Monochrome |

Aptos (Regular, SemiBold), Instrument Serif (Regular, Italic) and
JetBrains Mono (Regular) are copied verbatim. Every face registers as
its own named family (`src/typography.rs`), since egui has no weight
axis; the two component styles that name weight 700 resolve through
`typography::display_bold_family`, where Teal keeps Instrument Serif's
single weight and Monochrome's Geist takes its 600 instance (documented
deviations in the typography module docs).

Licences sit next to the fonts: the two OFL texts and
`THIRD-PARTY-NOTES.md`, which covers Aptos (Microsoft's typeface, Vynatix
products only) and the Fraunces and Geist OFL sources.
