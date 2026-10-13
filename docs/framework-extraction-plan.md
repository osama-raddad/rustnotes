# From Rust Notes to a mobile framework — extraction plan

Status: revised proposal, 2026-10-13, re-anchored to `main` at `4caffb6`
(PRs #11–#13: safety net, android relaunch, review findings). This
revision corrects rows that were stale when the previous revision
(`3543885`) was committed mid-wave — its anchor said `2cd3725` while its
own tree already carried the fixes — and folds the wave's changes in.
It remains a decision record plus a new proposal; it replaces the
extraction plan of the same date written against `71b08f1`.

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

## What PRs #11–#13 added

The wave between the two anchors changed the plan's raw material:

- **Tests and CI exist.** 61 tests in `rustnotes-core` (durability, the
  editor state machine, layout including WCAG contrast, headless egui
  passes that hit-test real widgets), `just test`/`lint`/`check` recipes,
  and a two-job CI: fmt/clippy/tests on Linux, the three-target
  cross-check on macOS. The host crate itself still has zero tests —
  cross-compiles are its only guard, which matters: `635d3f8` repaired a
  review-driven Cargo tweak that had silently broken both mobile targets
  with no local signal.
- **Durability semantics landed** (`12b853e`, `4fa7194`): DB errors
  surface as app state (editor strip, list state, Settings row), autosave
  fires 1.5 s after the last keystroke and retries after a failed save,
  and an unopenable database degrades to an in-memory store instead of
  crash-looping the app.
- **Layout quality rose** (`45afee3`): light-mode selection contrast
  fixed under WCAG tests, per-route nav id salts (a route keeps its egui
  state namespace in every position), retargeted transitions. `layout/`
  grew from 812 to 1588 lines and its remaining app concepts now carry
  `TODO(framework-extraction)` markers.
- **The host hardened** (`d517b04`, `34a002f`, `01f6607`): iOS
  background-save observers, reduce motion and UTC offset now host-fed on
  both platforms, panic routing (logcat with forced backtrace / NSLog),
  back-at-root exits the eframe loop cleanly, statics reset on
  `android_main` re-entry, a portable NDK path, targetSdk 35 everywhere.

## The extraction question now

Not "extract the shells" but: **make a mobile-egui distribution on top of
eframe.** eframe hosts the app; the reusable value is the thin layer Rust
Notes adds where eframe stops. Candidate pieces:

| Piece | Lives in | Why it is reusable |
|---|---|---|
| Android platform services: `android_main` bootstrap with re-entry-safe statics, JNI window insets incl. IME, night mode, font scale, bar icons, reduce motion, UTC offset | `crates/rustnotes/src/android.rs` (583 lines) | egui-winit 0.36 populates `safe_area_insets` only on iOS, and reports neither OS theme nor text scale on Android — every mobile egui app needs these reads |
| iOS platform services: CoreText system font, safe-area + keyboard-frame insets, appearance, Dynamic Type, reduce motion, background-save observers, wgpu limits | `crates/rustnotes/src/ios.rs` (375) | the same gaps on iOS, filled through objc2 with no Swift/ObjC source |
| The host adapter: `eframe::App` impl, the seam below, entries, exit | `crates/rustnotes/src/lib.rs`, `main.rs` (249) | the pattern any embedded app follows; the framework's actual product |
| Layout: density contract, window size classes, scaffold, metrics, palette, nav | `crates/rustnotes-core/src/layout/` (1588) | egui-only, no platform code; single consumer today — extract-later rule below |
| Packaging: the aapt2/zipalign/apksigner and codesign/simctl pipelines — no Gradle, no Xcode project | `justfile`, `.github/workflows/ci.yml` | shell-independent recipes any pure-Rust mobile app needs |

### The seam today

The host reaches the app through nine calls, all one-directional
(`rustnotes` → `rustnotes-core`, which depends only on egui, log and
rusqlite):

`new(data_dir, ctx, platform_font)` · `set_edge_swipe_back` (iOS) ·
`font_source` · `set_reduce_motion` · `set_utc_offset_secs` ·
`filter_input` · `save_current` (focus loss, backgrounding, exit) ·
`on_back` · `ui`

Everything else the platform knows crosses through egui itself: the
platform modules read insets, keyboard frame, theme, font scale, reduce
motion and UTC offset and inject them at `eframe::App::raw_input_hook`
time (into `raw.safe_area_insets`, `ctx.set_zoom_factor`,
`ctx.options.fallback_theme`) or return them to the host to apply as
`set_*` calls. Two mechanisms now live deliberately outside that hook:
the iOS lifecycle observers, which fire between frames through a
main-thread pointer the host re-arms every `ui`, and the exit path, where
back-at-root sends `ViewportCommand::Close`, eframe's loop returns, and
android-activity's glue finishes the activity.

## Platform services matrix

What a mobile host must supply, and what supplies it today — answered:
**eframe/winit, plus the JNI/objc2 helpers in the host crate.**

| Service | Android today | iOS today | Status |
|---|---|---|---|
| Window, event loop, GPU | eframe + winit (glow / OpenGL ES 3) | eframe + winit (wgpu / Metal) | done |
| Soft keyboard | eframe/winit | eframe/winit | done |
| Data directory | `internal_data_path()`, resolved by the host | `$HOME/Documents`, resolved by the host | done, host-owned — eframe has no mobile storage dir |
| Safe-area insets | JNI `WindowInsets` query incl. IME (the egui-winit Android gap) | `safeAreaInsets` + keyboard frame | done |
| Pixels per point | winit scale factor | winit scale factor | done |
| Text scale | JNI `fontScale` → `zoom_factor` | Dynamic Type → `zoom_factor` | done; zoom is the carrier — egui has no font-size input |
| Appearance | `ui_mode_night` via JNI | `userInterfaceStyle` | done |
| Reduce motion | `animator_duration_scale` via JNI | `UIAccessibility.isReduceMotionEnabled` | done, host-fed |
| UTC offset | `TimeZone.getOffset` via JNI | `NSTimeZone.secondsFromGMT` | done, host-fed (the app formats local dates) |
| Back navigation | back key → `on_back`, exit the eframe loop at the root | edge-swipe filter → `Nav` | done |
| Save on backgrounding | focus loss | `WillResignActive`/`DidEnterBackground` observers | wired in the host |
| Lifecycle (pause, resume, low memory) | focus loss only | observers only | open — see below |
| IME composing | — | — | open (below) |
| Clipboard, haptics, share | — | — | later |

The lifecycle row is what remains of the old gap: no low-memory handling
exists on either platform, Android still has no true pause/resume
(focus loss is the only signal, and a system-initiated destroy simply
kills the process — winit drops `MainEvent::Destroy`), and iOS `on_exit`
is unreachable because `UIApplicationMain` never returns, which is why
the observers exist.

## Guard rails the framework carries for its users

Lessons from the reviews in this repo, each already encoded in Rust Notes
and each a framework responsibility:

- Opening a text field with a shared widget id must reset its stored
  caret, or the previous screen's selection leaks in.
- Android has no safe-area insets from egui-winit; the host derives them
  itself through JNI, and the IME inset must be *included* — bottom =
  max(bars, ime) — because under enforced edge-to-edge (targetSdk 35)
  `adjustResize` no longer makes room.
- winit reports no OS theme and no text scale on Android or iOS; the host
  feeds both directly (JNI `ui_mode_night`/`fontScale`, objc2
  `userInterfaceStyle`/Dynamic Type), with font scale riding
  `zoom_factor` and insets divided by the enforced zoom.
- Mobile persistence has no default directory in eframe; the host always
  resolves the data path itself (never `Activity.finish()` from inside
  the frame, and never a `persistence_path` eframe would ignore on
  mobile anyway).
- Saves must not depend on destroy-time callbacks: they fire on editor
  close/switch, 1.5 s after typing stops (retrying after a failure), on
  Android focus loss, and on iOS backgrounding observers.
- Exit by leaving eframe's loop (`ViewportCommand::Close` → `run_native`
  returns → the glue finishes the activity). Posting `Activity.finish()`
  deadlocks: NativeActivity's `onDestroy` waits for `android_main` to
  return, which the loop never does — a singleTask relaunch hangs.
- NativeActivity re-enters `android_main` in the same process; every
  query static (report, throttle, in-flight, bar-icon memo, handed-facts)
  must reset on re-entry or the new activity runs blind.
- iOS startup requires adapter-supported wgpu limits, not egui-wgpu's
  WebGPU defaults — otherwise eframe exits at startup on Metal.
- The manifest and packaging are a matched set: the full `configChanges`
  list (a missed entry recreates the activity and ANRs), edge-to-edge
  under targetSdk 35, `extractNativeLibs` with zipalign and 16 KB page
  alignment.
- Panics must route somewhere visible: logcat with a forced backtrace on
  Android, NSLog on iOS (stderr is unreadable on device).

## Still open: IME composing on Android

Unchanged by the pivot and now the biggest functional gap. Under
NativeActivity, Android delivers key events only — no `InputConnection`
ever exists, so there is no composing text: CJK input, autocorrect and
swipe typing are impossible. iOS has the same shape of problem in winit's
UIKeyInput view (committed text only, no marked text). The real fix on
Android is eframe's `android-game-activity` feature — which is also the
only route to Android AccessKit — but `androidx.games:games-activity` is
a Java library: the APK gains a dex and loses `hasCode="false"`, and the
no-Gradle pipeline gains a packaging step. Recommendation: keep
NativeActivity as the framework default (keycode + dead-key accents
work), ship the GameActivity path as a documented opt-in, and take the
decision when a real app needs CJK; it sets what kind of apps the
framework can host.

## Open questions

1. **Layer on eframe, or fork it?** The layer now reaches past
   `raw_input_hook`: creation-time font and wgpu setup, lifecycle
   observers firing between frames through a main-thread pointer, the
   Close exit path, panic hooks. Each hole is small but non-standard —
   the pointer's 13-line safety argument is the kind of thing a fork
   would delete. Recommendation: stay a layer while the holes stay
   enumerable, push the generic ones upstream (winit 0.31's
   `Window::safe_area`, which egui-winit's own iOS safe-area module
   already names as its exit plan; eframe lifecycle hooks; Android
   IME), and write down the fork triggers before the list doubles
   again.
2. **Do `android.rs` / `ios.rs` generalize?** They are small because this
   app is small. The audit is concrete now: hard-coded are the app name,
   the data-dir fallback path, `$HOME/.rustnotes`, one live `Host`
   (the pointer the iOS observers hold), and save hooks typed to
   `save_current`. The durability behaviour — debounce delay, in-memory
   degrade, error strips — is app policy; the framework's product is the
   hooks (save moments, `on_back`, `filter_input`, zoom/inset injection),
   not the policy. Answering this question is the first PR (below).
3. **Does `layout/` come out too?** Not yet. Per
   `mobile-layout-system-plan.md`: "Extract it to `crates/mobile-layout`
   only when a real second consumer appears." The module is being
   prepared anyway — it doubled with generic machinery (id salts,
   retargeted transitions, WCAG-tested palette) and its remaining app
   concepts carry `TODO(framework-extraction)` markers (icon painters,
   `&'static str` labels). The second consumer and this question resolve
   together.
4. **IME composing** — see above.
5. **Name and home.** Crate names are unsettled; this repo (renamed) vs a
   new repo with Rust Notes as the example app.

## Sequence

No five phases; the questions order themselves:

1. **First PR — formalize the seam in place.** Define the `App` trait
   over the nine calls inside the current crates, make the host generic
   (`Host<A>`), and move the name/data-dir specifics behind it. No crate
   moves, no behaviour change; `just check` and the 61 tests stay green,
   the per-device screenshots (a manual comparison today) identical.
   This answers Q2 with code and surfaces every assumption the audit
   would otherwise miss.
2. **Second consumer.** A minimal second app — or the cargo-generate
   template — against the genericized host. This is the layout rule's
   trigger and the real test of Q2; extraction of the host crate (and
   `layout/`, if the rule fires) follows from what it finds.
3. **IME decision and endpoint.** Take Q4 with a real app's needs on the
   table, then the tooling/publish work below.

## Endpoint, unchanged by the pivot

A thin CLI over the packaging recipes (`run --android`, `run --ios`, and
a `screenshot` recipe the CLI would add — none exists today) and a
crates.io release of whatever crates survive the questions above — the
old plan's tooling/publish phase, which never depended on the shells.

## Not in scope

Bindings to native UI (this stays an egui framework), wasm/web, desktop
as a product target (desktop is a dev loop), and accessibility trees
(desktop-only in this stack: accesskit_winit has no iOS adapter, and on
Android eframe refuses the accesskit feature under NativeActivity —
GameActivity would unlock it, the same decision as IME).

## Addendum: the seam PRs, as built (2026-10-13)

Sequence steps 1 and 2 landed on `feat/framework-seam` (`d47bb27`) and
`feat/framework-second-consumer` (`bb19215`, stacked). What building them
found that the record above didn't predict:

- The trait carries four identity consts — name, desktop data dir,
  Android fallback dir, logcat tag — not the audit's name/data-dir pair:
  the log tag was a specific the Q2 list missed. iOS `$HOME/Documents`
  stayed host-side: platform convention, not app identity. And the impl
  for `rustnotes_core::App` lives in the host crate because the
  dependency arrow forces it there (core cannot name a host trait);
  `rustnotes-core` shipped unchanged.
- The iOS `HOST_PTR` thread-local cannot hold `*mut dyn App` — a trait
  with associated consts is not dyn-compatible. It stores a thin
  `*mut ()` to the host's `app` field, re-typed to `*mut A` inside the
  save-hook closure, which is monomorphized over that same `A` (the
  process hosts exactly one). The pointer's safety argument gained these
  clauses; none were lost.
- The nine calls held exactly. The entry points around them needed
  load-bearing turbofishes — `A` appears only in consts, so inference
  cannot recover it — an ergonomic debt to settle at extraction time
  (marker type vs an `AppSpec` value).
- The cdylib's `android_main` is app identity, not framework code. It
  compiles behind a default-on `android-entry` feature; a consumer must
  opt out or the two `#[no_mangle]` symbols collide at link — invisible
  to `cargo check`, and re-armed by feature unification in multi-package
  Android builds (NDK lld errors; macOS ld64 drops the duplicate
  silently). `just check-links` links both apps' cdylibs as the guard.
- `hello-mobile` — no `rustnotes-core` dependency, checked in all three
  shapes by `just check` — answers Q2 for structure. It does not answer
  packaging: the justfile's Android/iOS recipes carry Rust Notes'
  package identity, the next hard-coded surface (the endpoint CLI's
  problem). Q3 stays open — a hello screen is not "a real second
  consumer" for `layout/`.
