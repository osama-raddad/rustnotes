# From Rust Notes to a mobile framework — extraction plan

> **Status: superseded.** PR #10 (commit 2e31204) made eframe + winit the
> host and deleted the hand-rolled shells this plan proposed to extract, so
> the plan below was never implemented. It is kept as the historical record;
> any future extraction would target the eframe host crate
> (`crates/rustnotes`) instead.

Status: proposal, 2026-10-13. Nothing here is implemented. Line numbers refer
to `main` at `71b08f1`.

Goal: turn what Rust Notes already proves — a pure-Rust Android + iOS app
with egui, hand-rolled EGL/EAGL shells, and a mobile layout layer — into a
framework another app can depend on, so that a new app consists of one
trait implementation and no platform code. Rust Notes itself becomes the
framework's example app and its end-to-end test.

Working crate names below are placeholders (`mobile-*`); the real name is an
open question.

## What exists today

| Piece | Lines | Goes to |
|---|---|---|
| `crates/rustnotes-core/src/lib.rs` — `App` | 691 | split: host driver → framework, notes logic → example |
| `crates/rustnotes-core/src/screens.rs` | 662 | example (list, editor, settings, two-pane) |
| `crates/rustnotes-core/src/layout/` | 785 | framework (`mobile-layout`) |
| `crates/rustnotes-core/src/notes.rs`, `util.rs` | 132 | example (SQLite, date formatting) |
| `crates/rustnotes-android/` (`lib.rs` + `egl.rs`) | 599 | framework (`mobile-android`) → deleted in PR #10 |
| `crates/rustnotes-ios/src/main.rs` | 607 | framework (`mobile-ios`) → deleted in PR #10 |
| `justfile`, `resources/`, `rust-toolchain.toml` | — | templates |
| `examples/eframe-mobile/` | — | open question (see below) → deleted in PR #10 |

The seam runs through `App` (`lib.rs:94–165`). Its fields sort cleanly:

- **Host driver (framework):** `ctx`, `painter`, `font_source`, `events`,
  `size_px`, `ppp`, `density_ppp`, `text_scale`, `content_top`,
  `content_bottom`, `start`, `next_repaint`, `keyboard_shown`, `tapped`,
  `reduce_motion`, `edge_swipe_enabled`, `edge`, `two_pane`.
- **App (example):** `db`, `notes`, `edit_id`, `buffer`, `focus_editor`,
  `reset_list_scroll`, `armed_delete`, `pull_*`, `list_offset`, `swipe`.
- **Shared, framework-owned but app-driven:** `nav`, `tab`, `cmds`. The
  stack and the command queue are generic; the route and tab enums are the
  app's.

Methods sort the same way. Framework: `attach_gl`, `detach_gl`,
`set_viewport`, `set_text_scale`, `set_appearance`, `set_content_*`,
`set_reduce_motion`, `set_edge_swipe_back`, `handle_event` (touch and key
translation, the edge-swipe state machine), `tick`, `frame`. App:
`persist_buffer`, `open_editor`, `close_editor`, `save_current`,
`handle_key`'s per-screen branches, everything in `screens.rs`.

The shells (`rustnotes-android/src/lib.rs:150–429`,
`rustnotes-ios/src/main.rs`) reach the core through fourteen methods —
`new`, `attach_gl`, `detach_gl`, `set_viewport`, `set_content_top`,
`set_content_bottom`, `set_text_scale`, `set_appearance`,
`set_edge_swipe_back` (iOS only), `handle_event`, `tick`, `frame`,
`save_current`, `font_source` — and one public field, `dirty`, which both
loops read every iteration to decide whether to draw. That is the whole
host-to-shell contract, and it contains nothing about notes. The field is
the one wart: the framework turns it into `Host::needs_repaint()` so the
contract is methods only.

`nav`, `tab` and `cmds` are generic containers over app-defined enums. The
`App` trait carries them as associated types (`type Route`, `type Tab`),
and `Host` owns the `Nav<Route>` and the command queue; the app only pushes
commands and reads `host.nav().top()`.

## What a consuming app writes

```rust
use mobile_app::{App, Host, Key};
use mobile_layout::{Scaffold, Metrics, Palette};

struct Counter { n: u32 }

impl App for Counter {
    fn new(host: &Host) -> Self {
        // host.data_dir(), host.appearance(), host.text_scale() available
        Counter { n: 0 }
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let (pal, m) = (Palette::of(ui.ctx()), Metrics::DEFAULT);
        Scaffold::new(&pal, &m)
            .top_bar(|u| { mobile_layout::bar_title(u, m.sp(3), "Counter", &pal, &m); })
            .show(ui, |ui, _class| {
                if ui.button(format!("Tapped {} times", self.n)).clicked() {
                    self.n += 1;
                }
            });
    }

    fn on_pause(&mut self, _host: &mut Host) { /* save */ }
    fn on_back(&mut self, _host: &mut Host) -> bool { false } // not consumed
}

mobile_android::entry!(Counter); // expands to android_main; empty off Android
```

plus a two-line `src/main.rs` with `mobile_ios::entry!(Counter);` for the
iOS binary. One package, `crate-type = ["cdylib", "rlib"]` for Android and
a `[[bin]]` for iOS, the shape the eframe template already compiles on all
three targets. No EGL, no objc2, no insets, no keyboard, no density, no font
loading in the app. If the plan cannot keep that block this small, the
framework is not done.

## Crate layout

```
crates/
  mobile-app/       App trait, Host (the driver split out of today's App),
                    CoreEvent/Key, Platform services trait, font discovery,
                    Nav<R> + command queue re-exported from mobile-layout
  mobile-layout/    today's layout/ module, unchanged API
  mobile-android/   library: Egl, JNI inset/fontScale queries, input
                    translation, `entry!` macro producing android_main
  mobile-ios/       library: objc2 classes, EAGL, text field, display link,
                    appearance/Dynamic Type polling, `entry!` producing main
  mobile-desktop/   dev shell (see open questions): winit + glutin, host OS,
                    for fast iteration and screenshot tests
examples/
  notes/            Rust Notes: NotesApp + screens + SQLite; lib.rs ends
                    with the Android entry!, main.rs holds the iOS one
templates/
  app/              cargo-generate template: Cargo workspace, justfile,
                    AndroidManifest.xml, Info.plist, rust-toolchain.toml
```

The shells become libraries so a downstream app owns zero platform code. The
`entry!` macros are the only unsafe-adjacent surface a user touches, and
each expands to the `android_main` / `fn main` that exists today with the
app type substituted.

Two patterns the framework must own rather than leave to users:

- **iOS has no default framebuffer.** The shell keeps its own
  `glow::Context` for the FBO and the painter gets a second handle; the
  shell binds the FBO before `frame()` and presents after. `mobile-ios`
  keeps both handles internal.
- **Android input and insets are asynchronous.** System-bar insets and
  `fontScale` come back through JNI on the Java main thread; the shell
  polls receivers from its loop. The IME inset is dropped while the keyboard
  is visible because `adjustResize` already shrinks the surface. All of
  this stays inside `mobile-android`.

## Platform services

Today's `Platform` trait has two methods (`show_keyboard`, `hide_keyboard`).
The framework version, implemented by each shell and reached through `Host`:

| Service | Android today | iOS today | Status |
|---|---|---|---|
| Soft keyboard show/hide | `show_soft_input` | hidden `UITextField` | done |
| Data directory | `internal_data_path()` | `$HOME/Documents` | done, becomes `Host::data_dir()` |
| Safe-area insets | JNI `WindowInsets` query | `safeAreaInsets` + keyboard frame | done |
| Pixels per point | configuration density | `UIScreen.scale` | done |
| Text scale | JNI `fontScale` | `preferredContentSizeCategory` | done |
| Appearance | `ui_mode_night` | `userInterfaceStyle` | done |
| Reduce motion | — | — | API exists, nothing feeds it |
| Lifecycle (pause, resume, low memory) | `MainEvent::Pause/Resume` only | delegate `willResignActive` only | partial |
| Back navigation | hardware key → `on_back` | edge swipe → `Nav` | done, needs the trait hook |
| Clipboard, haptics, share | — | — | later |

Lifecycle is the gap that bites first: a framework app expects `on_pause`
to be reliable for saving, and today only the notes app's `save_current`
is wired.

## Guard rails the framework carries for its users

These are the things found by the two reviews in this repo, each one
already handled in Rust Notes and each one a framework responsibility:

- Opening a text field with a shared widget id must reset its stored
  caret, or the previous screen's selection leaks in.
- Input events queue nowhere when no GL context is attached; the host drops
  them instead of growing a `Vec` forever.
- Android has no safe-area insets from egui-winit; the shell derives them
  itself. The framework shell does the same, so users never see the
  status-bar overlap the template review found.
- winit reports no OS theme on Android or iOS; the shells feed appearance
  directly. Same for text scale.
- Mobile persistence has no default directory in eframe; `Host::data_dir()`
  always resolves to the app's private storage.
- Touch drag-to-scroll in egui only works when `Event::Touch` is sent
  alongside pointer events; the host sends both.

## Phases

Each phase is one PR and ends with the Phase 1–7 layout screenshots re-taken
on the Pixel emulator, the iPhone 17 Pro and the iPad Pro 13-inch
simulators, matching the previous phase.

**Phase 1 — split `App` in place.** Inside `rustnotes-core`, move the host
driver into `Host` and the notes logic into `NotesApp: App`. The shells keep
calling the same nine methods, now on `Host<NotesApp>`. No crate moves, no
behaviour change. Acceptance: screenshots identical; `handle_event`'s
back-key semantics unchanged on Android.

**Phase 2 — extract crates.** `mobile-app`, `mobile-layout`,
`mobile-android`, `mobile-ios` as workspace members; `examples/notes` holds
`NotesApp`, `screens.rs`, `notes.rs`, `util.rs` and the two `entry!` lines.
The root `justfile` gains `example` recipes. Acceptance: `just check` plus
`just example-android` / `example-ios` give the same screenshots on all
three simulators; `cargo doc --no-deps` builds for every framework crate.

**Phase 3 — zero-platform-code apps.** `entry!` macros in both shells, the
`templates/app` cargo-generate template, and `mobile-desktop` so a new app
can run on the host with `cargo run`. Acceptance: `cargo generate` a
counter app, build it for Android and iOS without editing any file but
`app.rs`; the desktop shell renders the notes example and the layout unit
tests still pass.

**Phase 4 — lifecycle and input completeness.** `on_pause` / `on_resume` /
`on_low_memory` from both shells; `Host::data_dir()`; reduce-motion from
`AccessibilityManager` (JNI) and `UIAccessibility.isReduceMotionEnabled`;
IME composing on Android. That last item is the one place NativeActivity is
not enough: composed input (CJK, autocorrect, swipe typing) arrives through
an `InputConnection`, which NativeActivity never creates. The standard
route is `GameActivity`'s `TextInputState`, but `GameActivity` is a Java
`AppCompatActivity` subclass shipped as a Gradle dependency
(`androidx.games:games-activity`, per the android-activity README), so it
breaks the no-Java rule. See open question 5. Acceptance: backgrounding
the app mid-edit and relaunching restores the note; if the IME decision is
"relax the rule", typing a CJK string on the emulator keyboard lands in the
editor.

**Phase 5 — tooling and publish.** `cargo-mobile-cli` (a thin wrapper over
the justfile recipes: `new`, `run --android`, `run --ios`, `screenshot`),
crates.io release of the four framework crates with an MSRV of 1.95,
README and a docs site built from `cargo doc`. Acceptance: a clean machine
with the Android SDK/NDK and Xcode command-line tools goes from `cargo
install` to the counter app on both simulators following only the README.

## Open questions

1. **Name.** `mobile-*` is a placeholder. The name decides the repo name,
   the crate prefix and the macro names, so it is the first decision.
2. **This repo or a new one.** Renaming `rustnotes` keeps the history and
   the PRs; a new repo keeps `rustnotes` as a clean example consumer. The
   plan assumes a rename with `examples/notes` inside it.
3. **May the desktop dev shell use winit?** The pure-Rust, no-Java/Swift
   rule was about the mobile shells. A winit + glutin desktop shell is still
   pure Rust but is the thing the project deliberately avoided on mobile.
   Without it, every iteration goes through a simulator.
4. **The eframe template.** Once the framework is the alternative to eframe,
   `examples/eframe-mobile` is either a comparison kept current, or
   deleted. Its ten open review findings decide: every one of them is
   something the framework handles for its users.
5. **IME composing on Android.** Three ways out, none free: relax the
   no-Java rule for one small activity class compiled with `javac` + `d8`
   at build time (Slint does exactly this); stay on NativeActivity and
   accept key-event-only input, which rules out CJK and autocorrect; or
   drive an `InputConnection` from Rust through JNI by subclassing
   `NativeActivity` at runtime, which is unproven. The framework's answer
   sets what kind of apps it can host.

## Not in scope

Bindings to native UI (this stays an egui framework), wasm/web, desktop as
a product target (the desktop shell is for development only), and
accessibility trees (egui's AccessKit integration is desktop-only in 0.36).
