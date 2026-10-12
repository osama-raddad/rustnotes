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

Paths are defaulted for a macOS install under `~/Library/Android/sdk`; override
with `ANDROID_HOME` in `build.sh` if needed. The NDK linker path and the
16 KB-page-alignment flag live in `.cargo/config.toml`.

## Build & run

The entry point is [`just`](https://github.com/casey/just) (`brew install just`); run `just` alone to list recipes.

```sh
just android          # release APK (aapt2/apksigner pipeline, targetSdk 35)
just android-install  # + install & launch on a connected device
just android-x        # release APK via xbuild (auto-signed, targetSdk 33)
just android-run      # xbuild: deploy + launch + stream logs (Ctrl-C detaches)
just ios              # simulator .app bundle (ad-hoc signed)
just ios-run          # + install & launch on the booted simulator
just check            # type-check all crates for their real targets
```

The recipes wrap the underlying pipelines, which also work directly:

```sh
./build.sh            # Android APK: cargo -> aapt2 -> zipalign -> apksigner
./x.sh build --platform android --arch arm64 --release --format apk   # xbuild
./build-ios.sh aarch64-apple-ios-sim install   # iOS: cargo -> .app -> codesign -> simctl
```

Device target: `./build-ios.sh aarch64-apple-ios` (needs your signing identity to install).

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
| `build.sh` / `build-ios.sh` | cargo → package pipelines for each platform |
