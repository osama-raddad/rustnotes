# Rust Notes

A minimal note-taking app for Android **and iOS**, written **entirely in Rust** —
no Kotlin, no Java, no Swift, no Objective-C source, no Gradle, no Xcode
project. The app is pure egui; [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)
and winit host it on Android, iOS and desktop, and one small host crate adds
only what they lack on mobile.

> Session log: [docs/worklog-2026-10-12.md](docs/worklog-2026-10-12.md) — how
> this was built and debugged in one day, with every bug and measurement
> (day one: the hand-rolled EGL/fontdue stack; replaced next day by
> egui + eframe — PR #10).
>
> Renderer research: [docs/rendering-engine-research-2026-10-12.md](docs/rendering-engine-research-2026-10-12.md)
> — open-source engines evaluated to replace the hand-rolled GL renderer
> (recommendation: egui + egui_glow; parley + swash for a text-only swap;
> led to adopting egui — hosting later settled on eframe itself, PR #10).

```
┌──────────────────────────────────────────────────────────────────────┐
│ rustnotes-core — the app (pure egui, no platform code)               │
│  notes + SQLite · screens · layout/ (metrics, size classes, palette, │
│  scaffold, nav, bottom nav) · iOS edge-swipe input filter            │
├──────────────────────────────────────────────────────────────────────┤
│ rustnotes — the host (eframe + winit)                                │
│  window, event loop, GPU, density, soft keyboard: eframe/winit       │
│  Android: JNI window insets, font scale, night mode, bar icons,      │
│           finish() on back at the root   (glow / OpenGL ES 3)        │
│  iOS:     CoreText system font, safe area + keyboard inset,          │
│           appearance, Dynamic Type, wgpu limits   (wgpu / Metal)     │
│  desktop: cargo run -p rustnotes                  (glow)             │
└──────────────────────────────────────────────────────────────────────┘
```

## Features

- Note list with title (first line), snippet, and dates
- Create / edit / delete (tap ✕ twice within 2 s to confirm)
- Word-wrapped plain-text editor (egui `TextEdit`): blinking cursor,
  tap-to-place-cursor, drag selection
- Scrolling list and editor, auto-scroll to keep the cursor visible
- Notes persisted in SQLite across restarts — saved when the editor closes
  or switches, ~1.5 s after typing stops, on Android focus loss, and on iOS
  app backgrounding (`WillResignActive`/`DidEnterBackground` observers)
- Correct status-bar/safe-area and keyboard inset handling on both platforms
- Native-size layout: the OS pixels-per-point sets the scale, Android
  "Display size" / font scale and iOS Dynamic Type scale the UI, and the
  OS light/dark appearance is followed live
- Window size classes: tablets get a list–detail two-pane layout; phones in
  landscape stay single-pane
- Slide transitions between screens, an iOS edge-swipe back, swipe-to-delete,
  pull-to-refresh, and a bottom navigation bar with a Settings diagnostics
  tab (density, size class, appearance — and DB/storage errors)

## Prerequisites

- Rust 1.95+ (pinned to 1.99.0 by `rust-toolchain.toml`; egui 0.36 needs
  it) — the mobile targets (`aarch64-linux-android`, `aarch64-apple-ios`,
  `aarch64-apple-ios-sim`) are installed automatically by
  `rust-toolchain.toml`, so no manual `rustup target add` is needed
- Android SDK with build-tools 35, platform android-35, platform-tools
- Android NDK r27+ (`~/Library/Android/sdk/ndk/27.1.12297006`)
- Java (for `keytool`/`apksigner`)
- iOS device installs: a code-signing identity in the keychain **and** a
  provisioning profile for `com.osamaraddad.rustnotes.ios`. `just ios-install`
  auto-detects both — override with `IDENTITY=` / `PROFILE=` / `DEVICE=` when
  needed. Free personal-team profiles expire after 7 days: when that happens,
  `just ios-profile` rebuilds the bundled helper Xcode project
  (`tools/ios-profile-helper/`) to mint a fresh one (iPhone connected, Apple ID
  signed in to Xcode).

Paths are defaulted for a macOS install under `~/Library/Android/sdk`; override
with `ANDROID_HOME` if needed. The `justfile` exports the NDK linker (and
CC/AR) from `ANDROID_HOME`; `.cargo/config.toml` holds only the target dir and
the 16 KB-page-alignment link flag — so a direct
`cargo build --target aarch64-linux-android` outside `just` needs
`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` set.

The NDK path itself derives from `ANDROID_HOME`, but the packaging recipes
are still macOS-only: they call `keytool`, `security`, `plutil` and
`open -a Simulator`, and select the NDK's darwin toolchain tag. Only the
desktop `cargo run -p rustnotes` loop is cross-platform.

## Build & run

The whole build lives in the [`justfile`](justfile) — no shell scripts, one
command surface ([`just`](https://github.com/casey/just), `brew install just`).
Run `just` alone to list recipes.

```sh
cargo run -p rustnotes # desktop window: the fastest loop for UI work
just run-android       # connected device (physical first), else emulator (boots one if needed)
just run-ios           # connected iPhone, else simulator (boots one if needed)
just android          # release APK (aapt2/zipalign/apksigner pipeline, targetSdk 35)
just android-install  # + install & launch on a specific adb target
just android-x        # release APK via xbuild (auto-signed, targetSdk 33)
just android-run      # xbuild: deploy + launch + stream logs (Ctrl-C detaches)
just ios              # simulator .app bundle (ad-hoc signed)
just ios-run          # + install & launch on the booted simulator
just ios-device       # device .app (unsigned)
just ios-install      # device: sign + install + launch on a connected iPhone
just ios-profile      # refresh the 7-day device profile (see below)
just check            # type-check core, Android lib and iOS binary
just fmt              # cargo fmt --all
just lint             # fmt --check, then clippy on both crates
just test             # rustnotes-core unit tests
```

Underneath, the recipes drive plain `cargo` plus the platform packagers
(`aapt2`/`apksigner`, `codesign`/`simctl`, or xbuild) — no Gradle, no Xcode
project, no shell scripts.

### IntelliJ / JetBrains IDEs

Every recipe is also available as a run configuration in
[`.idea/runConfigurations/`](.idea/runConfigurations) (Shell Script type,
shared via git). Pick one from the Run/Debug dropdown:

| Configuration | What it does |
|---|---|
| Run on Android (just run-android) | physical device if connected → else emulator (boots the first AVD if needed) |
| Run on iOS (just run-ios) | connected iPhone (sign + install + launch) → else simulator (boots one if needed) |
| Check all targets | `cargo check` for both crates on their real targets (core, Android lib, iOS-sim binary, host desktop binary) |

The granular recipes (`just android`, `just ios-run`, `just ios-install`,
…) remain available as plain `just` commands in the terminal, and
`just ios-profile` refreshes the 7-day device profile when it expires.

Each config prepends `~/.cargo/bin`, the Android platform-tools, and Homebrew
to `PATH` before calling `just`, so they work even when the IDE is launched
from the Dock (GUI apps don't inherit your shell profile).

Logs: Android `adb logcat -s rustnotes`. iOS: panics go through an NSLog
panic hook (Console.app on a device); other log output is stderr via
`env_logger`, so view it attached to the process — Xcode, or
`xcrun simctl launch --console`.

## Testing notes (emulator)

- Typing from the command line: `adb shell input text 'Hello%sworld'`
  (`%s` = space).
- `adb shell input keyevent 4` sends BACK: the first press dismisses the
  keyboard, the next closes the editor, and at the list it finishes the app.
- Night mode and font scale: `adb shell cmd uimode night yes|no`,
  `adb shell settings put system font_scale 1.3`; both apply live.

## Limitations (v1, by design)

- Text input has no IME composing on either platform (NativeActivity on
  Android; winit's UIKeyInput view on iOS), so it works for basic Latin
  typing; CJK composition, autocorrect and emoji input are not supported.
  The real fix on Android is eframe's `android-game-activity` feature.
- The Android manifest lists every configuration change in `configChanges`.
  Leaving via back at the root is clean now — the eframe loop exits and
  `android_main` returns before the activity is finished — but a destroy
  the system starts itself still stalls: NativeActivity's `onDestroy`
  waits for `android_main` to return, which only our exit path ends, so
  relaunching the cached process after such a destroy can freeze the app.
- arm64-v8a only (add more ABIs in the `justfile`).
- No search, no clipboard, no user-selectable theme — it's a demo of the pure-Rust stack,
  not a product.
- Dragging inside the editor selects text (egui `TextEdit`); long notes
  scroll by moving the cursor, not by swiping.

## Project layout

Grouped by file type: source in `crates/`, packaging manifests in
`resources/`, generated output in `build/` (delete it to clean everything;
`rm -rf build` is always safe). Files the toolchains anchor to the root
(`Cargo.toml`/`Cargo.lock`, `justfile`, `.cargo/`, `.gitignore`, `.idea/`)
stay there; `crates/rustnotes/manifest.yaml` must sit next to its crate
because xbuild reads it from the working directory.

| Path | Purpose |
|---|---|
| `crates/rustnotes-core/` | The app, pure egui: notes + SQLite, screens, `layout/` (metrics, size classes, palette, scaffold, nav, bottom nav), edge-swipe input filter, unit tests |
| `crates/rustnotes/` | The eframe host: `src/lib.rs` (shared, `android_main` lives in `android.rs`), `src/main.rs` (iOS + desktop entry), `android.rs` (JNI), `ios.rs` (objc2/CoreText), `manifest.yaml` (xbuild) |
| `resources/AndroidManifest.xml` | Single NativeActivity, no code, extractNativeLibs |
| `resources/Info.plist` | App bundle manifest (hand-assembled .app) |
| `tools/ios-profile-helper/` | Throwaway Xcode project that mints 7-day provisioning profiles |
| `build/` | All generated artifacts: cargo target dir, APK staging + final APK, iOS .app |
| `debug.keystore` | Local debug signing key (auto-minted; kept at root so cleaning `build/` never invalidates installed apps) |
| `justfile` | The single build file: cargo → package pipelines for each platform |
| `.idea/runConfigurations/` | IntelliJ run configurations, one per just recipe |
