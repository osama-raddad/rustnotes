# Mobile-first layout system — design plan

Status: **implemented for Rust Notes** in PR #5 (merged 2026-10-13), all
seven phases. The eframe template (`examples/eframe-mobile`) was deleted in
PR #10; the module's consumer is the app core itself, and the eframe host
crate does not use it. Line numbers in "What exists today" describe the code
*before* PR #5.

## As shipped

The design below held. Where the implementation differs, the section text
has been corrected in place; the differences in one table:

| Topic | Plan said | Shipped |
|---|---|---|
| iPad Pro 11-inch, portrait | two panes | **one pane**: 834 pt wide is under the 840 pt Expanded breakpoint, so it is Medium. Two panes show on the 13-inch in portrait (1032 pt) and on tablets in landscape |
| Text scale | `ctx.set_zoom_factor` | multiplies the effective pixels-per-point passed to egui (`density × text_scale`), avoiding egui's zoom rect adjustment |
| Appearance on iOS | `traitCollectionDidChange` | the shell polls `traitCollection.userInterfaceStyle` and `preferredContentSizeCategory` on its frame timer; the core ignores unchanged values |
| Appearance API | `CoreEvent::Appearance` | `App::set_appearance(dark)` and `App::set_text_scale(s)` |
| Transitions | two layers with transforms | two child UIs at a horizontal offset; `Scaffold` lays out relative to the rect it is given |
| Reduced motion | maps to 0 ms | `App::set_reduce_motion` exists; no shell feeds it yet |
| Routes | `Route::Editor(id)` | `Screen::Editor`; the note id stays in app state |
| Bottom nav | `BottomNav` struct, a stack per tab | `bottom_nav()` function; tabs are Notes and Settings (a diagnostics page); only Notes has a stack |
| Swipe to delete | `Sense::drag` rows, two-step confirm | pointer watcher (rows stay click-only so drag-to-scroll works); deletes on release past 40 %; the cross button and its two-tap confirm remain |
| Android bottom inset | not planned | the JNI inset query now returns the system-bar bottom too, dropped while the keyboard is visible |
| Info.plist | not planned | declares iPad and landscape orientations |
| Type scale | caption 11, body 14, body-large 16, title 17, headline 22 | as planned; list snippets moved from 13 to 14 pt |

Still open: the iOS soft-keyboard inset check, iPad landscape (the simulator
could not be rotated during verification), and a shell feeding reduced
motion.

Scope: a thin layout layer on top of egui 0.36 for the Rust Notes core
(`crates/rustnotes-core`), its one consumer; the eframe template
(`examples/eframe-mobile`), the second consumer when this was written, was
deleted in PR #10. It covers three things and nothing
else:

1. **Density contract** — points come from the device's real density, not a
   fixed design width.
2. **Window size classes** — compact / medium / expanded, decided by width
   in points, so screens choose a structure rather than guessing from
   orientation.
3. **Scaffold** — a safe-area-aware frame with top bar, body and bottom
   slots, keyboard-aware, that every screen renders into.

Phases 1–3 deliver those three. Phases 4–7 (navigation stack and
transitions; theming, system appearance and text scale; bottom navigation
and pull-to-refresh; swipe gestures) are designed in the "Later phases"
section and build on the same seams. A general widget library is not part
of this plan.

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

eframe template (`examples/eframe-mobile/src/app.rs`, PR #3 — deleted in
PR #10):

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

The list–detail row is the one visible payoff; it is Phase 3, and in scope.

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
    pub touch_target: f32,      // 48 (Material minimum; also satisfies the iOS HIG 44)
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
both at it. As shipped, the template never adopted it and was itself deleted
in PR #10; the framework extraction plan
(`docs/framework-extraction-plan.md`), now superseded by PR #10, would have
made `mobile-layout` a crate in its Phase 2.

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

**Phase 3 — size classes and list–detail.** Rust Notes renders
list–detail when `two_pane_ok()`; the template replaces its orientation
branch with `WindowClass`. Acceptance: the iPad Pro 13-inch simulator
shows list and editor side by side in portrait (1032 pt wide); the iPad Pro
11-inch in portrait (834 pt) correctly stays single-pane as Medium width;
the iPhone 17 Pro in landscape (874 x 402 pt) still shows one pane.

## Decisions (resolved 2026-10-13)

- **Tablets are in scope.** All three phases are planned; Phase 3 ships
  list–detail in Rust Notes and the `WindowClass` switch in the template,
  verified on the iPad Pro 13-inch simulator in portrait. Rust Notes gains
  a tablet target with this plan.
- **Touch target is 48 points** on both platforms. Material's minimum,
  which also satisfies the iOS HIG's 44. The template's `TOUCH_TARGET`
  constant moves from 44 to 48 when it adopts `Metrics`.
- **Android follows the system "Display size" setting.** Density comes
  from `config().density()`, so the UI scales with the user's
  accessibility preference exactly as native apps do. Consequence for
  testing: screenshots on the emulator are only comparable at the default
  display size.

## Later phases

Everything below builds on the scaffold's slots and `Metrics` without
changing them. Each phase is one PR with a screenshot or recording as its
acceptance check. Order matters only where stated.

### Phase 4 — navigation stack and transitions

Today the editor open/close is a state flip (`Screen::List` /
`Screen::Editor`). A stack makes back behaviour uniform across the Android
back key, the scaffold's back button and the iOS edge swipe (Phase 7).

```rust
pub struct Nav<R> { stack: Vec<R>, transition: Option<Transition<R>> }
pub struct Transition<R> { from: R, to: R, push: bool, started: f64 }

impl<R: Clone + PartialEq> Nav<R> {
    pub fn push(&mut self, r: R);        // starts a slide-in from the right
    pub fn pop(&mut self) -> bool;       // false when at the root (shell may exit)
    pub fn top(&self) -> &R;
    /// Render: during a transition both routes are drawn, offset by the
    /// eased progress; otherwise only `top`.
    pub fn show(&mut self, ui: &mut Ui, draw: impl FnMut(&mut Ui, &R));
}
```

Mechanism (as shipped): progress comes from egui's clock with an ease-out
cubic over 250 ms; `show` creates two child UIs with `UiBuilder::max_rect`
on the viewport rect offset sideways, outgoing at `-0.3 * width * t` and
incoming at `width * (1 - t)`, the Material "shared axis" feel. `Scaffold`
lays out relative to the rect it is given, so a screen needs no change to
be drawn mid-slide. `App::set_reduce_motion` turns the slide off; no shell
feeds it yet. Rust Notes: `Screen` became `Nav<Screen>` with `Screen::List`
and `Screen::Editor`; `close_editor` became `pop`. Screens queue commands
instead of touching the stack, because the stack is borrowed while it draws. The `Back` key handler
already returns whether the event was consumed, so the Android shell keeps
working unchanged.

Acceptance: recording of list → editor → back on both simulators shows a
single 250 ms slide with no flash; the Android hardware back key pops;
the back key at the root is reported unhandled.

### Phase 5 — theming, system appearance, text scale

Three layers, in order of precedence:

1. **Tokens** (`Palette`): background, surface, text, text-weak, accent,
   danger, hairline. Both apps' colour constants collapse into one struct
   with a light and a dark instance. `Palette::apply(ctx)` writes them into
   `Visuals` through `set_visuals_of(Theme::Light, ..)` and
   `set_visuals_of(Theme::Dark, ..)`, which keeps both styles correct and
   is what the PR #3 review found the template getting wrong.
2. **System appearance.** egui follows `ThemePreference::System` only when
   the shell reports the OS theme, and winit reports none on Android or iOS.
   The shells feed it directly: Android reads
   `AndroidApp::config().ui_mode_night()` every loop iteration (a cheap
   local read), iOS polls `traitCollection.userInterfaceStyle` on its frame
   timer. Both call `App::set_appearance(dark)`, which sets
   `ctx.options_mut(|o| o.fallback_theme = ..)` only when it changed. Rust
   Notes gets a dark mode out of this for free.
3. **Text scale.** Android `fontScale` (not exposed by the NDK
   configuration, so a JNI query on `Configuration.fontScale`, re-run on
   config change) and iOS Dynamic Type (`preferredContentSizeCategory`,
   mapped to a ratio) call `App::set_text_scale(s)`. The core multiplies the
   effective pixels-per-point it hands egui by `s`, which scales text, bars
   and targets uniformly without egui's zoom machinery; `set_viewport` keeps
   density, `set_text_scale` keeps the multiplier.

Acceptance: emulator with the system dark theme on shows the dark palette
at launch and switches live when toggled in Settings; "Font size: largest"
on the emulator scales text and bars together without clipping; the same
on the iPhone simulator via Accessibility → Larger Text.

### Phase 6 — bottom navigation bar and pull-to-refresh

**BottomNav** fills the scaffold's bottom slot:

```rust
pub struct NavItem { pub label: &'static str, pub icon: fn(&Painter, Rect, Color32) }
/// Some(i) when a tab other than `selected` was tapped.
pub fn bottom_nav(ui: &mut Ui, items: &[NavItem], selected: usize,
                  pal: &Palette, m: &Metrics) -> Option<usize>;
```

Height is `Metrics::bar_height` (56) plus the bottom safe inset, which the
scaffold already excludes from the body, so the bar paints its background
over `viewport_rect` down to the screen edge and keeps its items inside
`content_rect`. Items are `touch_target` wide at minimum and the active
one gets the accent colour. Rust Notes ships two tabs: Notes, which owns
the `Nav` stack (so switching tabs preserves an open editor), and Settings,
a read-only diagnostics page showing density, text scale, window size,
size class and appearance.

**Pull-to-refresh** is the one piece egui lacks a primitive for: a
`ScrollArea` clamps at offset 0 and does not overscroll. The mechanism is
a drag watcher around the list: when the scroll offset is 0 and the
pointer drags downward (`input.pointer.delta().y > 0` with
`press_origin()` inside the list), accumulate `pull` with resistance
(`pull += dy * 0.5`), draw a spinner at `top + pull`, and on release past
`64` points fire the refresh and animate `pull` back with
`animate_value_with_time`. While `pull > 0` the ScrollArea is shown with
`vertical_scroll_offset(0)` so it does not fight. Rust Notes uses it to
re-read the SQLite list.

Acceptance: recording on the Android emulator (`adb shell input swipe`)
shows the spinner follow the finger, snap back, and the list reload; a
drag that starts mid-list scrolls instead of pulling.

### Phase 7 — swipe gestures

Two gestures, both with an axis lock so they do not fight drag-to-scroll:
nothing is decided until the pointer has moved 8 points, then the larger
axis wins for the rest of that touch.

- **Swipe-to-delete on list rows.** Rows stay `Sense::click()`, because a
  dragging row would swallow egui's drag-to-scroll; instead the list watches
  the pointer from the press origin. A leftward drag reveals a
  `danger`-coloured action behind the card by translating the card's
  painted rect, and while it is locked horizontal the scroll area's drag
  source is switched off. Past 40% of the width on release the delete fires
  immediately; otherwise it animates back. The cross button and its
  two-tap confirmation remain alongside.
- **Edge swipe back (iOS).** A drag whose `press_origin().x` is within 20
  points of the left edge of `content_rect` drives `Nav::pop` interactively:
  the transition progress follows the finger, commits past 35% or on a
  fast fling, cancels otherwise. Android keeps the system back gesture,
  which arrives as the back key.

Acceptance: swipe-to-delete recording on both simulators; the iOS edge
swipe recording shows the editor following the finger and snapping back
when released early.

### Still not covered

Haptics, share sheets and other platform services, accessibility tree
(egui's AccessKit integration is desktop-only in 0.36), and multi-window.
None of them touch layout.
