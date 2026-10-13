//! Crate root: shared app code plus the Android entry point.
//!
//! - `app.rs`   — the eframe `App` (state, responsive layout, i18n, theming)
//! - `fonts.rs` — Noto fallback fonts so Arabic / CJK render
//! - `theme.rs` — named Visuals presets (dark, light, system, Nord, Catppuccin)
//! - `main.rs`  — iOS and desktop entry (binary)
//! - this file  — Android entry (`android_main`, loaded as a cdylib)

pub mod app;
pub mod fonts;
pub mod theme;

// Compile every YAML file under ./locales into the binary. `t!("key")` is
// then a hash lookup, which is the only kind of work that belongs inside
// egui's per-frame `update`. Missing translations fall back to English.
rust_i18n::i18n!("locales", fallback = "en");

pub use app::MobileApp;

/// Human-readable name, also used by eframe for the persistence key.
pub const APP_NAME: &str = "mobile_app";

/// Build the eframe options shared by every platform.
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        // glow (OpenGL ES 3) on Android and desktop; wgpu (Metal) on iOS,
        // because eframe's glow path depends on glutin, which has no iOS
        // backend. The feature flags in Cargo.toml mirror this split.
        #[cfg(not(target_os = "ios"))]
        renderer: eframe::Renderer::Glow,
        #[cfg(target_os = "ios")]
        renderer: eframe::Renderer::Wgpu,
        // Mobile has no window chrome; eframe maximises on iOS by itself.
        viewport: egui::ViewportBuilder::default().with_title(APP_NAME),
        ..Default::default()
    }
}

/// Start the app with the given options. Blocks until the app exits.
pub fn run(options: eframe::NativeOptions) -> eframe::Result {
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(MobileApp::new(cc)))),
    )
}

/// Android entry point. NativeActivity dlopens `libmobile_app.so` and calls
/// this on its own thread; eframe/winit take over the event loop from here.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(APP_NAME)
            .with_max_level(log::LevelFilter::Info),
    );
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));

    let mut options = native_options();
    // winit needs the AndroidApp handle to build its event loop.
    options.android_app = Some(app);

    if let Err(e) = run(options) {
        log::error!("eframe exited with error: {e}");
    }
}
