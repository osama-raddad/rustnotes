# eframe mobile template

A standalone starter for an `egui` + `eframe` 0.36 app that targets Android
and iOS, kept here as a reference next to Rust Notes. It is its own Cargo
workspace (see the `[workspace]` table in `Cargo.toml`), so the root
`just check` does not build it.

Compared to Rust Notes, which drives `egui_glow` from hand-rolled EGL/EAGL
shells, this template lets `eframe` + `winit` own the window and event loop.
That is less code but less control: the Android soft keyboard through winit
is still an open egui issue (emilk/egui#5590), and iOS is not an officially
listed eframe target.

| File | Role |
|---|---|
| `src/lib.rs` | Shared setup, `native_options()`, `run()`, Android `android_main` |
| `src/main.rs` | iOS and desktop binary entry |
| `src/app.rs` | `MobileApp`: persisted state, portrait/landscape layout, `t!()` strings |
| `src/fonts.rs` | Noto fallback fonts: glyph coverage for Arabic and CJK |
| `src/theme.rs` | Named `Visuals` presets (system, light, dark, Nord, Catppuccin) |
| `locales/app.yml` | rust-i18n v2 translations (en, ar, zh-CN) |
| `android/AndroidManifest.xml` | NativeActivity manifest with `configChanges` set |

## Decisions baked in

- **Renderer per platform.** glow (OpenGL ES 3) on Android and desktop; wgpu
  (Metal) on iOS, because eframe's glow path depends on glutin, which has no
  iOS backend. `Cargo.toml` selects the feature per target and `lib.rs`
  picks the matching `Renderer` with a `cfg`.
- **No `accesskit`.** eframe's default feature set turns it on, and it
  refuses to build with NativeActivity on Android.
- **No egui-thematic.** Its 0.1.1 release pins egui 0.33 and depends on
  `rfd`, which has no Android backend. `theme.rs` covers presets natively and
  marks the one-line swap point.
- **Fonts are embedded.** Drop real files into `assets/fonts/` (see the
  README there). The loader skips empty placeholders with a warning so the
  crate compiles before you do. Latin comes from eframe's default fonts;
  Noto is only added for Arabic and CJK.
- **Arabic is a coverage demo, not RTL support.** Noto Sans Arabic gives the
  glyphs, but egui 0.36 has no bidi or shaping pass, so Arabic strings are
  laid out left-to-right in logical order and read wrong. The `ar` locale is
  kept to show the font fallback working.
- **Safe area and system theme on mobile.** egui-winit only reports safe-area
  insets on iOS and winit reports no OS theme on Android or iOS. On Android
  `MobileApp::raw_input_hook` reads the root view's `WindowInsets` through
  JNI (system bars, display cutout, and the keyboard, which `adjustResize` no
  longer handles under Android 15+'s enforced edge-to-edge) and follows the
  configuration's night mode. On iOS the `System` theme falls back to dark.
- **Persistence path set explicitly.** eframe has no default storage
  directory on Android or iOS, so `android_main` points it at the app's
  internal data dir and `main.rs` at `$HOME/Documents` on iOS.

## Build

Needs rustc 1.95 or newer (`rust-toolchain.toml` pins 1.99.0).

```sh
cd examples/eframe-mobile
cargo run                                              # desktop, fastest loop
cargo check --lib --target aarch64-linux-android       # needs NDK clang in CC_aarch64_linux_android
cargo check --bin mobile_app --target aarch64-apple-ios-sim
```

Packaging follows the same recipes as Rust Notes: `aapt2`/`apksigner` or
xbuild for the APK, a hand-assembled `.app` plus `codesign` for iOS.
