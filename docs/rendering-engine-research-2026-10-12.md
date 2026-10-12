# Rendering engine research — 2026-10-12

Question: is there an open-source engine that can replace the hand-rolled
renderer and text pipeline in `rustnotes-core`?

Survey date: 2026-10-12. Versions and dates are as published on crates.io /
GitHub on that day.

## What would be replaced

- `crates/rustnotes-core/src/gl.rs` (~500 lines): immediate-mode GLES3 via
  `glow` 0.14 — `fill_rect`, `fill_circle`, `fill_round_rect`, `line`, and
  text through a fontdue glyph atlas (one 512 px R8 texture).
- `crates/rustnotes-core/src/text.rs` (~190 lines): word wrap
  (`layout_text`), `cursor_geometry`, `hit_to_cursor`.
- Optionally the list/editor widgets in `lib.rs` (~700 lines).

## Constraints the evaluation was run against

- Pure Rust. No Kotlin, Java, Swift, Objective-C or C++ sources; no Gradle,
  no Xcode project.
- Android (NativeActivity + hand-rolled EGL) and iOS (objc2 + CAEAGLLayer /
  EAGL) shells each create a GLES3 context and hand the core a
  `glow::Context` built with `glow::Context::from_loader_function`.
- The integration seam is `Renderer::new(gl, w, h)` plus a per-frame
  `begin()` → draw calls → `finish()`. On iOS the shell binds its own FBO
  around the frame.
- Fonts are loaded at runtime from `/system/fonts/Roboto-Regular.ttf` and
  `/System/Library/Fonts/SFNS.ttf`; the bundle carries no font file.
- Anything that needs `wgpu` means replacing the GL context setup on both
  platforms (Metal surface on iOS, Vulkan/GLES surface on Android).

## Recommendation: egui + egui_glow

- Repo: https://github.com/emilk/egui — MIT OR Apache-2.0 — egui /
  egui_glow 0.36.2 (2026-09-08), actively maintained.
- **Verified against the 0.36.2 source and crates.io:**
  - `egui_glow::Painter::new(gl: Arc<glow::Context>, shader_prefix: &str,
    shader_version: Option<ShaderVersion>, dithering: bool)` — accepts an
    existing glow context. When `shader_version` is `None` it calls
    `ShaderVersion::get(&gl)`, which reads `GL_SHADING_LANGUAGE_VERSION`;
    passing `Some(ShaderVersion::Es300)` is the safe choice for our GLES3
    contexts.
  - `paint_and_update_textures(screen_size_px, pixels_per_point,
    &[ClippedPrimitive], &mut TexturesDelta)` is the per-frame call.
  - egui_glow depends on `glow ^0.17`; `winit` and `egui-winit` are
    optional and off by default. `painter.rs` never references winit.
- Integration shape: the shells keep their EGL/EAGL code, bump glow from
  0.14 to 0.17, and build an `egui::RawInput` from the events they already
  translate (`Event::Touch`, `PointerMoved`/`PointerButton`, `Text`, `Key`,
  `Event::Ime`). The README's "write your own integration" section and
  `examples/external_eventloop` cover this path.
- What stays in the shells: soft-keyboard show/hide and IME event
  translation. egui's `PlatformOutput.ime` and `mutable_text_under_cursor`
  signal when a text field wants input; map that to the existing JNI insets
  and hidden `UITextField` code. Known open egui issues on this:
  https://github.com/emilk/egui/issues/5590 (Android keyboard),
  https://github.com/emilk/egui/issues/2893 (mobile IME).
- Fonts: egui bundles no CJK font. Load Roboto / SFNS from the system paths
  into `FontDefinitions`. Noto Emoji (monochrome) is bundled.
- Mobile evidence: eframe lists Android (`examples/hello_android`,
  android-activity, no Java). iOS is not an official eframe target
  (https://github.com/emilk/egui/issues/3117), which does not matter here
  because eframe and egui-winit would not be used.
- Replaces: `gl.rs`, `text.rs`, and most of the widget code in `lib.rs`.
  Brings word wrap, selection, scrolling and a mature `TextEdit`.
- Binary size: egui + epaint + egui_glow only, no wgpu. Not measured; the
  surveying agent estimated roughly 1–3 MB stripped.

## Alternative: keep the quad batcher, replace only text

**parley + fontique + swash (Linebender)** — https://github.com/linebender/parley
— Apache-2.0 OR MIT — parley/fontique 0.12.0 (2026-10-09), swash 0.2.10
(2026-07-17).

- Shaping via harfrust, outlines via skrifa, bidi, CSS white-space, CJK line
  breaking (`complex-scripts` feature).
- `PlainEditor` / `PlainEditorDriver`: `move_to_point` (hit test), word and
  line motion, `select_word_at_point`, `extend_selection_to_point`,
  insert/delete, `set_compose` / `finish_compose` (IME), `cursor_geometry`,
  `selection_geometry`, `ime_cursor_area`
  (https://docs.rs/parley/latest/parley/editing/struct.PlainEditorDriver.html).
- Android font backend (`fontique/src/backend/android.rs`) scans
  `$ANDROID_ROOT/fonts` (default `/system/fonts`) and parses
  `/system/etc/fonts.xml` for aliases and per-script fallback. No NDK calls.
- Apple backend (`coretext.rs`, `target_vendor = "apple"`) enumerates fonts
  through objc2-core-text and scans `Library/Fonts` directories. Compiles for
  iOS (https://github.com/linebender/parley/pull/47); on-device enumeration
  was not verified. Fallback: `Collection::register_fonts` on
  `/System/Library/Fonts`.
- swash renders `Mask` (8-bit alpha) and `Color` (RGBA, colour emoji)
  images that drop straight into an R8 / RGBA GL atlas. No maintained glow
  wrapper exists; expect ~200 lines for an atlas (glyphon's etagere-based
  `TextAtlas` is a good model: https://github.com/grovesNL/glyphon).
- Users: Bevy (bevy_text moved from cosmic-text to parley), Slint, Blitz,
  Masonry/Xilem.
- Heavier dependency tree than cosmic-text (ICU4X data); API still moving
  (11 breaking releases).

**cosmic-text** — https://github.com/pop-os/cosmic-text — MIT OR Apache-2.0
— 0.19.0 (2026-04-22).

- `Buffer` (wrap, `hit(x,y)`, `cursor_motion`) and `Editor` (cursor,
  selection, actions); copy/paste listed as TODO, no IME compose API found.
- **No Android font discovery**: `fontdb::load_system_fonts()` skips android
  (https://github.com/pop-os/cosmic-text/issues/243, open since 2024), and
  the fallback module selected on Android has empty script/common fallback
  lists. Workaround: `db.load_fonts_dir("/system/fonts")` plus hand-written
  fallback lists (configurable since PR #369).
- iOS: fontdb scans `/System/Library/Fonts` and the MobileAsset font
  directories, but its fallback names are Noto families that iOS does not
  ship. Not verified on a device.
- Users: iced, Zed gpui, femtovg, floem, glyphon.

## Other 2D canvases considered

- **femtovg** — https://github.com/femtovg/femtovg — MIT OR Apache-2.0 —
  0.27.0 (2026-08-31). GL via glow 0.18 (plus optional wgpu). No constructor
  that takes a glow handle; it builds its own `Rc<glow::Context>` from a
  proc-address loader (`OpenGl::new_from_function`), so it is not `Send`
  and the shells would pass the loader instead. Shaders are `#version 100`
  and run on ES3. Text: `break_text_vec` wrapping and `measure_text` glyph
  positions, no editor or cursor helpers. Default text layout still uses
  rustybuzz + ttf-parser. No shipped mobile app found; Slint marks its
  femtovg module `cfg(not(android))`.
- **vello / vello_gpu / vello_cpu** — https://github.com/linebender/vello —
  Apache-2.0 OR MIT — all 0.x (2026-10-02). `vello` needs compute shaders
  (wgpu, WebGPU limits). `vello_gpu` (ex vello_hybrid) has wgpu and
  browser-only WebGL2 backends, no native GLES/glow path, and its README says
  not production-ready. `vello_cpu` is the mature one: CPU raster to an RGBA8
  `Pixmap`, SIMD on aarch64, text via glifo + parley. Upload cost is the
  price: a 1440×3120 RGBA8 frame is ~18 MB per redraw.
- **tiny-skia** — https://github.com/linebender/tiny-skia — BSD-3-Clause —
  0.12.0 (2026-02-02). CPU only, no text, no dirty rects. Would not reduce
  code for this app.
- **skia-safe / rust-skia** — https://github.com/rust-skia/rust-skia — MIT —
  0.153.3 (2026-09-04). Wraps an existing GLES context and SkParagraph gives
  full layout, but needs LLVM/clang, Python, Ninja and NDK at build time and
  links prebuilt C++ libraries: Android gl+textlayout 19.0 MB and iOS
  gl+metal+textlayout 15.1 MB compressed. Against the spirit of "no C++".
- **epaint** (egui's painter crate, usable without the widget layer),
  **lyon** (tessellation only), **vger** (wgpu-only, stale), **raqote**
  (CPU, no text, stale), **piet-glow** (maintenance mode): none beat the
  options above.

## Full toolkits considered

- **Slint** — https://github.com/slint-ui/slint — GPLv3 / Royalty-free 2.0
  / commercial — 1.18.1 (2026-09-21). Custom `slint::platform::Platform`
  can wrap an existing GL context through `femtovg_renderer::OpenGLInterface`,
  but that module is `cfg(not(android))`; on Android only Skia (C++ build)
  or the software renderer remain. The Android backend compiles
  `SlintAndroidJavaHelper.java` with javac + d8 at build time and ships it
  in the APK (https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/android/).
  iOS is a tech preview since 1.12 (Rust-only, winit + Skia)
  (https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/ios/).
  Royalty-free licence requires the `AboutSlint` widget or an attribution
  badge (https://slint.dev/terms-and-conditions.html).
- **Makepad** — https://github.com/makepad/makepad — MIT — 1.0.0 on
  crates.io (2025-05-13), git far ahead. Owns window, context and event loop
  (Metal on iOS, GL on Android), so it cannot consume our glow context. Best
  mobile text input of the survey (hidden native `UITextView` on iOS,
  `MakepadInputConnection` on Android), pure-Rust iOS packaging via
  `cargo makepad`, but the Android build compiles Makepad's Java activity
  sources into the APK.
- **iced** — https://github.com/iced-rs/iced — MIT — 0.14.0 (2025-12-07).
  wgpu / tiny-skia only, no mobile support
  (https://github.com/iced-rs/iced/issues/302).
- **Xilem/Masonry, Dioxus Native/Blitz, Freya, Floem, Ribir, gpui**: either
  wgpu-only, Skia-based, desktop-only, or without an iOS story.

## Not independently verified

The three candidate groups were surveyed by delegated research agents
reading crates.io, docs.rs and repository sources; only the egui_glow
constructor, its glow requirement and the winit optionality were re-checked
by hand. Treat the following as secondary-source claims until tested:

- fontique CoreText font enumeration on a real iOS device.
- femtovg running on Android or iOS at all.
- All binary-size figures except the skia-binaries release asset sizes.
- Feeding Slint's Skia renderer an existing GL context.
- Internal details of Slint's and Makepad's Android Java helpers (claim is
  from their repositories, not from a build on this machine).

## Ecosystem note

rustybuzz (RUSTSEC-2026-0206) and ttf-parser (RUSTSEC-2026-0192) were
marked unmaintained in mid-2026; cosmic-text and parley have moved to
harfrust + skrifa. fontdue 0.9.4 (2026-07-29) is alive but solo-maintained
and still depends on ttf-parser, so the current text path has a shelf life
independent of any renderer decision.
