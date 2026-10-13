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
    // (window insets via JNI for the safe area, night mode for Theme::System).
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

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

    /// Window insets in pixels, as last reported by the platform.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct InsetsPx {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    static INSETS: Mutex<Option<InsetsPx>> = Mutex::new(None);
    static LAST_QUERY: Mutex<Option<Instant>> = Mutex::new(None);
    static IN_FLIGHT: AtomicBool = AtomicBool::new(false);

    /// How often to re-ask while frames are being drawn. Insets change on
    /// rotation, keyboard show/hide and cutout changes, all of which also
    /// produce frames, so polling at frame time is enough.
    const REQUERY: Duration = Duration::from_millis(250);

    /// Safe-area insets in points for `RawInput::safe_area_insets`.
    ///
    /// egui-winit only fills them on iOS. On Android the reliable source is
    /// the root view's `WindowInsets`: with targetSdk 35 Android 15+
    /// enforces edge-to-edge, so the content view (and NativeActivity's
    /// content rect) covers the whole window, and `adjustResize` no longer
    /// shrinks it for the keyboard. The query therefore takes the union of
    /// system bars and display cutout on every side, and the larger of the
    /// navigation bar and the keyboard at the bottom.
    ///
    /// The query runs on the Java main thread (views may only be touched
    /// there); the result is cached and a repaint requested when it changes,
    /// so the first frame may lay out without insets for one frame.
    pub(crate) fn safe_area_insets(
        ctx: &egui::Context,
        ppp: f32,
    ) -> Option<egui::SafeAreaInsets> {
        maybe_query(ctx);
        let px = (*INSETS.lock().unwrap_or_else(|e| e.into_inner()))?;
        if ppp <= 0.0 {
            return None;
        }
        Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
            left: px.left.max(0) as f32 / ppp,
            top: px.top.max(0) as f32 / ppp,
            right: px.right.max(0) as f32 / ppp,
            bottom: px.bottom.max(0) as f32 / ppp,
        }))
    }

    fn maybe_query(ctx: &egui::Context) {
        if IN_FLIGHT.load(Ordering::Acquire) {
            return;
        }
        let now = Instant::now();
        {
            let mut last = LAST_QUERY.lock().unwrap_or_else(|e| e.into_inner());
            if last.is_some_and(|t| now.duration_since(t) < REQUERY) {
                return;
            }
            *last = Some(now);
        }
        let Some(app) = app() else { return };
        IN_FLIGHT.store(true, Ordering::Release);
        let ctx = ctx.clone();
        let app2 = app.clone();
        app.run_on_java_main_thread(Box::new(move || {
            let res = query_insets(&app2);
            IN_FLIGHT.store(false, Ordering::Release);
            match res {
                Ok(Some(px)) => {
                    let mut cur = INSETS.lock().unwrap_or_else(|e| e.into_inner());
                    if *cur != Some(px) {
                        log::info!("window insets: {px:?}");
                        *cur = Some(px);
                        ctx.request_repaint();
                    }
                }
                Ok(None) => {} // view not attached yet; try again next frame
                Err(e) => log::warn!("window insets query failed: {e}"),
            }
        }));
    }

    /// `decorView.getRootWindowInsets()`, read on the Java main thread.
    /// API 30+: `WindowInsets.Type` masks. API 24-29: the deprecated
    /// `getSystemWindowInset*` getters, which already include the keyboard.
    fn query_insets(app: &AndroidApp) -> jni::errors::Result<Option<InsetsPx>> {
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
        vm.attach_current_thread(|env| -> jni::errors::Result<Option<InsetsPx>> {
            let activity = unsafe {
                jni::objects::JObject::from_raw(env, app.activity_as_ptr() as jni::sys::jobject)
            };
            let r = (|| -> jni::errors::Result<Option<InsetsPx>> {
                let window = env
                    .call_method(
                        &activity,
                        jni::jni_str!("getWindow"),
                        jni::jni_sig!("()Landroid/view/Window;"),
                        &[],
                    )?
                    .l()?;
                let decor = env
                    .call_method(
                        &window,
                        jni::jni_str!("getDecorView"),
                        jni::jni_sig!("()Landroid/view/View;"),
                        &[],
                    )?
                    .l()?;
                let insets = env
                    .call_method(
                        &decor,
                        jni::jni_str!("getRootWindowInsets"),
                        jni::jni_sig!("()Landroid/view/WindowInsets;"),
                        &[],
                    )?
                    .l()?;
                if insets.is_null() {
                    return Ok(None);
                }

                // API 30+.
                let modern = (|| -> jni::errors::Result<InsetsPx> {
                    let types = env.find_class(jni::jni_str!("android/view/WindowInsets$Type"))?;
                    let bars = env
                        .call_static_method(&types, jni::jni_str!("systemBars"), jni::jni_sig!("()I"), &[])?
                        .i()?;
                    let cutout = env
                        .call_static_method(&types, jni::jni_str!("displayCutout"), jni::jni_sig!("()I"), &[])?
                        .i()?;
                    let ime = env
                        .call_static_method(&types, jni::jni_str!("ime"), jni::jni_sig!("()I"), &[])?
                        .i()?;
                    let sys = env
                        .call_method(
                            &insets,
                            jni::jni_str!("getInsets"),
                            jni::jni_sig!("(I)Landroid/graphics/Insets;"),
                            &[jni::JValue::from(bars | cutout)],
                        )?
                        .l()?;
                    let kb = env
                        .call_method(
                            &insets,
                            jni::jni_str!("getInsets"),
                            jni::jni_sig!("(I)Landroid/graphics/Insets;"),
                            &[jni::JValue::from(ime)],
                        )?
                        .l()?;
                    let left = env.get_field(&sys, jni::jni_str!("left"), jni::jni_sig!("I"))?.i()?;
                    let top = env.get_field(&sys, jni::jni_str!("top"), jni::jni_sig!("I"))?.i()?;
                    let right = env.get_field(&sys, jni::jni_str!("right"), jni::jni_sig!("I"))?.i()?;
                    let bottom = env.get_field(&sys, jni::jni_str!("bottom"), jni::jni_sig!("I"))?.i()?;
                    let kb_bottom = env.get_field(&kb, jni::jni_str!("bottom"), jni::jni_sig!("I"))?.i()?;
                    Ok(InsetsPx {
                        left,
                        top,
                        right,
                        bottom: bottom.max(kb_bottom),
                    })
                })();
                match modern {
                    Ok(px) => Ok(Some(px)),
                    Err(_) => {
                        // API < 30: WindowInsets$Type does not exist.
                        env.exception_clear();
                        let left = env
                            .call_method(&insets, jni::jni_str!("getSystemWindowInsetLeft"), jni::jni_sig!("()I"), &[])?
                            .i()?;
                        let top = env
                            .call_method(&insets, jni::jni_str!("getSystemWindowInsetTop"), jni::jni_sig!("()I"), &[])?
                            .i()?;
                        let right = env
                            .call_method(&insets, jni::jni_str!("getSystemWindowInsetRight"), jni::jni_sig!("()I"), &[])?
                            .i()?;
                        let bottom = env
                            .call_method(&insets, jni::jni_str!("getSystemWindowInsetBottom"), jni::jni_sig!("()I"), &[])?
                            .i()?;
                        Ok(Some(InsetsPx { left, top, right, bottom }))
                    }
                }
            })();
            // The activity reference belongs to NativeActivity; never release it.
            std::mem::forget(activity);
            r
        })
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
