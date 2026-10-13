#!/usr/bin/env python3
"""Prepare the Vynatix faces under `assets/fonts/` from the design-system fonts.

Reads the source TTFs from `../../../.afk/design-system/fonts/` (relative to
the crate directory; the script also resolves them from its own location, so
any working directory works) and writes the self-hosted font set the kit
embeds with `include_bytes!`:

  Fraunces-Display.ttf           Fraunces upright instanced at
                                 wght 500, opsz 144, SOFT 30, WONK 0
                                 (display, H1, H2; H2's opsz 96 reuses the
                                 144 instance, a documented deviation)
  Fraunces-Headline.ttf          Fraunces upright instanced at
                                 wght 600, opsz 36, SOFT 0, WONK 0 (H3)
  Fraunces-EditorialItalic.ttf   Fraunces italic instanced at
                                 wght 400, opsz 22, SOFT 30, WONK 0
                                 (editorial pull-quotes)
  Geist-Regular.ttf              Geist instanced at wght 400
  Geist-Medium.ttf               Geist instanced at wght 500
                                 (Monochrome display, H1, H2)
  Geist-SemiBold.ttf             Geist instanced at wght 600
                                 (eyebrows; H3, H4 in Teal and Monochrome)
  Aptos.ttf                      Aptos Regular, copied verbatim (body, lede,
                                 caption in Forest)
  Aptos-SemiBold.ttf             Aptos SemiBold, copied verbatim (H4 in
                                 Forest)
  InstrumentSerif-Regular.ttf    Instrument Serif Regular, copied verbatim
                                 (Teal display, H1, H2)
  InstrumentSerif-Italic.ttf     Instrument Serif Italic, copied verbatim
                                 (Teal editorial)
  JetBrainsMono-Regular.ttf      JetBrains Mono Regular, copied verbatim
                                 (the mono style, every theme)

Plus the licence set: `InstrumentSerif-OFL.txt` and `JetBrainsMono-OFL.txt`
copied from `.afk/design-system/assets/notes/licences/`, and
`THIRD-PARTY-NOTES.md` covering Aptos, Fraunces and Geist.

The variable sources are never shipped: Fraunces defaults to wght 900 at
opsz 9 with WONK 1, and egui has no variation-axis control, so every
variable face is pinned to one named instance per style. Character sets
are kept whole (no glyph subsetting): Swedish text needs the full Latin
extended ranges and the size win comes from dropping the variation tables.

The output is deterministic (timestamps are not recalculated) and
committed; builds never run this script. After the design-system fonts
change, regenerate from the crate directory and commit the result:

  python3 tools/prepare_fonts.py

Requires python3 with fonttools (`pip install fonttools`); instancing
itself pulls in no compiler extras.
"""

import shutil
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[3]
SOURCE_DIR = ROOT / ".afk" / "design-system" / "fonts"
LICENCE_DIR = ROOT / ".afk" / "design-system" / "assets" / "notes" / "licences"
OUT_DIR = Path(__file__).resolve().parents[1] / "assets" / "fonts"

# The named instances: (output name, source file, axis coordinates, family
# name, subfamily name, note). The name records are rewritten after
# instancing (updateFontNames needs STAT AxisValues the sources do not
# carry), so a designer opening the file sees the instance it is, not the
# wght 900 default the source shipped as. WONK is pinned to 0 everywhere:
# the wonky alternate glyphs are a display flavour the brand specimens do
# not use.
INSTANCES = [
    (
        "Fraunces-Display.ttf",
        "Fraunces-VariableFont_SOFT_WONK_opsz_wght.ttf",
        {"wght": 500, "opsz": 144, "SOFT": 30, "WONK": 0},
        "Fraunces Display",
        "Medium",
        "display, headline 1 and headline 2",
    ),
    (
        "Fraunces-Headline.ttf",
        "Fraunces-VariableFont_SOFT_WONK_opsz_wght.ttf",
        {"wght": 600, "opsz": 36, "SOFT": 0, "WONK": 0},
        "Fraunces Headline",
        "SemiBold",
        "headline 3",
    ),
    (
        "Fraunces-EditorialItalic.ttf",
        "Fraunces-Italic-VariableFont_SOFT_WONK_opsz_wght.ttf",
        {"wght": 400, "opsz": 22, "SOFT": 30, "WONK": 0},
        "Fraunces Editorial",
        "Italic",
        "editorial italic",
    ),
    (
        "Geist-Regular.ttf",
        "Geist-VariableFont_wght.ttf",
        {"wght": 400},
        "Geist",
        "Regular",
        "body in Teal and Monochrome",
    ),
    (
        "Geist-Medium.ttf",
        "Geist-VariableFont_wght.ttf",
        {"wght": 500},
        "Geist",
        "Medium",
        "display, headline 1 and 2 in Monochrome",
    ),
    (
        "Geist-SemiBold.ttf",
        "Geist-VariableFont_wght.ttf",
        {"wght": 600},
        "Geist",
        "SemiBold",
        "eyebrows; headline 3 and 4 in Teal and Monochrome",
    ),
]

# The verbatim copies: (output name, source file, note). Static sources
# ship exactly as the design system hosts them.
COPIES = [
    ("Aptos.ttf", "Aptos.ttf", "body, lede and caption in Forest"),
    ("Aptos-SemiBold.ttf", "Aptos-SemiBold.ttf", "headline 4 in Forest"),
    ("InstrumentSerif-Regular.ttf", "InstrumentSerif-Regular.ttf", "Teal display faces"),
    ("InstrumentSerif-Italic.ttf", "InstrumentSerif-Italic.ttf", "Teal editorial"),
    ("JetBrainsMono-Regular.ttf", "JetBrainsMono-Regular.ttf", "the mono style, every theme"),
]

LICENCES = ["InstrumentSerif-OFL.txt", "JetBrainsMono-OFL.txt"]

THIRD_PARTY_NOTES = """\
# Third-party font notes

The faces under `assets/fonts/` are prepared by `tools/prepare_fonts.py`
from the Vynatix design-system font sources and embedded with
`include_bytes!`.

- **Aptos** (Aptos.ttf, Aptos-SemiBold.ttf): Microsoft's typeface. It may
  ship in Vynatix products only; check Microsoft's terms before
  distributing it outside Vynatix's own documents and apps.
- **Fraunces** (Fraunces-Display.ttf, Fraunces-Headline.ttf,
  Fraunces-EditorialItalic.ttf): SIL Open Font License upstream
  (underdale.org). The source zip the design system hosts carried no
  licence text; see https://openfontlicense.org for the terms.
- **Geist** (Geist-Regular.ttf, Geist-Medium.ttf, Geist-SemiBold.ttf):
  SIL Open Font License upstream (Vercel). No licence text shipped in
  the source download; see https://openfontlicense.org for the terms.
- **Instrument Serif** (InstrumentSerif-Regular.ttf,
  InstrumentSerif-Italic.ttf): SIL Open Font License; the full licence
  text sits next to this file as `InstrumentSerif-OFL.txt`.
- **JetBrains Mono** (JetBrainsMono-Regular.ttf): SIL Open Font License;
  the full licence text sits next to this file as
  `JetBrainsMono-OFL.txt`.

The instanced outputs are derivatives of their OFL sources and keep the
OFL terms: they may be used, embedded and redistributed under the same
licence, and none of them may be sold by itself.
"""


def die(message: str) -> None:
    raise SystemExit(f"prepare_fonts.py: {message}")


def human(size: int) -> str:
    return f"{size / 1024:.0f} KB"


def set_names(font: TTFont, family: str, subfamily: str) -> None:
    """Rewrite the family, subfamily, full and PostScript names on both
    platform records, so the instanced file describes itself, not the
    source's default instance."""
    full = family if subfamily == "Regular" else f"{family} {subfamily}"
    postscript = full.replace(" ", "")
    names = font["name"]
    for platform, encoding, language in ((3, 1, 0x409), (1, 0, 0)):
        names.setName(family, 1, platform, encoding, language)
        names.setName(subfamily, 2, platform, encoding, language)
        names.setName(full, 4, platform, encoding, language)
        names.setName(postscript, 6, platform, encoding, language)


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    report: list[tuple[str, str, int]] = []

    for out_name, source_name, axes, family, subfamily, note in INSTANCES:
        source = SOURCE_DIR / source_name
        if not source.is_file():
            die(f"missing source font: {source}")
        # recalcTimestamp=False keeps the output byte-deterministic: the
        # head table's modified stamp stays the source's, and instancing
        # itself touches no clock.
        font = TTFont(source, recalcTimestamp=False)
        instancer.instantiateVariableFont(font, axes, inplace=True)
        set_names(font, family, subfamily)
        out = OUT_DIR / out_name
        font.save(out)
        report.append((out_name, f"{source_name} at {axes}", out.stat().st_size))
        print(f"  instanced {out_name:<34} <- {source_name} {axes}")

    for out_name, source_name, note in COPIES:
        source = SOURCE_DIR / source_name
        if not source.is_file():
            die(f"missing source font: {source}")
        out = OUT_DIR / out_name
        shutil.copyfile(source, out)
        report.append((out_name, source_name, out.stat().st_size))
        print(f"  copied    {out_name:<34} <- {source_name}")

    for licence_name in LICENCES:
        source = LICENCE_DIR / licence_name
        if not source.is_file():
            die(f"missing licence file: {source}")
        shutil.copyfile(source, OUT_DIR / licence_name)
        print(f"  licence   {licence_name:<34} <- assets/notes/licences/")

    notes = OUT_DIR / "THIRD-PARTY-NOTES.md"
    notes.write_text(THIRD_PARTY_NOTES, encoding="utf-8")
    print(f"  wrote     THIRD-PARTY-NOTES.md")

    print()
    print("Size report")
    print("  file                                size     derived from")
    print("  ----------------------------------  -------  ------------------------------")
    total = 0
    for out_name, derivation, size in report:
        total += size
        print(f"  {out_name:<34} {human(size):>7}  {derivation}")
    print(f"  {'11 faces, total':<34} {human(total):>7}")
    print()
    print(f"Wrote {len(report)} faces, {len(LICENCES)} licences and the notes to {OUT_DIR}")
    print("The output is committed; builds embed it with include_bytes!.")


if __name__ == "__main__":
    main()
