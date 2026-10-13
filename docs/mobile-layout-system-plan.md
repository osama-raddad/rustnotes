# Mobile-first layout system — design plan

Status: proposal, 2026-10-13. Nothing here is implemented. Line numbers
below refer to the branches named in each section (the egui migration on
`worktree-docs-rendering-engine-research`, the template on
`feat/eframe-mobile-template`), not to `main`.

Scope: a thin layout layer on top of egui 0.36 that both consumers in this
repo can use: the Rust Notes core (`crates/rustnotes-core`) and the eframe
template (`examples/eframe-mobile`). It covers three things and nothing
else:

1. **Density contract** — points come from the device's real density, not a
   fixed design width.
2. **Window size classes** — compact / medium / expanded, decided by width
   in points, so screens choose a structure rather than guessing from
   orientation.
3. **Scaffold** — a safe-area-aware frame with top bar, body and bottom
   slots, keyboard-aware, that every screen renders into.

Explicitly out of scope for this plan: a widget library, navigation stack,
page transitions, theming, gestures. They are listed as later phases only
so the seams are left open.

## What exists today

Rust Notes core (`crates/rustnotes-core/src/lib.rs`, branch
`worktree-docs-rendering-engine-research` after PR #2):

- `set_viewport` (line 215) derives pixels-per-point as
  `width_px / 360`, clamped to 0.75–3.5. The Pixel 10 Pro XL emulator
  (1344 px wide, 480 dpi) gets 3.73 pixels per point where its real density
  is 3.0, so every bar and glyph renders about 24% larger than in a native
  app; the iPhone 17 Pro gets 3.35 instead of its real 3.0. A 2560 px
  tablet hits the 3.5 clamp and is left with 731 points of width, so the UI
  scales *up* on tablets instead of gaining space.
- `ui()` (line 507) paints the full `viewport_rect` and carves a child UI
  from `content_rect` by hand. Every screen must repeat this.
- `header()` (line 528) is a 56-point bar with hand-painted contents; the
  list body uses literal 12/14/16/78-point spacing (lines 613–621) and the
  editor 12/16-point margins (lines 778–781). Nothing is shared with the
  template.
- Insets already flow correctly: shells push `content_top` / `content_bottom`
  in pixels and `frame()` converts them to `RawInput.safe_area_insets`
  (lines 450–451). The scaffold can rely on `ctx.content_rect()` without
  any shell change.

eframe template (`examples/eframe-mobile/src/app.rs`, PR #3):

- `orientation()` (line 98) compares `content_rect` height to width and
  branches portrait/landscape (lines 193–201). A 7-inch tablet in portrait
  is "portrait" and gets the phone stack even though it has room for two
  panes; a phone in landscape is "landscape" and gets two 180-point columns
  that are too narrow to use.
- A `TOUCH_TARGET` of 44 points is applied through `interact_size` (line
  77) and repeated inline for the demo button (line 149).
- The safe area is `content_rect().shrink(16.0)` (line 186), hand-rolled
  again.

The common defect: both consumers decide layout from the wrong inputs
(pixel width, aspect ratio) and repeat the safe-area and spacing arithmetic
per screen.

## Goals

- One `points` space whose values mean the same physical size on every
  device, as Android dp and iOS points do.
- Screens pick structure from a size class, never from orientation or
  pixel counts.
- Safe area, keyboard inset and the bar slots are handled once, in the
  scaffold, not in each screen.
- Spacing and type sizes come from one scale so the two apps look related.
- No new shell code beyond reading the density; no new crates until a
  second consumer actually adopts the module.

## Non-goals

- Pixel-perfect parity with Material or Cupertino. The size-class
  thresholds are borrowed; the visual language is not.
- Animated transitions between size classes. Re-layout on the next frame is
  enough; egui already repaints on resize.
- Replacing egui's own `Layout`/`Ui` for in-body content. The scaffold ends
  at the body rect; inside it, normal egui.

## Design

### 1. Density

`App::set_viewport` takes the scale from the shell and stops computing it
from width:

```rust
/// `pixels_per_point`: the OS scale. Android: configuration density / 160
/// (480 dpi -> 3.0). iOS: UIScreen.scale. None: keep the width heuristic
/// so desktop builds and tests still work.
pub fn set_viewport(&mut self, w: i32, h: i32, pixels_per_point: Option<f32>)
```

Android reads the density from
`android_activity::AndroidApp::config().density()` (`ConfigurationRef`,
android-activity 0.6.1, `src/config.rs:64`) and divides by 160; iOS
already holds `UIScreen.scale` in the shell as `s.scale`. Both are
one-line changes at the existing `set_viewport` call sites.

eframe gets this for free: winit reports the monitor scale factor and
eframe forwards it as `native_pixels_per_point`, so the template only has
to stop doing its own arithmetic.

This is a visible change on phones: the Android UI shrinks by about a fifth
and the iOS UI by about a tenth, to the size a native app would draw.

Acceptance: the 56-point top bar measures 168 px on the 480-dpi emulator
and 168 px on the 3x iPhone 17 Pro simulator, and sits next to a native
app's toolbar at the same height in a side-by-side screenshot.

### 2. Window size classes

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeClass { Compact, Medium, Expanded }

/// Material 3 window size classes, in points.
/// Width:  compact < 600, medium 600..840, expanded >= 840.
/// Height: compact < 480, medium 480..900, expanded >= 900.
pub struct WindowClass { pub width: SizeClass, pub height: SizeClass }

impl WindowClass {
    pub fn of(ctx: &egui::Context) -> Self;           // from content_rect()
    /// Two panes need room both ways: a phone in landscape has an expanded
    /// width (iPhone 17 Pro: 874 pt, Pixel 10 Pro XL: 997 pt) but a compact
    /// height (402 pt / 448 pt), and must stay single-pane.
    pub fn two_pane_ok(&self) -> bool {
        self.width == SizeClass::Expanded && self.height != SizeClass::Compact
    }
}
```

With real densities every current phone is width-Expanded in landscape, so
width alone would hand them the tablet layout; the height class is what
keeps the phone single-pane. Orientation stays available through
`content_rect()` for the rare case that genuinely needs it (camera, media).

What each class means for the two apps:

| Width class | Rust Notes | Template demo |
|---|---|---|
| Compact (phones in portrait) | list *or* editor, full width | settings above content |
| Medium (small tablets, unfolded foldables) | list with wider cards; editor with a max line width | settings and content side by side at 1:2 |
| Expanded and `two_pane_ok` (tablets, desktop) | list–detail: list pane left, editor right, no navigation | same, with a max content width |
| Expanded but compact height (phones in landscape) | single pane, editor with a max line width | settings above content, scrolled |

The list–detail row is the one visible payoff; it is Phase 3 and optional.

### 3. Scaffold

One struct that owns the frame every screen renders into:

```rust
pub struct Scaffold<'a> {
    top_bar: Option<Box<dyn FnOnce(&mut Ui) + 'a>>,      // 56 pt, hairline below
    bottom_bar: Option<Box<dyn FnOnce(&mut Ui) + 'a>>,   // 56 pt, hairline above
    fab: Option<Box<dyn FnOnce(&mut Ui) + 'a>>,          // 56 pt, 16 pt inset, above bottom bar
    background: Color32,
    scroll_body: bool,                                   // wrap body in ScrollArea
}

impl<'a> Scaffold<'a> {
    pub fn new() -> Self;
    pub fn top_bar(self, f: impl FnOnce(&mut Ui) + 'a) -> Self;
    pub fn bottom_bar(self, f: impl FnOnce(&mut Ui) + 'a) -> Self;
    pub fn fab(self, f: impl FnOnce(&mut Ui) + 'a) -> Self;
    pub fn background(self, c: Color32) -> Self;
    pub fn show(self, root: &mut Ui, body: impl FnOnce(&mut Ui, SizeClass)) -> ScaffoldResponse;
}
```

`show` does, in order:

1. Paint `background` over `viewport_rect()` so the notch and home-indicator
   strips take the screen colour (today's `ui()` does this by hand).
2. Take `content_rect()`; in Rust Notes this already excludes the status
   bar and the keyboard because the shells feed `safe_area_insets`. In the
   eframe template it does **not** on Android: egui-winit 0.36 populates
   `safe_area_insets` only under `cfg(target_os = "ios")` (the PR #3 review
   found this), so the template has to derive top/bottom insets from
   `AndroidApp::content_rect()` against the window size and push them into
   `RawInput` itself. The scaffold reads `content_rect()` either way.
3. Allocate the top bar at the top of the content rect, the bottom bar at
   the bottom, the FAB over the body's bottom-right corner, and hand the
   remaining rect to `body` with the current `SizeClass`.
4. Return the rects it used, so callers can position popups or measure.

The Rust Notes `header()` becomes a `top_bar` closure; the "+" button and
the back chevron become small helpers in the same module (`icon_button`,
`back_button`) so both screens stop painting line segments inline.

Keyboard: nothing new. When the keyboard opens, `content_bottom` grows,
`content_rect` shrinks, the body shrinks, and the editor's TextEdit
scroll-to-cursor already keeps the caret visible. The bottom bar rides up
with the keyboard, which is the usual mobile behaviour.

### 4. Spacing and type scale

A small `Metrics` table, in points, applied once to `Style` and read by the
scaffold:

```rust
pub struct Metrics {
    pub space: [f32; 5],        // 4, 8, 12, 16, 24
    pub bar_height: f32,        // 56
    pub touch_target: f32,      // 44 (iOS HIG) — 48 on Android if desired
    pub corner: f32,            // 10
    pub text: TextScale,        // caption 11, body 14, body_large 16, title 17, headline 22
    pub max_content_width: f32, // 640, used in Medium/Expanded bodies
}
```

The numbers are the ones both apps already use, collected. `Metrics::apply(ctx)`
sets `interact_size`, `item_spacing`, `button_padding` and the text styles,
replacing the ad-hoc `style_mut` blocks in both `App::new` functions.

### 5. Where it lives

`crates/rustnotes-core/src/layout/` with `mod.rs`, `size_class.rs`,
`scaffold.rs`, `metrics.rs`. It depends only on `egui`. It is **not** a
separate crate yet: the template is a standalone workspace and would need
a path dependency into the main workspace, which breaks the "copy this
folder out" property that makes the template useful. When the second
consumer adopts the module, extract it to `crates/mobile-layout` and point
both at it; until then, the template keeps its own copy of the two files it
needs (`size_class.rs`, `scaffold.rs`) with a header comment saying where
the source of truth is.

## Phases

Each phase is one PR and ends with a screenshot on both booted simulators.

**Phase 1 — density + metrics (visible: the UI shrinks to native size).**
Pass `pixels_per_point` into `set_viewport` from both shells, add
`Metrics` and apply it. Acceptance: the top bar is 168 px tall on both the
480-dpi emulator and the 3x iPhone simulator; the emulator in tablet mode
(`adb shell wm size 1600x2560`, `wm density 320`) shows the list at the same
physical size as the phone, with more rows.

**Phase 2 — scaffold.** Move both Rust Notes screens onto `Scaffold`;
delete `header()` and the inline `content_rect` carve-out. Acceptance: the
same four flows as the migration test (list, new note, typing, back) pass
on both simulators. The path that is still unverified is the iOS keyboard
inset: with the simulator's hardware-keyboard toggle switched off by hand,
the soft keyboard must appear and the caret must stay visible above it.
(Android proves nothing here: `adjustResize` shrinks the surface itself.)

**Phase 3 — size classes and list–detail (optional).** Rust Notes renders
list–detail when `two_pane_ok()`; the template replaces its orientation
branch with `WindowClass`. Acceptance: the iPad simulator shows list and
editor side by side in both orientations; the iPhone 17 Pro in landscape
(874 x 402 pt) still shows one pane.

## Open questions

- **Is Expanded in scope at all?** Rust Notes is a phone demo; list–detail
  is the only reason to carry size classes. If tablets are not a target,
  stop after Phase 2 and keep `SizeClass` as a two-line enum for the
  template.
- **Touch target: 44 or 48 points?** iOS HIG says 44, Material says 48.
  One number for both is simpler; 48 is the safer default.
- **Android density source.** `config().density()` is the configuration
  density, which follows the user's "display size" setting. That is the
  desired behaviour for dp parity, but it means the UI can scale by the
  user's choice. Confirm that is wanted.

## Not covered, on purpose

Navigation stack and transitions (the editor open/close today is a state
flip; a stack would come with the scaffold's back button), a bottom
navigation bar component, pull-to-refresh, swipe gestures, and theming.
Each builds on the scaffold's slots and `Metrics` without changing them.
