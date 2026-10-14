# ui-kit

The Vynatix design system as pure egui: the design tokens generated from
the brand book (six themes, 54 colour roles each, spacing, radii,
shadows, type scale), the typography stack with self-hosted fonts, the
interaction states and focus ring, the Material Symbols icon catalogue,
the logo mesh, and the 22 components built on them. Like
`rustnotes-core`, the crate is pure egui — no window, GL or platform
glue; the gallery app (`crates/ui-kit-gallery`) hosts it for the demo.

## Run the gallery

```sh
cargo run -p ui-kit-gallery               # desktop window
just run-ios app=ui-kit-gallery           # iOS simulator (boots one if needed)
just ios-install app=ui-kit-gallery       # connected iPhone (sign + install + launch)
just run-android app=ui-kit-gallery       # Android device or emulator
```

The gallery is the kit's demo surface and its QA: the bar-plus-tabs
chrome, four panels (Brand, Foundations, Components, Feedback), every
component live in the active theme. The bar's Theme menu (a kit button
opening a kit popover) switches all six themes and the reduced-motion
and reduced-transparency flags; the choice holds for the session.

## The gallery app and mobile input

`crates/ui-kit-gallery` is a real app crate (bin + cdylib) hosted by
`crates/rustnotes` the way `hello-mobile` is: the host supplies the
platform glue (iOS safe-area and keyboard observers, wgpu limits;
Android window insets, night mode, font scale) and the app implements
the host's `App` seam. The kit stays pure egui.

The mobile-input facts live in the app crate:

- The chrome honours the safe-area insets itself (egui hands the app a
  root that covers the whole viewport); the margins behind the insets
  are painted with the live theme's canvas.
- Touch scrolling is egui's own. `GalleryApp::filter_input` filters
  nothing: egui-winit's touches reach egui, egui then knows the screen
  is a touch screen, and its drag-to-scroll moves the page under the
  finger, with the kinetic fling on release. The page's scroll area
  carries its own id salt, because the kit's Tabs strip above it is an
  unsalted scroll area in the same `Ui`; sharing one id, the strip
  clamped the page's offset back to zero every frame.
- The gallery's labels are not selectable (a gallery is not a
  document), so a drag that starts on text scrolls the page instead of
  selecting it.
- The sideways specimen rows scroll under the finger too. A drag that
  starts on one belongs to that row: sideways it scrolls the row, but
  up or down it does not reach the page (handing vertical drags on to
  the page is a follow-up).

Headless input tests (`src/input_tests.rs`) drive the real app the way
egui-winit delivers a finger and pin all of it: the page following a
swipe and flinging on, a row scrolling sideways, taps landing once
touches have been seen, the wheel, and the theme menu. One egui
behaviour to know: the drag-to-scroll arms on the first touch, so on a
fresh launch the very first swipe lands on nothing and every later one
scrolls.

On a phone-width window the bar drops its nav links and call to action
(the Tabs strip switches panels) and its theme menu keeps the bare
label; the caption under the tab strip steps aside; the button rows and
the Foundations specimens wrap, the card grid and the form grids stack,
and the specimen rows scroll sideways as the web previews do. Layout
tests hold the Foundations, Components and Feedback panels to a 360pt
phone and to a 1200pt desktop window. The Brand tab's showcase pages
still lay their cards out in wide rows (egui's `horizontal_wrapped`
does not wrap a card, which is a child `Ui`); phone-tight rows there
are a follow-up.

## Using the kit in an egui app

Install the fonts and apply the theme once at startup; pass the theme
and the flags down to every component:

```rust
use eframe::egui;
use ui_kit::components::Button;
use ui_kit::states::UiKitFlags;
use ui_kit::theme::ThemeId;
use ui_kit::{icon, typography};

fn main() -> eframe::Result {
    eframe::run_native(
        "My app",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            typography::install(ThemeId::Light, &cc.egui_ctx); // the faces
            icon::install(&cc.egui_ctx); // the icon font
            ThemeId::Light.apply(&cc.egui_ctx); // the visuals
            Ok(Box::new(MyApp { theme: ThemeId::Light, flags: UiKitFlags::default() }))
        }),
    )
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        Button::new("Start a project").ui(ui, self.theme, &self.flags);
    }
}
```

Components resolve their own tokens from the `ThemeId` they are given
(the Themes showcase paints all six at once this way), so a theme
switch is one field change plus `install` + `apply`. `UiKitFlags`
stands in for the web system's reduced-motion and reduced-transparency
media queries: the host holds them and passes them down.

Every string the gallery paints lives in `ui_kit::gallery_copy` as a
constant, and a unit test scans those constants (and the showcase
copy tables) with the voice lint: no em dashes, no ellipses outside
operational busy patterns, no exclamation marks, no emoji or
out-of-range glyphs, none of the brand's forbidden vocabulary, sentence
case for headings. Hosts can run `gallery_copy::voice_failures` over
their own copy.

| Module | What it holds |
|---|---|
| `tokens` | The generated design tokens (colour roles, spacing, radii, shadows, type scale, motion, breakpoints) |
| `theme` | The six themes and their egui `Visuals` |
| `typography` | The 11 specimen styles, the per-theme faces, tracking, `TrackedLabel` |
| `states` | The hover/press/disabled overlays, the focus ring, `UiKitFlags` |
| `icon` / `icons` | The icon widget and the generated 162-glyph catalogue |
| `logo` | The mark and wordmark as build-time triangulated role-coloured fills |
| `components` | The 22 widgets (button, chip, badge, card, metric and editorial cards, liquid glass, text field, select, tabs, top app bar, banner, snackbar, checkbox, radio, toggle, focus ring, icon, logo, cover, themes and brand showcases) |
| `gallery_copy` | The gallery's copy constants and the voice lint |

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

The instanced faces, what sets them, and the sizes they serve (the type
scale runs eyebrow 11, caption 12, label 13, body-sm 14, body 16,
body-lg 18, H6 20, H5 24, H4 32, H3 40, H2 56, H1 72, display 96):

| Face | Instance | Set by |
|---|---|---|
| Fraunces-Display | wght 500, opsz 144, SOFT 30 | display (96), H1 (72), H2 (56) in Forest |
| Fraunces-Headline | wght 600, opsz 36, SOFT 0 | H3 (40) in Forest |
| Fraunces-Metric | wght 700, opsz 144, SOFT 30 | the metric figure (56) and the editorial headline (24) |
| Fraunces-EditorialItalic | wght 400, opsz 22, SOFT 30 | editorial pull-quotes |
| Geist-Regular | wght 400 | body, lede, caption in Teal and Monochrome |
| Geist-Medium | wght 500 | display, H1, H2 in Monochrome |
| Geist-SemiBold | wght 600 | eyebrows; H3, H4 in Teal and Monochrome; the bold display voice in Monochrome |

Aptos (Regular, SemiBold), Instrument Serif (Regular, Italic) and
JetBrains Mono (Regular) are copied verbatim. Every face registers as
its own named family (`src/typography.rs`), since egui has no weight
axis; the two component styles that name weight 700 resolve through
`typography::display_bold_family`, where Teal keeps Instrument Serif's
single weight and Monochrome's Geist takes its 600 instance (documented
deviations in the typography module docs).

## Licences

The licence texts sit next to the fonts: `InstrumentSerif-OFL.txt` and
`JetBrainsMono-OFL.txt` (the two OFL texts), and
`THIRD-PARTY-NOTES.md`, which covers Aptos (Microsoft's typeface:
restricted to Vynatix products, not redistributed beyond the products)
and the Fraunces and Geist sources (both OFL upstream; the instanced
statics this repo ships are derivative works under the same licence).
The icon fonts carry `MaterialSymbolsOutlined-LICENSE.txt` (Apache 2.0)
beside them.

## Deviations from the brand book

The port is honest about where egui cannot follow the spec. Every
deviation is pinned in the module docs named in parentheses; this is
the reader-facing list.

### Typography

- Variable fonts ship as instanced statics: egui has no
  variation-axis control, so each variable face is pinned to one named
  instance per style (`tools/prepare_fonts.py`, `typography`).
- Tracking is painted through `extra_letter_spacing` rather than a
  per-glyph painter; the eyebrow's +0.24em and the headlines' negative
  tracks ride the text (`typography`).
- Headline 2's opsz 96 reuses the display's 144 instance (one optical
  size fewer, imperceptible at 56px).
- The lede's weight 350 ships as the Regular 400 face (no 350 static
  exists).
- Teal's display styles name weight 500 but set Instrument Serif,
  which ships one weight (no synthesis).
- The two weight-700 component styles resolve through
  `display_bold_family` (Forest instanced a Fraunces 700; Teal keeps
  Instrument Serif; Monochrome's Geist takes 600).
- The chip's weight 500 and the tab labels' weight 500 rest at the
  400 stand-ins (`chip`, `tabs`; Forest and Teal tab labels rest at
  400).
- The brand page's principle numeral sits at the shipped display
  weight; the theme cards' display specimen sits at 22px where the
  preview uses 32 (`brand_showcase`, `themes_showcase`).
- The brand page's Don't strings drop the source's trailing ellipsis
  (the voice forbids it; `brand_showcase`).

### Icons

- The optical size is pinned at 24 in every subset; the grade ships as
  two static instances (0 on light canvases, -25 on dark); the weight
  is 300 only (`tools/prepare_icons.py`, `icon`).
- The checkbox mark's weight 600 is not prepared: the mark draws in the
  icon font's 300 (`checkbox`).
- `location_on` has no glyph in Material Symbols 0.47.5, so the
  generated enum aliases it to `push_pin`, the catalogue's pin
  (`tools/prepare_icons.py`).

### Motion, state and accessibility

- Press scale, hover overlays and state-layer fills land on the frame
  they are detected (no tweened fills); the per-emphasis CSS hover
  filters are superseded by the shared state layers; the interactive
  card's hover lift is un-animated (`states`, `button`, `card`).
- The motion tokens publish the one curve the design system documents;
  the toggle's knob is the single animated travel, on that house curve
  (`tokens::motion`, `toggle`).
- Reduced motion and reduced transparency are host-held flags, not
  media queries: the host reads the OS and sets `UiKitFlags`
  (`states`).
- The focus ring follows the browsers' `:focus-visible` rule: egui says
  a widget holds focus but not how it got there, so the ring tracks the
  last input itself. Key presses and typed text show it, a mouse press
  or touch hides it, and Ctrl/Alt/Cmd chords do not count; a text
  input's ring ignores the tracking (`focus_ring`).
- Chrome (hover) state lands one frame late where the focus ring does
  not; popup arrow-key focus lands the frame after the gain; Tab walks
  the radio rows rather than the group as one (`select`, `radio`).
- The snackbar's message doubles as its accessible one-liner; egui has
  no Switch or mixed widget-info slot, so the toggle reports through
  the checkbox slot and an indeterminate checkbox reports unchecked;
  forced-colours and currentColor have no egui counterpart
  (`snackbar`, `toggle`, `checkbox`).

### Glass and blur

- egui composites no backdrop blur: the liquid glass panes carry the
  tint and its opacity plus the 1px specular edge, and turn solid
  under the reduced-transparency flag (`liquid_glass`).

### Rendering and platform

- Shadows render as one composited layer where the tokens carry the
  raw layer lists (the lists are preserved in `tokens::shadows`).
- The select's popup rows sit square inside the rounded frame
  (`select`).
- The text field's control height is pinned at 44 rather than computed
  from the padding arithmetic (`text_field`).

## Generators

Three scripts under `tools/`, all deterministic (no timestamps
recalculated), all with committed output that builds never rerun.
Re-run from this directory after the design-system sources change,
then format and commit the result:

| Script | Reads | Writes | Rerun when |
|---|---|---|---|
| `generate_tokens.py` | `.afk/design-system/tokens.json` | `src/tokens/*.rs`, `src/tokens/snapshot.txt` | the tokens change; then `cargo fmt` over the tables |
| `prepare_fonts.py` | `.afk/design-system/fonts/` | `assets/fonts/*.ttf` + licence texts | the font sources change |
| `prepare_icons.py` | the icon variable font, `assets/Icons/catalogue.json`, `assets/Logos/*.svg` | `assets/fonts/MaterialSymbols-*.ttf`, `src/icons.rs`, `src/logo/paths.rs` | the icon catalogue or logo artwork changes; then `cargo fmt` |

`generate_tokens.py --stub` writes the same structure with placeholder
values while `snapshot.txt` keeps the real ones: it exists so the token
tests can be shown failing before real generation (red) and passing
after (green).

## Testing

`just test` runs the crate's unit tests (plus `rustnotes-core`'s);
`just lint` runs fmt and clippy over all crates; `just check`
type-checks every crate for its real targets. The token tests pin the
generated code against `snapshot.txt`, the component tests paint
headless in every theme, and the voice lint holds the gallery's copy to
the brand book's rules.
