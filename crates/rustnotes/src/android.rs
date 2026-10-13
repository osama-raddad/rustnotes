//! Android platform glue that eframe/winit do not provide.

use android_activity::ndk::configuration::UiModeNight;
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

/// What the Java side reports: window insets in pixels and the font scale.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Report {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    font_scale: f32,
}

static REPORT: Mutex<Option<Report>> = Mutex::new(None);
static LAST_QUERY: Mutex<Option<Instant>> = Mutex::new(None);
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
/// Insets change on rotation, keyboard show/hide and cutout changes, and font
/// scale on a configuration change; all of those also produce frames, so
/// polling at frame time is enough.
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
    }
    // winit's Window::theme() is None on Android; ask the configuration.
    if let Some(a) = app() {
        let theme = match a.config().ui_mode_night() {
            UiModeNight::Yes => Some(egui::Theme::Dark),
            UiModeNight::No => Some(egui::Theme::Light),
            _ => None,
        };
        if let Some(t) = theme {
            if ctx.options(|o| o.fallback_theme) != t {
                ctx.options_mut(|o| o.fallback_theme = t);
            }
        }
    }
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
            }))
        })();
        // The activity reference belongs to NativeActivity; never release it.
        std::mem::forget(activity);
        r
    })
}

/// Back at the root of the app: finish the activity like a native app does.
/// winit marks every key as handled, so the system would never do this.
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
