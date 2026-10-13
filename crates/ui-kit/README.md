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

Licences sit next to the fonts: the two OFL texts and
`THIRD-PARTY-NOTES.md`, which covers Aptos (Microsoft's typeface, Vynatix
products only) and the Fraunces and Geist OFL sources.
