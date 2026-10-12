# Rust Notes

A minimal note-taking app for Android, written **entirely in Rust** — no Kotlin,
no Java, no Gradle. The app is a `cdylib` loaded by the framework's
`android.app.NativeActivity`; it creates its own OpenGL ES 3 context, renders
its own UI, and stores notes in SQLite.

```
┌─────────────────────────────────────────────────────────────┐
│ Android                                                     │
│  └─ NativeActivity (framework, no app Java code)            │
│      └─ librustnotes.so                                     │
│          ├─ android-activity  lifecycle + input (NativeActivity glue) │
│          ├─ EGL (FFI)       GLES3 context on the ANativeWindow       │
│          ├─ glow            shaders: colored quads + glyph atlas     │
│          ├─ fontdue         Roboto loaded from /system/fonts at runtime │
│          └─ rusqlite        notes.db in the app's internal dir (bundled SQLite) │
└─────────────────────────────────────────────────────────────┘
```

## Features

- Note list with title (first line), snippet, and dates
- Create / edit / delete (tap ✕ twice within 2 s to confirm)
- Word-wrapped plain-text editor with blinking cursor, tap-to-place-cursor
- Scrolling list and editor, auto-scroll to keep the cursor visible
- Notes persisted in SQLite across restarts (autosave on pause/destroy)
- Correct status-bar inset handling (queried once via JNI)

## Prerequisites

- Rust (`rustup target add aarch64-linux-android`)
- Android SDK with build-tools 35, platform android-35, platform-tools
- Android NDK r27+ (`~/Library/Android/sdk/ndk/27.1.12297006`)
- Java (for `keytool`/`apksigner`)

Paths are defaulted for a macOS install under `~/Library/Android/sdk`; override
with `ANDROID_HOME` in `build.sh` if needed. The NDK linker path and the
16 KB-page-alignment flag live in `.cargo/config.toml`.

## Build & run

```sh
./build.sh            # -> rustnotes.apk (arm64-v8a, minSdk 24, targetSdk 35)
./build.sh install    # build + install + launch on a connected device
```

The script:

1. Cross-compiles the `cdylib` with the NDK clang as linker (also builds the
   bundled SQLite C code via `cc`).
2. Packages the manifest + `lib/arm64-v8a/librustnotes.so` with `aapt2`
   (`android:hasCode="false"` — the APK contains no dex at all).
3. `zipalign`s and signs with a generated local `debug.keystore`.

Logs go to logcat under the tag `rustnotes`:

```sh
adb logcat -s rustnotes
```

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
| `src/lib.rs` | App state machine, event loop, input handling, UI layout/drawing |
| `src/egl.rs` | Hand-rolled EGL 1.4 FFI (context/surface management) |
| `src/gl.rs` | Immediate-mode GLES3 renderer + fontdue glyph atlas |
| `src/text.rs` | Font loading, measurement, word wrap, cursor mapping |
| `src/notes.rs` | SQLite persistence, note model, title/snippet helpers |
| `src/util.rs` | Epoch → "Oct 12" date formatting (no chrono) |
| `AndroidManifest.xml` | Single NativeActivity, no code, extractNativeLibs |
| `build.sh` | cargo → aapt2 → zipalign → apksigner pipeline |
