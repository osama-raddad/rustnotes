#!/usr/bin/env python3
"""Prepare the Vynatix icon fonts and logo geometry from the design system.

Two jobs, one run (python3 + fonttools + brotli; output committed, builds
never run this):

Fonts. Reads the variable `.afk/design-system/fonts/
MaterialSymbolsOutlined.woff2` (npm material-symbols 0.47.5, Apache 2.0),
resolves every catalogue name (`.afk/design-system/assets/Icons/
catalogue.json`) to its private-use codepoint through the font's cmap,
then instances and subsets four static TTFs into `assets/fonts/`:

  MaterialSymbols-Outline.ttf       wght 300, opsz 24, GRAD 0,   FILL 0
  MaterialSymbols-OutlineDark.ttf   wght 300, opsz 24, GRAD -25, FILL 0
  MaterialSymbols-Fill.ttf          wght 300, opsz 24, GRAD 0,   FILL 1
  MaterialSymbols-FillDark.ttf      wght 300, opsz 24, GRAD -25, FILL 1

Instancing runs before subsetting so the pinned coordinates land on the
full glyph set, then the subset keeps exactly the catalogue codepoints.
Sizes print at the end; if the four exceed ~1.5 MB total the grade
distinction drops (the dark files become copies of the GRAD 0 instances)
and the deviation is recorded in the generated header. Deviations pinned
here: opsz is 24 in every subset (egui has no variation-axis control, so
the per-size opsz the tokens describe ships as the single default
instance); `location_on` has no glyph in font 0.47.5, so the generated
enum aliases it to `push_pin`, the catalogue's pin (see src/icons.rs).

Generated code. Writes `src/icons.rs`: the `Icon` enum over the whole
catalogue (snake_case names as PascalCase variants), `as_char()` from the
cmap, `has_fill()` from the catalogue's fill flags, `CATEGORY_GROUPS`
for the gallery, and the catalogue name list and count the unit tests
pin against.

Logo. Parses the V mark and wordmark SVGs from `.afk/design-system/
assets/Logos/` ONCE: asserts every per-ink file shares the same path
data (they do; colour is applied at paint time in the kit), flattens the
Bezier contours at a documented tolerance (1/1000 of the art box width),
triangulates them by even-odd scanline decomposition (robust to the
exported paths' hair folds, where the outline doubles back on itself,
and to their pinched duplicate vertices; the script verifies the mesh
against point-sampling the artwork before emitting), and writes
normalized triangle constants to `src/logo/paths.rs`: the art boxes are
scaled to 1000 units tall, y down, origin at the box's top-left, so the
Rust side scales by height/1000 and the aspect is preserved by
construction.

Also copies the icon licence beside the fonts and the two source logo
SVGs under `assets/logos/source/` for provenance. The output is
deterministic (no timestamps recalculated); regenerate from the crate
directory after the design system changes and commit the result, then
run rustfmt over the generated tables (the same workflow
`tools/generate_tokens.py` documents):

  python3 tools/prepare_icons.py && cargo fmt
"""

import hashlib
import json
import math
import re
import shutil
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[3]
DS = ROOT / ".afk" / "design-system"
FONT_SOURCE = DS / "fonts" / "MaterialSymbolsOutlined.woff2"
CATALOGUE = DS / "assets" / "Icons" / "catalogue.json"
ICON_LICENCE = DS / "assets" / "Icons" / "LICENSE.txt"
LOGO_DIR = DS / "assets" / "Logos"
CRATE = Path(__file__).resolve().parents[1]
FONTS_OUT = CRATE / "assets" / "fonts"
LOGOS_OUT = CRATE / "assets" / "logos"
ICONS_RS = CRATE / "src" / "icons.rs"
PATHS_RS = CRATE / "src" / "logo" / "paths.rs"

# The pinned axis coordinates: the brand's one weight, the default
# optical size, and the two grades at both fill states. opsz 24 is the
# axis default; pinning it anyway keeps the instance honest if the
# source changes.
VARIANTS = [
    # (file, family, axes)
    ("MaterialSymbols-Outline.ttf", "Vynatix Icons", {"wght": 300, "opsz": 24, "GRAD": 0, "FILL": 0}),
    ("MaterialSymbols-OutlineDark.ttf", "Vynatix Icons Dark", {"wght": 300, "opsz": 24, "GRAD": -25, "FILL": 0}),
    ("MaterialSymbols-Fill.ttf", "Vynatix Icons Fill", {"wght": 300, "opsz": 24, "GRAD": 0, "FILL": 1}),
    ("MaterialSymbols-FillDark.ttf", "Vynatix Icons Fill Dark", {"wght": 300, "opsz": 24, "GRAD": -25, "FILL": 1}),
]
GRADE_DROP_PLAN = [
    ("MaterialSymbols-Outline.ttf", "Vynatix Icons", {"wght": 300, "opsz": 24, "GRAD": 0, "FILL": 0}),
    ("MaterialSymbols-Fill.ttf", "Vynatix Icons Fill", {"wght": 300, "opsz": 24, "GRAD": 0, "FILL": 1}),
]
SIZE_BUDGET = 1.5 * 1024 * 1024

# `location_on` has no glyph in font 0.47.5; alias it to the visually
# closest catalogue sibling the font does carry.
MISSING_ALIAS = {"location_on": "push_pin"}

LOGO_INKS = [
    "bronze", "champagne", "charcoal", "cloud", "cream",
    "forest", "ink", "paper", "teal",
]

DO_NOT_EDIT = "DO NOT EDIT: rerun tools/prepare_icons.py,"


def die(message: str) -> None:
    raise SystemExit(f"prepare_icons.py: {message}")


def human(size: int) -> str:
    return f"{size / 1024:.0f} KB"


def set_names(font: TTFont, family: str) -> None:
    """Name the subset as the kit family it installs as, on both platform
    records (same approach as prepare_fonts.py)."""
    names = font["name"]
    for platform, encoding, language in ((3, 1, 0x409), (1, 0, 0)):
        names.setName(family, 1, platform, encoding, language)
        names.setName("Regular", 2, platform, encoding, language)
        names.setName(family, 4, platform, encoding, language)
        names.setName(family.replace(" ", ""), 6, platform, encoding, language)


# ---- the icon pipeline -----------------------------------------------------


def build_font(out_name: str, family: str, axes: dict, unicodes: list[int]) -> int:
    """Instance the variable source at `axes`, subset it to the catalogue
    codepoints, name it, and write it as a TTF. Returns the file size."""
    from fontTools.subset import Options, Subsetter

    font = TTFont(FONT_SOURCE, recalcTimestamp=False)
    instancer.instantiateVariableFont(font, axes, inplace=True)
    options = Options()
    options.layout_features = []
    options.glyph_names = False
    options.name_IDs = [1, 2, 3, 4, 6]
    options.name_languages = [0x409]
    options.drop_tables += ["GSUB", "GPOS"]
    options.notdef_outline = True
    subsetter = Subsetter(options=options)
    subsetter.populate(unicodes=unicodes)
    subsetter.subset(font)
    set_names(font, family)
    font.flavor = None
    out = FONTS_OUT / out_name
    font.save(out)
    return out.stat().st_size


def generate_fonts(report: list[tuple[str, str, int]]) -> dict[str, int]:
    catalogue = json.loads(CATALOGUE.read_text(encoding="utf-8"))
    names = [entry["name"] for entry in catalogue["icons"]]

    source = TTFont(FONT_SOURCE, recalcTimestamp=False)
    cmap = source.getBestCmap()
    name_to_cp = {glyph: cp for cp, glyph in cmap.items()}

    missing = [name for name in names if name not in name_to_cp]
    for name in missing:
        alias = MISSING_ALIAS.get(name)
        if alias is None or alias not in name_to_cp:
            die(f"{name} has no glyph and no resolvable alias")

    # codepoint per catalogue name; the alias borrows its sibling's.
    codepoints = [name_to_cp[MISSING_ALIAS.get(name, name)] for name in names]
    unique = sorted(set(codepoints))
    if len(unique) != len(set(n for n in names if n not in MISSING_ALIAS)):
        die("aliasing collapsed two catalogue icons onto one codepoint")

    FONTS_OUT.mkdir(parents=True, exist_ok=True)
    sizes: dict[str, int] = {}
    for out_name, family, axes in VARIANTS:
        sizes[out_name] = build_font(out_name, family, axes, unique)
        # The subset must address every catalogue codepoint: reopen and
        # check the cmap, so a subsetting regression cannot ship.
        subset = TTFont(FONTS_OUT / out_name, recalcTimestamp=False)
        carried = set(subset.getBestCmap())
        dropped = [hex(cp) for cp in unique if cp not in carried]
        if dropped:
            die(f"{out_name} dropped catalogue codepoints: {dropped}")

    total = sum(sizes.values())
    dropped_grades = False
    if total > SIZE_BUDGET:
        # Drop the grade distinction: rebuild the two fill states at
        # GRAD 0 only and copy them over the dark slots, so the four
        # families the kit installs stay addressable (the dark ones just
        # carry the same grade as the light ones). Recorded in the
        # generated header.
        dropped_grades = True
        print(
            f"  four subsets total {human(total)} over the "
            f"{human(int(SIZE_BUDGET))} budget: dropping the grade distinction"
        )
        for out_name, family, axes in GRADE_DROP_PLAN:
            sizes[out_name] = build_font(out_name, family, axes, unique)
        for dark_name in ("MaterialSymbols-OutlineDark.ttf", "MaterialSymbols-FillDark.ttf"):
            light_name = dark_name.replace("Dark", "")
            shutil.copyfile(FONTS_OUT / light_name, FONTS_OUT / dark_name)
            sizes[dark_name] = (FONTS_OUT / dark_name).stat().st_size
        total = sum(sizes.values())

    report.clear()
    report.extend(
        (out_name, f"MaterialSymbolsOutlined at {axes}", sizes[out_name])
        for out_name, _, axes in VARIANTS
    )

    shutil.copyfile(ICON_LICENCE, FONTS_OUT / "MaterialSymbolsOutlined-LICENSE.txt")

    return {
        "names": names,
        "codepoints": dict(zip(names, codepoints)),
        "total": total,
        "dropped_grades": dropped_grades,
    }


# ---- the logo pipeline -----------------------------------------------------

Point = tuple[float, float]


def parse_path_data(d: str, tol: float) -> list[list[Point]]:
    """Parse an SVG path with absolute M/L/C/Z into closed subpaths of
    sample points, flattening cubics adaptively at `tol`. Anything else
    (arcs, relative commands) is a hard error: the source files use
    exactly these four."""
    tokens = re.findall(r"[MLCZ]|[-+]?\d*\.?\d+(?:[eE][-+]?\d+)?", d)
    subpaths: list[list[Point]] = []
    current: list[Point] = []
    start: Point = (0.0, 0.0)
    pen: Point = (0.0, 0.0)
    i = 0

    def flatten_cubic(p0: Point, p1: Point, p2: Point, p3: Point, out: list[Point]) -> None:
        def dist(p: Point, a: Point, b: Point) -> float:
            ax, ay = a[0] - p[0], a[1] - p[1]
            bx, by = b[0] - p[0], b[1] - p[1]
            cross = ax * by - ay * bx
            denom = math.hypot(bx, by)
            if denom < 1e-12:
                return math.hypot(ax, ay)
            return abs(cross) / denom

        if dist(p1, p0, p3) <= tol and dist(p2, p0, p3) <= tol:
            out.append(p3)
            return
        mid = lambda a, b: ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)  # noqa: E731
        m01, m12, m23 = mid(p0, p1), mid(p1, p2), mid(p2, p3)
        m012, m123 = mid(m01, m12), mid(m12, m23)
        m = mid(m012, m123)
        flatten_cubic(p0, m01, m012, m, out)
        flatten_cubic(m, m123, m23, p3, out)

    while i < len(tokens):
        command = tokens[i]
        i += 1
        if command == "M":
            if current:
                subpaths.append(current)
                current = []
            x, y = float(tokens[i]), float(tokens[i + 1])
            i += 2
            start = pen = (x, y)
            current.append(pen)
            # Implicit linetos may follow.
            while i < len(tokens) and tokens[i] not in "MLCZ":
                x, y = float(tokens[i]), float(tokens[i + 1])
                i += 2
                pen = (x, y)
                current.append(pen)
        elif command == "L":
            x, y = float(tokens[i]), float(tokens[i + 1])
            i += 2
            pen = (x, y)
            current.append(pen)
            while i < len(tokens) and tokens[i] not in "MLCZ":
                x, y = float(tokens[i]), float(tokens[i + 1])
                i += 2
                pen = (x, y)
                current.append(pen)
        elif command == "C":
            while i < len(tokens) and tokens[i] not in "MLCZ":
                nums = [float(t) for t in tokens[i : i + 6]]
                if len(nums) < 6:
                    die("truncated cubic in path data")
                i += 6
                c1, c2, end = (nums[0], nums[1]), (nums[2], nums[3]), (nums[4], nums[5])
                flatten_cubic(pen, c1, c2, end, current)
                pen = end
        elif command == "Z":
            if current:
                subpaths.append(current)
                current = []
                pen = start
        else:
            die(f"unsupported path command {command!r} in logo SVG")
    if current:
        subpaths.append(current)
    return [ring for ring in subpaths if len(ring) >= 3]


def signed_area(ring: list[Point]) -> float:
    area = 0.0
    previous = ring[-1]
    for p in ring:
        area += previous[0] * p[1] - p[0] * previous[1]
        previous = p
    return area / 2.0


def point_in_ring(p: Point, ring: list[Point]) -> bool:
    """Ray casting, half-open at y: consistent for containment trees."""
    inside = False
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        if (a[1] > p[1]) != (b[1] > p[1]):
            t = (p[1] - a[1]) / (b[1] - a[1])
            if p[0] < a[0] + t * (b[0] - a[0]):
                inside = not inside
    return inside


def even_odd_contains(p: Point, rings: list[list[Point]]) -> bool:
    """Even-odd containment across a set of rings: the fill rule the
    scanline triangulator realizes (and, for this artwork, what the
    source's nonzero rule paints too: counters are wound against their
    bowls, subpaths do not overlap within an element)."""
    crossings = 0
    for ring in rings:
        if point_in_ring(p, ring):
            crossings += 1
    return crossings % 2 == 1


def triangle_contains(p: Point, a: Point, b: Point, c: Point) -> bool:
    """Closed containment, for verification sampling."""
    def cross(u: Point, v: Point, w: Point) -> float:
        return (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])

    d1, d2, d3 = cross(a, b, p), cross(b, c, p), cross(c, a, p)
    has_neg = d1 < 0 or d2 < 0 or d3 < 0
    has_pos = d1 > 0 or d2 > 0 or d3 > 0
    return not (has_neg and has_pos)


def triangulate_element(rings: list[list[Point]], min_area: float):
    """Triangulate one SVG path element's rings by even-odd scanline
    decomposition: one band between consecutive distinct vertex ys,
    inside spans paired by parity, each span a trapezoid cut into two
    triangles. Robust to the exported artwork's degenerate features
    (hair folds where the outline doubles back on itself, pinched
    duplicate vertices), which plain ear clipping is not; slivers under
    `min_area` drop away with the folds that cause them."""
    edges: list[tuple[Point, Point]] = []
    for ring in rings:
        for i in range(len(ring)):
            a, b = ring[i], ring[(i + 1) % len(ring)]
            if a[1] == b[1]:
                continue  # exactly horizontal edges never cross a band
            edges.append((a, b) if a[1] < b[1] else (b, a))
    ys = sorted({p[1] for ring in rings for p in ring})
    triangles: list[tuple[Point, Point, Point]] = []
    for y0, y1 in zip(ys, ys[1:]):
        if y1 - y0 <= 0.0:
            continue
        ymid = (y0 + y1) / 2.0
        # Each spanning edge contributes its x at both band boundaries:
        # the span between a pair of edges is then an exact trapezoid,
        # not a mid-band rectangle (the artwork's tall diagonal strokes
        # make that difference visible).
        crossings: list[tuple[float, float, float]] = []
        for a, b in edges:
            if a[1] <= ymid < b[1]:
                span = b[1] - a[1]
                t0 = (y0 - a[1]) / span
                t1 = (y1 - a[1]) / span
                tm = (ymid - a[1]) / span
                crossings.append(
                    (
                        a[0] + tm * (b[0] - a[0]),  # sort key: x at mid-band
                        a[0] + t0 * (b[0] - a[0]),  # x at y0
                        a[0] + t1 * (b[0] - a[0]),  # x at y1
                    )
                )
        if not crossings:
            continue
        crossings.sort()
        if len(crossings) % 2 != 0:
            die("scanline crossings do not pair: the ring set is degenerate")
        for (_, xl0, xl1), (_, xr0, xr1) in zip(
            crossings[::2], crossings[1::2]
        ):
            top_l, top_r = (xl0, y0), (xr0, y0)
            bot_l, bot_r = (xl1, y1), (xr1, y1)
            # A wedge span (a stroke tapering to a tip, top or bottom
            # width zero) yields one degenerate half: emit each triangle
            # on its own rounded area, so no zero-area triple ships.
            for triangle in ((top_l, top_r, bot_r), (top_l, bot_r, bot_l)):
                (a, b, c) = triangle
                area = abs(
                    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
                ) / 2.0
                if area >= 2.0 * min_area:
                    triangles.append(triangle)
    return triangles


def triangulate_art(elements: list[list[list[Point]]], eps: float, box: tuple[float, float, float, float]):
    """Triangulate the artwork one SVG path element at a time (the fill
    rule applies within a path element; overlapping elements union when
    painted the same colour), then verify the union of triangles
    against even-odd sampling of the source rings."""
    triangles: list[tuple[Point, Point, Point]] = []
    for rings in elements:
        triangles.extend(triangulate_element(rings, eps * eps))

    import random

    random.seed(4)  # fixed: the verification is deterministic
    x, y, w, h = box
    samples = 20000
    mismatches = 0
    counted = 0
    while counted < samples:
        p = (x + w * random.random(), y + h * random.random())
        expected = any(even_odd_contains(p, rings) for rings in elements)
        painted = any(
            triangle_contains(p, *triangle)
            for triangle in triangles
        )
        # Points near a boundary misclassify either way: skip the 0.5%
        # margin by only counting clear disagreements loosely.
        if expected != painted:
            mismatches += 1
        counted += 1
    if mismatches > 0.005 * samples:
        die(
            f"triangulated mesh disagrees with the artwork on "
            f"{mismatches} of {samples} samples"
        )
    return triangles


def load_logo(kind: str) -> tuple[list[list[list[Point]]], tuple[float, float, float, float]]:
    """Read one logo across its inks: assert the shared geometry, parse
    one file at the tolerance its own box dictates, return (path
    elements, viewBox)."""
    digests = set()
    for ink in LOGO_INKS:
        text = (LOGO_DIR / f"vynatix-{kind}-{ink}.svg").read_text(encoding="utf-8")
        digests.add(
            hashlib.sha256(
                "|".join(re.findall(r'<path d="([^"]+)"', text)).encode()
            ).hexdigest()
        )
    if len(digests) != 1:
        die(f"the {kind} inks disagree on path data")

    text = (LOGO_DIR / f"vynatix-{kind}-forest.svg").read_text(encoding="utf-8")
    view = re.search(r'viewBox="([\d.eE+-]+) ([\d.eE+-]+) ([\d.eE+-]+) ([\d.eE+-]+)"', text)
    if not view:
        die(f"the {kind} SVG carries no parsable viewBox")
    box = tuple(float(g) for g in view.groups())
    tol = box[2] / 1000.0
    elements = [parse_path_data(d, tol) for d in re.findall(r'<path d="([^"]+)"', text)]
    if not elements:
        die(f"the {kind} SVG carries no paths")
    return elements, box


def generate_logo_paths() -> dict:
    result = {}
    for kind in ("mark", "wordmark"):
        elements, box = load_logo(kind)
        x, y, w, h = box
        tol = w / 1000.0
        eps = tol / 4.0
        triangles = triangulate_art(elements, eps, box)
        scale = 1000.0 / h

        def to_units(p: Point) -> tuple[float, float]:
            # Two decimals, the same precision fmt_unit emits, so the
            # degenerate-triangle filter sees exactly what ships.
            return (round((p[0] - x) * scale, 2), round((p[1] - y) * scale, 2))

        flat: list[list[float]] = []
        for a, b, c in triangles:
            ua, ub, uc = to_units(a), to_units(b), to_units(c)
            # Filter on the emitted (rounded) coordinates: a hair fold
            # rounds to a zero-area triple no matter its source area, so
            # the floor lives here, in the units the widget paints.
            emitted = abs(
                (ub[0] - ua[0]) * (uc[1] - ua[1]) - (ub[1] - ua[1]) * (uc[0] - ua[0])
            ) / 2.0
            if emitted > 0.05:
                flat.append([ua[0], ua[1], ub[0], ub[1], uc[0], uc[1]])

        # Clear space is the height of the V: the mark is its own V; the
        # wordmark's V is the first path element (the mark pieces lead
        # the wordmark's geometry).
        v_source = elements[0][0] if kind == "wordmark" else [
            p for element in elements for subpath in element for p in subpath
        ]
        ys = [p[1] for p in v_source]
        v_height = round((max(ys) - min(ys)) * scale, 1)

        result[kind] = {
            "width": round(w * scale, 3),
            "height": 1000.0,
            "clear": v_height,
            "triangles": flat,
            "source_box": (w, h),
        }
    return result


def fmt_unit(v: float) -> str:
    """One normalized coordinate: two decimals (a 1/100 of a unit is
    beyond the flattening tolerance by two orders), trailing zeros
    stripped to keep the generated table lean and clippy-quiet."""
    s = f"{v:.2f}".rstrip("0")
    s = s + "0" if s.endswith(".") else (s if "." in s else s + ".0")
    return s


def write_paths_rs(logo: dict) -> None:
    def block(kind: str, label: str) -> str:
        art = logo[kind]
        tris = art["triangles"]
        lines = [
            f"/// {label} art box width in normalized units (the box is 1000 units tall).",
            f"pub const {kind.upper()}_WIDTH: f32 = {art['width']};",
            "",
            f"/// {label} art box height in normalized units.",
            f"pub const {kind.upper()}_HEIGHT: f32 = {art['height']};",
            "",
            f"/// {label} clear space in normalized units: the height of the V.",
            f"pub const {kind.upper()}_CLEAR_SPACE: f32 = {art['clear']};",
            "",
            f"/// {label} triangles, each `[x0, y0, x1, y1, x2, y2]` in normalized",
            f"/// units, y down, origin at the art box's top-left corner.",
            f"pub const {kind.upper()}_TRIANGLES: &[[f32; 6]] = &[",
        ]
        for i in range(0, len(tris), 2):
            chunk = tris[i : i + 2]
            row = " ".join(
                "[" + ", ".join(fmt_unit(v) for v in tri) + "]," for tri in chunk
            )
            lines.append(f"    {row}")
        lines.append("];")
        return "\n".join(lines)

    PATHS_RS.parent.mkdir(parents=True, exist_ok=True)
    PATHS_RS.write_text(
        "//! Generated by tools/prepare_icons.py from the design-system logo\n"
        f"//! SVGs ({DO_NOT_EDIT} see src/logo.rs for the painting side).\n"
        "//! The art boxes are scaled to 1000 units tall; the script flattens the\n"
        "//! Bezier contours at 1/1000 of the art box width and triangulates them\n"
        "//! by even-odd scanline decomposition (robust to the exported paths'\n"
        "//! hair folds), verifying the mesh against point-sampling the artwork\n"
        "//! before emitting.\n"
        "//! The colour never lives here: the widget paints these triangles in a\n"
        "//! role colour at run time.\n\n"
        + block("mark", "V mark")
        + "\n\n"
        + block("wordmark", "wordmark")
        + "\n",
        encoding="utf-8",
    )


# ---- the generated enum ----------------------------------------------------


def variant_name(snake: str) -> str:
    return "".join(part.capitalize() for part in snake.split("_"))


def write_icons_rs(data: dict) -> None:
    catalogue = json.loads(CATALOGUE.read_text(encoding="utf-8"))
    icons = catalogue["icons"]
    codepoints = data["codepoints"]
    categories: list[tuple[str, list[str]]] = []
    for entry in icons:
        if categories and categories[-1][0] == entry["category"]:
            categories[-1][1].append(entry["name"])
        else:
            categories.append((entry["category"], [entry["name"]]))

    def variant(entry) -> str:
        return variant_name(entry["name"])

    def doc(entry) -> str:
        name = entry["name"]
        parts = [f"`{name}` · {entry['category']}. {entry['use']}"]
        if name in MISSING_ALIAS:
            parts.append(
                f"ALIAS: no glyph in font 0.47.5, so it paints "
                f"`{MISSING_ALIAS[name]}`'s codepoint, the catalogue's pin."
            )
        if entry.get("fill"):
            parts.append("Fills when active.")
        return " ".join(parts)

    header = [
        "//! Generated by tools/prepare_icons.py from the design-system icon",
        f"//! catalogue and font ({DO_NOT_EDIT} see src/icon.rs for the runtime).",
        "//! Material Symbols Outlined (npm material-symbols 0.47.5, Apache 2.0;",
        "//! the licence sits beside the fonts as",
        "//! MaterialSymbolsOutlined-LICENSE.txt): every icon of the approved",
        "//! 162-name catalogue as a snake-case variant, its private-use",
        "//! codepoint in the subsetted fonts, its fill flag and its category.",
    ]
    if data.get("dropped_grades"):
        header += [
            "//!",
            "//! DEVIATION: the four subsets exceeded the size budget, so the",
            "//! grade distinction was dropped (the dark families carry the",
            "//! GRAD 0 instances).",
        ]

    out = ["\n".join(header) + "\n\n"]
    out.append(f"/// The number of icons in the catalogue.\npub const CATALOGUE_COUNT: usize = {len(icons)};\n\n")
    out.append("/// The catalogue names, in `catalogue.json` order.\npub const CATALOGUE_NAMES: &[&str] = &[\n")
    for i in range(0, len(icons), 4):
        out.append("    " + " ".join(f'"{entry["name"]}",' for entry in icons[i : i + 4]) + "\n")
    out.append("];\n\n")
    out.append("/// One icon of the approved catalogue: Material Symbols Outlined,\n"
               "/// weight 300, in every theme.\n")
    out.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Icon {\n")
    for entry in icons:
        out.append(f"    /// {doc(entry)}\n    {variant(entry)},\n")
    out.append("}\n\nimpl Icon {\n")
    out.append("    /// Every icon, in catalogue order.\n    pub const ALL: &[Icon] = &[\n")
    for i in range(0, len(icons), 6):
        out.append(
            "        "
            + " ".join(f"Icon::{variant(entry)}," for entry in icons[i : i + 6])
            + "\n"
        )
    out.append("    ];\n\n")
    out.append("    /// The catalogue name.\n    pub const fn name(self) -> &'static str {\n        match self {\n")
    for entry in icons:
        out.append(f'            Icon::{variant(entry)} => "{entry["name"]}",\n')
    out.append("        }\n    }\n\n")
    out.append("    /// The private-use character the subsetted fonts carry for the\n"
               "    /// glyph (`location_on` borrows `push_pin`'s; see the module docs).\n"
               "    pub const fn as_char(self) -> char {\n        match self {\n")
    for entry in icons:
        cp = codepoints[entry["name"]]
        out.append(f"            Icon::{variant(entry)} => '\\u{{{cp:x}}}',\n")
    out.append("        }\n    }\n\n")
    filled = [entry for entry in icons if entry.get("fill")]
    arms: list[str] = []
    for i in range(0, len(filled), 4):
        arms.append(
            "            "
            + " ".join(f"Self::{variant(entry)} |" for entry in filled[i : i + 4]).rstrip("| ")
            + ("\n" if i + 4 >= len(filled) else " |\n")
        )
    # The final line must end the match arm without a bar.
    arms[-1] = arms[-1].rstrip(" |\n") + "\n"
    out.append(
        "    /// Whether an active state fills this icon: the selected\n"
        "    /// navigation item, a toggled control.\n"
        "    pub const fn has_fill(self) -> bool {\n        matches!(\n            self,\n"
        + "".join(arms)
        + "        )\n    }\n\n"
    )
    out.append("    /// The catalogue category.\n    pub const fn category(self) -> &'static str {\n        match self {\n")
    for entry in icons:
        out.append(f'            Icon::{variant(entry)} => "{entry["category"]}",\n')
    out.append("        }\n    }\n}\n\n")
    out.append("/// The categories in catalogue order with their icons, for the gallery.\npub const CATEGORY_GROUPS: &[(&str, &[Icon])] = &[\n")
    for category, names in categories:
        out.append(
            f'    ("{category}", &['
            + " ".join(f"Icon::{variant_name(n)}," for n in names)
            + "]),\n"
        )
    out.append("];\n")

    ICONS_RS.write_text("".join(out), encoding="utf-8")


def main() -> None:
    report: list[tuple[str, str, int]] = []
    data = generate_fonts(report)
    print(f"Resolved {len(data['names'])} catalogue names "
          f"({len(data['names']) - len(MISSING_ALIAS)} direct, "
          f"{len(MISSING_ALIAS)} aliased)")
    for name, alias in MISSING_ALIAS.items():
        print(f"  aliased {name} -> {alias} (no glyph in font 0.47.5)")

    write_icons_rs(data)
    print(f"wrote {ICONS_RS.relative_to(ROOT)}")

    logo = generate_logo_paths()
    write_paths_rs(logo)
    for kind, art in logo.items():
        w, h = art["source_box"]
        print(
            f"  {kind}: viewBox {w:.1f}x{h:.1f}, {len(art['triangles'])} triangles, "
            f"clear space {art['clear']} units"
        )
    print(f"wrote {PATHS_RS.relative_to(ROOT)}")

    (LOGOS_OUT / "source").mkdir(parents=True, exist_ok=True)
    for source in sorted((LOGO_DIR / "source").glob("*.svg")):
        shutil.copyfile(source, LOGOS_OUT / "source" / source.name)
    (LOGOS_OUT / "README.md").write_text(
        "# Logo sources\n\n"
        "The V mark and wordmark artwork, byte for byte from the design\n"
        "system's `assets/Logos/source/`. The kit never parses these at run\n"
        "time: `tools/prepare_icons.py` triangulates them once into\n"
        "`src/logo/paths.rs`, and the widget colours the triangles with role\n"
        "tokens. The per-ink SVGs stay in the design system as reference.\n",
        encoding="utf-8",
    )
    print(f"copied the logo sources to {LOGOS_OUT.relative_to(ROOT)}")

    print()
    print("Size report")
    print("  file                                size     derived from")
    print("  ----------------------------------  -------  ------------------------------")
    for out_name, derivation, size in report:
        print(f"  {out_name:<34} {human(size):>7}  {derivation}")
    print(f"  {f'{len(report)} subsets, total':<34} {human(data['total']):>7}")
    print()
    print(f"Wrote {len(report)} subsets, the generated enum and the logo paths")
    print("The output is committed; builds embed it with include_bytes!.")


if __name__ == "__main__":
    main()
