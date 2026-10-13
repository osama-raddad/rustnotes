# From Rust Notes to a mobile framework — extraction plan

Status: revised proposal, 2026-10-13, written against `main` at `2cd3725`
(PR #10). This document is a decision record plus a new proposal; it
replaces the extraction plan of the same date written against `71b08f1`.

## Decision record: the first plan is moot

The original plan proposed extracting the hand-rolled mobile shells —
`rustnotes-android` (EGL) and `rustnotes-ios` (EAGL) — behind a
fourteen-method host→shell contract, with the `examples/eframe-mobile`
template as the second consumer and a five-phase rollout. PR #10 deleted
all of that: the workspace is now two crates, `rustnotes-core` (pure egui
app) and `rustnotes` (eframe host), and eframe + winit hosts on Android,
iOS and desktop alike. Two of the plan's open questions were resolved by
events:

- **Q3 — may the desktop dev shell use winit?** Resolved: everything uses
  winit now, through eframe; the desktop loop is `cargo run -p rustnotes`.
- **Q4 — keep or delete the eframe template?** Resolved: deleted; there is
  no `examples/` and no second consumer.

## The extraction question now

Not "extract the shells" but: **make a mobile-egui distribution on top of
eframe.** eframe hosts the app; the reusable value is the thin layer Rust
Notes adds where eframe stops. Candidate pieces:

| Piece | Lives in | Why it is reusable |
|---|---|---|
| Android platform services: JNI window insets, night mode, font scale, bar icons, `finish()` on back at the root | `crates/rustnotes/src/android.rs` | egui-winit 0.36 populates `safe_area_insets` only on iOS, and reports neither OS theme nor text scale on Android — every mobile egui app needs these reads |
| iOS platform services: CoreText system font, safe-area + keyboard-frame insets, appearance, Dynamic Type | `crates/rustnotes/src/ios.rs` | the same gaps on iOS, filled through objc2 with no Swift/ObjC source |
| Layout: density contract, window size classes, scaffold, metrics, palette, nav | `crates/rustnotes-core/src/layout/` | egui-only, no platform code; single consumer today — see `mobile-layout-system-plan.md` for the extract-later rule |
| Packaging: the aapt2/zipalign/apksigner and codesign/simctl pipelines — no Gradle, no Xcode project | `justfile` | shell-independent recipes any pure-Rust mobile app needs |

## Platform services matrix

What a mobile host must supply, and what supplies it today — answered:
**eframe/winit, plus the JNI/objc2 helpers in the host crate.**

| Service | Android today | iOS today | Status |
|---|---|---|---|
| Window, event loop, GPU | eframe + winit (glow / OpenGL ES 3) | eframe + winit (wgpu / Metal) | done |
| Soft keyboard | eframe/winit | eframe/winit | done |
| Data directory | eframe storage | eframe storage | done |
| Safe-area insets | JNI `WindowInsets` query (the egui-winit Android gap) | `safeAreaInsets` + keyboard frame | done |
| Pixels per point | winit scale factor | winit scale factor | done |
| Text scale | JNI `fontScale` | Dynamic Type (`preferredContentSizeCategory`) | done |
| Appearance | `ui_mode_night` via JNI | `userInterfaceStyle` | done |
| Reduce motion | — | — | core API exists; the host now feeds it |
| Back navigation | back key → `on_back`, `finish()` at the root | edge-swipe filter → `Nav` | done |
| Save on backgrounding | focus loss | `WillResignActive`/`DidEnterBackground` observers | wired in the app |
| IME composing | — | — | open (below) |
| Clipboard, haptics, share | — | — | later |

## Guard rails the framework carries for its users

Lessons from the reviews in this repo, each already encoded in Rust Notes
and each a framework responsibility:

- Opening a text field with a shared widget id must reset its stored
  caret, or the previous screen's selection leaks in.
- Android has no safe-area insets from egui-winit; the host derives them
  itself through JNI, so users never see the status-bar overlap.
- winit reports no OS theme and no text scale on Android or iOS; the host
  feeds both directly (JNI `ui_mode_night`/`fontScale`, objc2
  `userInterfaceStyle`/Dynamic Type).
- Mobile persistence has no default directory in eframe; the data path
  must always resolve to the app's private storage.
- Saves must not depend on destroy-time callbacks: they fire on editor
  close/switch, debounced after typing stops, and on backgrounding
  (Android focus loss, iOS `WillResignActive`/`DidEnterBackground`).

## Still open: IME composing on Android

Unchanged by the pivot and now the biggest functional gap. Under
NativeActivity, Android delivers key events only — no `InputConnection`
ever exists, so there is no composing text: CJK input, autocorrect and
swipe typing are impossible. iOS has the same shape of problem in winit's
UIKeyInput view. The real fix on Android is eframe's
`android-game-activity` feature; whether to take it is the decision that
sets what kind of apps the framework can host.

## Open questions

1. **Layer on eframe, or fork it?** The platform modules work as a layer
   today, running at `raw_input_hook` time inside eframe's loop. Anything
   that seam cannot reach — richer lifecycle callbacks, activity results,
   IME — means pushing changes upstream or forking.
2. **Do `android.rs` / `ios.rs` generalize?** They are small because this
   app is small; extraction means auditing every JNI/objc2 assumption
   against a second app's needs.
3. **Does `layout/` come out too?** Not yet. PR #10 left it with a single
   consumer; per `mobile-layout-system-plan.md`, extract only when a real
   second one appears.
4. **IME composing** — see above.
5. **Name and home.** Crate names are unsettled; this repo (renamed) vs a
   new repo with Rust Notes as the example app.

## Endpoint, unchanged by the pivot

A thin CLI over the packaging recipes (`run --android`, `run --ios`,
`screenshot`) and a crates.io release of whatever crates survive the
questions above — the old plan's tooling/publish phase, which never
depended on the shells.

## Not in scope

Bindings to native UI (this stays an egui framework), wasm/web, desktop as
a product target (desktop is a dev loop), and accessibility trees (egui's
AccessKit integration is desktop-only in 0.36).
