//! Android platform glue that eframe/winit do not provide.

use android_activity::AndroidApp;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// NativeActivity loads `librustnotes.so` and calls this on its own thread.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("rustnotes")
            .with_max_level(log::LevelFilter::Info),
    );
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
    // The manifest's singleTask keeps the process cached after a clean
    // back-exit, so the next launch calls `android_main` again in this
    // same process; the per-activity statics must not survive that.
    reset_activity_state();

    let data_dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp"));
    set_app(app.clone());
    let mut options = crate::native_options();
    // winit needs the AndroidApp handle to build its event loop.
    options.android_app = Some(app);
    if let Err(e) = crate::run(options, data_dir) {
        log::error!("eframe exited with error: {e}");
    }
}

/// Clear the per-activity state cached in this module. `APP` is not listed:
/// `set_app` overwrites it right after this returns. The logger and panic
/// hook are per-process and re-initializing them is a no-op.
fn reset_activity_state() {
    *REPORT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *LAST_QUERY.lock().unwrap_or_else(|e| e.into_inner()) = None;
    // A query queued on the Java thread of a dead activity never completes;
    // left set, it would stop the new activity from querying at all.
    IN_FLIGHT.store(false, Ordering::Release);
    *BARS_LIGHT.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// A Mutex rather than a OnceLock: NativeActivity can call `android_main`
/// again in the same process after the activity is recreated, and the new
/// handle must win.
static APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

fn set_app(app: AndroidApp) {
    *APP.lock().unwrap_or_else(|e| e.into_inner()) = Some(app);
}

fn app() -> Option<AndroidApp> {
    APP.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// What the Java side reports: window insets in pixels, font scale and
/// night mode.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Report {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    font_scale: f32,
    /// `Configuration.uiMode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES`.
    night: bool,
}

static REPORT: Mutex<Option<Report>> = Mutex::new(None);
static LAST_QUERY: Mutex<Option<Instant>> = Mutex::new(None);
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
/// Insets change on rotation, keyboard show/hide and cutout changes; font
/// scale and night mode on a configuration change. All of those also produce
/// frames, so polling at frame time is enough.
const REQUERY: Duration = Duration::from_millis(250);

/// Feed egui what winit does not report on Android: safe-area insets (status
/// bar, navigation bar, cutout, keyboard), the night-mode appearance, and the
/// user's font scale.
pub(crate) fn raw_input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) {
    maybe_query(ctx);
    let ppp = ctx.native_pixels_per_point().unwrap_or(1.0).max(0.1);
    let report = *REPORT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = report {
        // Insets are window pixels; egui wants points before zoom.
        let zoom = ctx.zoom_factor().max(0.1);
        let pt = |px: i32| px.max(0) as f32 / ppp / zoom;
        raw.safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
            left: pt(r.left),
            top: pt(r.top),
            right: pt(r.right),
            bottom: pt(r.bottom),
        }));
        let scale = r.font_scale.clamp(0.8, 2.0);
        if (ctx.zoom_factor() - scale).abs() > 1e-3 {
            ctx.set_zoom_factor(scale);
        }
        // winit's Window::theme() is None on Android, and android-activity's
        // NDK configuration is not refreshed on configuration changes under
        // winit, so night mode comes from the same JNI report.
        let t = if r.night { egui::Theme::Dark } else { egui::Theme::Light };
        if ctx.options(|o| o.fallback_theme) != t {
            log::info!("night mode -> {t:?}");
            ctx.options_mut(|o| o.fallback_theme = t);
        }
    }
    sync_bar_icons(ctx.theme() == egui::Theme::Light);
}

/// Last bar style sent to the window; None until the first send.
static BARS_LIGHT: Mutex<Option<bool>> = Mutex::new(None);

/// Under enforced edge-to-edge the app draws behind the status and
/// navigation bars, so it must say whether their icons should be dark (on a
/// light UI) or light (on a dark UI). Sent only when the theme changes.
fn sync_bar_icons(light_ui: bool) {
    {
        let mut cur = BARS_LIGHT.lock().unwrap_or_else(|e| e.into_inner());
        if *cur == Some(light_ui) {
            return;
        }
        *cur = Some(light_ui);
    }
    let Some(app) = app() else { return };
    let app2 = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr().cast()) };
        let res = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let activity = unsafe {
                jni::objects::JObject::from_raw(env, app2.activity_as_ptr() as jni::sys::jobject)
            };
            let r = (|| -> jni::errors::Result<()> {
                let window = env
                    .call_method(
                        &activity,
                        jni::jni_str!("getWindow"),
                        jni::jni_sig!("()Landroid/view/Window;"),
                        &[],
                    )?
                    .l()?;
                // API 30+: WindowInsetsController. APPEARANCE_LIGHT_STATUS_BARS
                // (8) | APPEARANCE_LIGHT_NAVIGATION_BARS (16) mean "dark icons".
                let modern = (|| -> jni::errors::Result<()> {
                    let ctrl = env
                        .call_method(
                            &window,
                            jni::jni_str!("getInsetsController"),
                            jni::jni_sig!("()Landroid/view/WindowInsetsController;"),
                            &[],
                        )?
                        .l()?;
                    let mask = 8 | 16;
                    let value = if light_ui { mask } else { 0 };
                    env.call_method(
                        &ctrl,
                        jni::jni_str!("setSystemBarsAppearance"),
                        jni::jni_sig!("(II)V"),
                        &[jni::JValue::from(value), jni::JValue::from(mask)],
                    )?;
                    Ok(())
                })();
                if modern.is_err() {
                    // API < 30: SYSTEM_UI_FLAG_LIGHT_STATUS_BAR (0x2000) |
                    // SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR (0x10) on the decor view.
                    env.exception_clear();
                    let decor = env
                        .call_method(
                            &window,
                            jni::jni_str!("getDecorView"),
                            jni::jni_sig!("()Landroid/view/View;"),
                            &[],
                        )?
                        .l()?;
                    let flags: i32 = if light_ui { 0x2000 | 0x10 } else { 0 };
                    env.call_method(
                        &decor,
                        jni::jni_str!("setSystemUiVisibility"),
                        jni::jni_sig!("(I)V"),
                        &[jni::JValue::from(flags)],
                    )?;
                }
                Ok(())
            })();
            std::mem::forget(activity);
            r
        });
        if let Err(e) = res {
            log::warn!("system bar appearance failed: {e}");
        }
    }));
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
    // Views may only be touched on the Java main thread.
    app.run_on_java_main_thread(Box::new(move || {
        let res = query(&app2);
        IN_FLIGHT.store(false, Ordering::Release);
        match res {
            Ok(Some(r)) => {
                let mut cur = REPORT.lock().unwrap_or_else(|e| e.into_inner());
                if *cur != Some(r) {
                    log::info!("window report: {r:?}");
                    *cur = Some(r);
                    ctx.request_repaint();
                }
            }
            Ok(None) => {} // view not attached yet; try again next frame
            Err(e) => log::warn!("window query failed: {e}"),
        }
    }));
}

/// Android 15+ with targetSdk 35 enforces edge-to-edge: the content view
/// covers the whole window and `adjustResize` no longer makes room for the
/// keyboard. So read the root view's `WindowInsets`: system bars and cutout on
/// every side, and the larger of navigation bar and keyboard at the bottom.
/// API 24-29 fall back to the `getSystemWindowInset*` getters, which already
/// include the keyboard. `fontScale` comes from the configuration (the NDK
/// exposes no accessor for it).
fn query(app: &AndroidApp) -> jni::errors::Result<Option<Report>> {
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<Option<Report>> {
        let activity = unsafe {
            jni::objects::JObject::from_raw(env, app.activity_as_ptr() as jni::sys::jobject)
        };
        let r = (|| -> jni::errors::Result<Option<Report>> {
            let resources = env
                .call_method(
                    &activity,
                    jni::jni_str!("getResources"),
                    jni::jni_sig!("()Landroid/content/res/Resources;"),
                    &[],
                )?
                .l()?;
            let config = env
                .call_method(
                    &resources,
                    jni::jni_str!("getConfiguration"),
                    jni::jni_sig!("()Landroid/content/res/Configuration;"),
                    &[],
                )?
                .l()?;
            let font_scale = env
                .get_field(&config, jni::jni_str!("fontScale"), jni::jni_sig!("F"))?
                .f()?;
            let ui_mode = env
                .get_field(&config, jni::jni_str!("uiMode"), jni::jni_sig!("I"))?
                .i()?;
            let night = ui_mode & 0x30 == 0x20; // UI_MODE_NIGHT_MASK, UI_MODE_NIGHT_YES

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
            let modern = (|| -> jni::errors::Result<[i32; 4]> {
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
                Ok([left, top, right, bottom.max(kb_bottom)])
            })();
            let [left, top, right, bottom] = match modern {
                Ok(v) => v,
                Err(_) => {
                    // API < 30: WindowInsets$Type does not exist.
                    env.exception_clear();
                    [
                        env.call_method(&insets, jni::jni_str!("getSystemWindowInsetLeft"), jni::jni_sig!("()I"), &[])?
                            .i()?,
                        env.call_method(&insets, jni::jni_str!("getSystemWindowInsetTop"), jni::jni_sig!("()I"), &[])?
                            .i()?,
                        env.call_method(&insets, jni::jni_str!("getSystemWindowInsetRight"), jni::jni_sig!("()I"), &[])?
                            .i()?,
                        env.call_method(&insets, jni::jni_str!("getSystemWindowInsetBottom"), jni::jni_sig!("()I"), &[])?
                            .i()?,
                    ]
                }
            };
            Ok(Some(Report {
                left,
                top,
                right,
                bottom,
                font_scale,
                night,
            }))
        })();
        // The activity reference belongs to NativeActivity; never release it.
        std::mem::forget(activity);
        r
    })
}

/// Finish the activity — called from `on_exit`, after
/// `ViewportCommand::Close` has torn eframe's state down. The teardown this
/// starts gives winit the window event it needs to exit the event loop (and
/// with it `android_main`), after which the activity glue finishes and
/// destroys the activity without blocking the Java main thread. winit marks
/// every key as handled, so the system would never finish us on its own.
pub(crate) fn finish_activity() {
    let Some(app) = app() else { return };
    let app2 = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr().cast()) };
        let res = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            let activity = unsafe {
                jni::objects::JObject::from_raw(env, app2.activity_as_ptr() as jni::sys::jobject)
            };
            let r = env
                .call_method(&activity, jni::jni_str!("finish"), jni::jni_sig!("()V"), &[])
                .map(|_| ());
            std::mem::forget(activity);
            r
        });
        if let Err(e) = res {
            log::warn!("finish() failed: {e}");
        }
    }));
}
