# Rust Notes

A minimal note-taking app for Android **and iOS**, written **entirely in Rust** —
no Kotlin, no Java, no Swift, no Objective-C source, no Gradle, no Xcode
project. One shared core drives both platforms; each platform shell is a thin
layer over the OS windowing/input APIs.

> Session log: [docs/worklog-2026-10-12.md](docs/worklog-2026-10-12.md) — how
> this was built and debugged in one day, with every bug and measurement.

```
┌────────────────────────────────────────────────────────────────────┐
│                        rustnotes-core (shared)                     │
│  App state machine · UI layout/drawing · word wrap · SQLite notes  │
├────────────────────────────────┬───────────────────────────────────┤
│ Android shell                  │ iOS shell (100% Rust, objc2)      │
│  android-activity/NativeActivity│  UIApplicationMain from fn main() │
│  hand-rolled EGL + GLES3        │  define_class! app delegate + VC  │
│  key/motion translation         │  CAEAGLLayer + EAGL (typed FFI)   │
│  insets via async JNI query     │  touches + hidden UITextField     │
│  no dex, aapt2-packaged APK     │  CADisplayLink/NSTimer frame loop │
│                                 │  script-built .app, ad-hoc signed│
└────────────────────────────────┴───────────────────────────────────┘
```

## Features

- Note list with title (first line), snippet, and dates
- Create / edit / delete (tap ✕ twice within 2 s to confirm)
- Word-wrapped plain-text editor with blinking cursor, tap-to-place-cursor
- Scrolling list and editor, auto-scroll to keep the cursor visible
- Notes persisted in SQLite across restarts (autosave on pause/destroy)
- Correct status-bar/safe-area and keyboard inset handling on both platforms

## Prerequisites

- Rust (`rustup target add aarch64-linux-android`)
- Android SDK with build-tools 35, platform android-35, platform-tools
- Android NDK r27+ (`~/Library/Android/sdk/ndk/27.1.12297006`)
- Java (for `keytool`/`apksigner`)
- iOS device installs: a code-signing identity in the keychain **and** a
  provisioning profile for `com.osamaraddad.rustnotes.ios`. `just ios-install`
  auto-detects both — override with `IDENTITY=` / `PROFILE=` / `DEVICE=` when
  needed. Free personal-team profiles expire after 7 days: when that happens,
  `just ios-profile` rebuilds the bundled helper Xcode project
  (`ios/profile-helper/`) to mint a fresh one (iPhone connected, Apple ID
  signed in to Xcode).

Paths are defaulted for a macOS install under `~/Library/Android/sdk`; override
with `ANDROID_HOME` if needed. The NDK linker path and the
16 KB-page-alignment flag live in `.cargo/config.toml`.

## Build & run

The whole build lives in the [`justfile`](justfile) — no shell scripts, one
command surface ([`just`](https://github.com/casey/just), `brew install just`).
Run `just` alone to list recipes.

```sh
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
just check            # type-check all crates for their real targets
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
| Check all targets | `cargo check` for all three crates on their real targets |

The granular recipes (`just android`, `just ios-run`, `just ios-install`,
…) remain available as plain `just` commands in the terminal, and
`just ios-profile` refreshes the 7-day device profile when it expires.

Each config prepends `~/.cargo/bin`, the Android platform-tools, and Homebrew
to `PATH` before calling `just`, so they work even when the IDE is launched
from the Dock (GUI apps don't inherit your shell profile).

Logs: Android `adb logcat -s rustnotes`; iOS NSLog (visible in Console.app or
`xcrun simctl spawn booted log show --predicate 'process == "rustnotes"'`).

## Testing notes (emulator)

- Typing from the command line: `adb shell input text 'Hello%sworld'`
  (`%s` = space).
- The hardware BACK key: on API 33+ emulators, injected BACK is consumed by
  the system back dispatcher and never reaches native input. Use the on-screen
  ‹ button; on real devices with a legacy back press the key event is handled
  (returns to the list from the editor). Either way, edits autosave.

## Limitations (v1, by design)

- Text input is key-event based (NativeActivity provides no IME composing), so
  it works well for basic Latin; CJK/emoji and autocorrect are not supported.
  The idiomatic fix would be GameActivity's `TextInputState` or a thin Kotlin
  layer.
- arm64-v8a only (add more targets in `build.sh` for other ABIs).
- No text selection, no search, no themes — it's a demo of the pure-Rust
  stack, not a product.

## Project layout

| Path | Purpose |
|---|---|
| `crates/rustnotes-core/` | Shared: App state machine, neutral events, UI drawing, renderer, storage |
| `crates/rustnotes-android/` | Android shell: NativeActivity, EGL FFI, input translation, JNI insets |
| `crates/rustnotes-ios/` | iOS shell: objc2 classes, EAGL+CAEAGLLayer, keyboard, frame loop |
| `AndroidManifest.xml` | Single NativeActivity, no code, extractNativeLibs |
| `ios/Info.plist` | App bundle manifest (hand-assembled .app) |
| `justfile` | The single build file: cargo → package pipelines for each platform |
| `.idea/runConfigurations/` | IntelliJ run configurations, one per just recipe |
