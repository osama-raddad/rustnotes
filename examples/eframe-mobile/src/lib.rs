//! Crate root: shared app code plus the Android entry point.
//!
//! - `app.rs`   — the eframe `App` (state, responsive layout, i18n, theming)
//! - `fonts.rs` — Noto fallback fonts (Arabic / CJK glyph coverage)
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
    // eframe has no default storage directory on Android (its `storage_dir()`
    // returns None there), so without an explicit path `cc.storage` is None
    // and nothing persists. The app's private files dir survives relaunches.
    options.persistence_path = app.internal_data_path().map(|dir| dir.join("app.ron"));
    // Keep a handle for per-frame queries egui-winit does not forward
    // (content rect for safe-area insets, night mode for Theme::System).
    android::set_app(app.clone());
    // winit needs the AndroidApp handle to build its event loop.
    options.android_app = Some(app);

    if let Err(e) = run(options) {
        log::error!("eframe exited with error: {e}");
    }
}

/// Android platform state that egui-winit does not forward to egui.
#[cfg(target_os = "android")]
pub(crate) mod android {
    use android_activity::ndk::configuration::UiModeNight;
    use android_activity::AndroidApp;
    use std::sync::Mutex;

    /// Set in `android_main` before eframe starts. A Mutex rather than a
    /// OnceLock: NativeActivity can call `android_main` again in the same
    /// process after the activity is recreated, and the new handle must win.
    static APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

    pub(crate) fn set_app(app: AndroidApp) {
        *APP.lock().unwrap_or_else(|e| e.into_inner()) = Some(app);
    }

    fn app() -> Option<AndroidApp> {
        APP.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Safe-area insets in points, derived from NativeActivity's content
    /// rect (window pixels) versus the viewport. egui-winit only fills
    /// `RawInput::safe_area_insets` on iOS, so Android needs this.
    ///
    /// Limitation: the content rect is the content view's frame inside the
    /// window. On Android 15+ with targetSdk 35, edge-to-edge is enforced
    /// and the content view fills the whole window, so this reports zero
    /// insets for the status and navigation bars there. Getting real system
    /// bar insets on those devices needs JNI (`View.getRootWindowInsets`) or
    /// a theme with `windowOptOutEdgeToEdgeEnforcement`.
    pub(crate) fn safe_area_insets(screen: egui::Rect, ppp: f32) -> Option<egui::SafeAreaInsets> {
        let rect = app()?.content_rect();
        if rect.right <= rect.left || rect.bottom <= rect.top || ppp <= 0.0 {
            return None; // not reported yet
        }
        Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
            left: (rect.left as f32 / ppp).max(0.0),
            top: (rect.top as f32 / ppp).max(0.0),
            right: (screen.width() - rect.right as f32 / ppp).max(0.0),
            bottom: (screen.height() - rect.bottom as f32 / ppp).max(0.0),
        }))
    }

    /// The OS dark-mode setting. winit's `Window::theme()` is None on
    /// Android, so egui never learns it without asking the configuration.
    pub(crate) fn night_mode() -> Option<egui::Theme> {
        match app()?.config().ui_mode_night() {
            UiModeNight::Yes => Some(egui::Theme::Dark),
            UiModeNight::No => Some(egui::Theme::Light),
            _ => None,
        }
    }
}
