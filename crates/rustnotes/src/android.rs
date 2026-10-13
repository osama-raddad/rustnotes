//! Android platform glue that eframe/winit do not provide.

use android_activity::AndroidApp;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// NativeActivity loads `librustnotes.so` and calls this on its own thread.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    start::<rustnotes_core::App>(app);
}

/// Android bootstrap, generic over the embedded [`crate::App`]. Everything
/// here is framework — logger, panic hook, data-dir resolution, entry into
/// eframe; the app-specific values (logcat tag, data-dir fallback, window
/// title) ride the trait's consts.
pub(crate) fn start<A: crate::App>(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(A::ANDROID_LOG_TAG)
            .with_max_level(log::LevelFilter::Info),
    );
    // Capture the backtrace too: the default hook prints it to stderr, which
    // this hook replaces — keep the only debugging signal a release panic
    // on device has.
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("panic: {info}\n{backtrace}");
    }));

    // `internalDataPath` is set before `android_main` runs; the trait's
    // fallback is that same directory for this package and stays writable
    // by the app uid (`/data/local/tmp`, the old fallback, is owned by
    // shell and would fail every DB operation).
    let data_dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from(A::ANDROID_DATA_DIR_FALLBACK));
    set_app(app.clone());
    // Prime the config query now, so insets/night/font-scale are already
    // known when eframe draws its first frames: the answer arrives
    // asynchronously on the Java main thread, otherwise those first frames
    // render with no safe-area insets at all.
    post_query(None);
    let mut options = crate::native_options::<A>();
    // winit needs the AndroidApp handle to build its event loop.
    options.android_app = Some(app);
    if let Err(e) = crate::run::<A>(options, data_dir) {
        log::error!("eframe exited with error: {e}");
    }
}

/// A Mutex rather than a OnceLock: NativeActivity can call `android_main`
/// again in the same process after the activity is recreated, and the new
/// handle must win.
static APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

fn set_app(app: AndroidApp) {
    // Drop the superseded handle explicitly (it owns an eventfd); the
    // closures already posted hold their own clones, so dropping this one
    // cannot pull the handle out from under them.
    let old = APP.lock().unwrap_or_else(|e| e.into_inner()).replace(app);
    drop(old);
    // A fresh activity must not inherit the previous session's state:
    // REPORT would apply stale insets/theme/font until a requery lands,
    // BARS_LIGHT would suppress the new window's first bar-icon sync, a
    // stranded IN_FLIGHT would disable reporting forever, and HANDED /
    // OS_SCALE would skip handing the (possibly changed) values to the app.
    *REPORT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *LAST_QUERY.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *IN_FLIGHT_SINCE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *BARS_LIGHT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *HANDED.lock().unwrap_or_else(|e| e.into_inner()) = None;
    IN_FLIGHT.store(false, Ordering::Release);
    OS_SCALE.with(|o| o.set(1.0));
}

fn app() -> Option<AndroidApp> {
    APP.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// What the Java side reports: window insets in pixels, font scale, night
/// mode, the animator scale and the zone offset.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Report {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    font_scale: f32,
    /// `Configuration.uiMode & UI_MODE_NIGHT_MASK == UI_MODE_NIGHT_YES`.
    night: bool,
    /// `Settings$Global.animator_duration_scale == 0` means the user asked
    /// for reduced motion.
    reduce_motion: bool,
    /// `TimeZone.getDefault().getOffset(System.currentTimeMillis())`,
    /// seconds east of UTC.
    utc_offset_secs: i32,
}

static REPORT: Mutex<Option<Report>> = Mutex::new(None);
static LAST_QUERY: Mutex<Option<Instant>> = Mutex::new(None);
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
/// When the currently in-flight query was posted; used to re-arm
/// `IN_FLIGHT` if the posted closure never executes.
static IN_FLIGHT_SINCE: Mutex<Option<Instant>> = Mutex::new(None);
/// Reduce-motion / UTC-offset pair already handed to the host.
static HANDED: Mutex<Option<(bool, i32)>> = Mutex::new(None);
/// Insets change on rotation, keyboard show/hide and cutout changes; font
/// scale and night mode on a configuration change. All of those also produce
/// frames, so polling at frame time is enough. 100 ms bounds the staleness
/// of a change to one requery plus one frame; the query itself is a short
/// burst of cached-method JNI calls, so the cadence is cheap.
const REQUERY: Duration = Duration::from_millis(100);
/// A posted query whose closure never runs (eventfd write failure, or a
/// Java main thread blocked past any reasonable latency) would strand
/// `IN_FLIGHT` and disable reporting for the process; re-arm after this.
const IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(2);

thread_local! {
    /// Last OS font scale this host enforced as zoom (see `raw_input_hook`).
    static OS_SCALE: Cell<f32> = const { Cell::new(1.0) };
}

/// Feed egui what winit does not report on Android: safe-area insets (status
/// bar, navigation bar, cutout, keyboard), the night-mode appearance, and the
/// user's font scale — plus the reduce-motion preference and UTC offset,
/// returned to the host when they change.
pub(crate) fn raw_input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) -> Option<(bool, i32)> {
    maybe_query(ctx);
    let ppp = ctx.native_pixels_per_point().unwrap_or(1.0).max(0.1);
    let report = *REPORT.lock().unwrap_or_else(|e| e.into_inner());
    let Some(r) = report else {
        sync_bar_icons(ctx.theme() == egui::Theme::Light);
        return None;
    };
    let scale = r.font_scale.clamp(0.8, 2.0);
    // Snap zoom to the OS font scale only when the OS-reported scale itself
    // changed, so a user's pinch-zoom survives until the setting changes.
    // (Zoom remains the carrier for the font scale — egui has no separate
    // font-size input.)
    let zoom = if (OS_SCALE.with(Cell::get) - scale).abs() > 1e-3 {
        OS_SCALE.with(|o| o.set(scale));
        if (ctx.zoom_factor() - scale).abs() > 1e-3 {
            ctx.set_zoom_factor(scale);
        }
        // `set_zoom_factor` applies in this pass's begin_pass while
        // `zoom_factor()` still reports the old value; divide the insets by
        // the value this pass enforces.
        scale.max(0.1)
    } else {
        ctx.zoom_factor().max(0.1)
    };
    // Insets are window pixels; egui wants points before zoom.
    let pt = |px: i32| px.max(0) as f32 / ppp / zoom;
    raw.safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
        left: pt(r.left),
        top: pt(r.top),
        right: pt(r.right),
        bottom: pt(r.bottom),
    }));
    // winit does not expose the theme on Android, and the ndk crate does
    // not wrap font scale, so night mode and fontScale come from this one
    // JNI report (the glue does refresh its Configuration on
    // ConfigChanged, and the raw NDK has ACONFIGURATION_FONT_SCALE —
    // neither is reachable through the APIs eframe surfaces).
    let t = if r.night {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    if ctx.options(|o| o.fallback_theme) != t {
        log::info!("night mode -> {t:?}");
        ctx.options_mut(|o| o.fallback_theme = t);
    }
    sync_bar_icons(ctx.theme() == egui::Theme::Light);
    // Hand reduce-motion / UTC offset to the host when they change.
    let cur = (r.reduce_motion, r.utc_offset_secs);
    let mut handed = HANDED.lock().unwrap_or_else(|e| e.into_inner());
    if *handed != Some(cur) {
        *handed = Some(cur);
        drop(handed);
        Some(cur)
    } else {
        None
    }
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
        // Teardown guard: android-activity nulls the activity pointer during
        // the destroy pass, but queued runnables may still execute after it;
        // touching the stale jobject then would be use-after-free.
        if app2.activity_as_ptr().is_null() {
            return;
        }
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
            // The activity jobject is a *borrowed global ref* owned by the
            // AndroidApp: jni 0.22's JObject has no Drop (never deletes
            // refs), so dropping it is a no-op — this forget is
            // defense-in-depth against a future owning wrapper, which would
            // delete the borrowed global and abort ART.
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
        let wedged = IN_FLIGHT_SINCE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|t| t.elapsed() > IN_FLIGHT_TIMEOUT);
        if !wedged {
            return;
        }
        log::warn!("window query wedged >{IN_FLIGHT_TIMEOUT:?}; re-arming");
        IN_FLIGHT.store(false, Ordering::Release);
    }
    let now = Instant::now();
    {
        let mut last = LAST_QUERY.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|t| now.duration_since(t) < REQUERY) {
            return;
        }
        *last = Some(now);
    }
    post_query(Some(ctx.clone()));
}

/// Post the JNI query to the Java main thread. `ctx`, when given, gets a
/// repaint request once the report changes; the priming call from
/// `android_main` has no context yet.
fn post_query(ctx: Option<egui::Context>) {
    let Some(app) = app() else { return };
    IN_FLIGHT.store(true, Ordering::Release);
    *IN_FLIGHT_SINCE.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
    let app2 = app.clone();
    // Views may only be touched on the Java main thread.
    app.run_on_java_main_thread(Box::new(move || {
        let res = query(&app2);
        IN_FLIGHT.store(false, Ordering::Release);
        *IN_FLIGHT_SINCE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        match res {
            Ok(Some(r)) => {
                let changed = {
                    let mut cur = REPORT.lock().unwrap_or_else(|e| e.into_inner());
                    if *cur != Some(r) {
                        log::info!("window report: {r:?}");
                        *cur = Some(r);
                        true
                    } else {
                        false
                    }
                };
                if changed {
                    if let Some(ctx) = &ctx {
                        ctx.request_repaint();
                    }
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
/// include the keyboard. Night mode, font scale (the ndk crate wraps no
/// accessor for it), the animator scale and the zone offset all ride the same
/// trip: one consistent source, JNI.
fn query(app: &AndroidApp) -> jni::errors::Result<Option<Report>> {
    // Teardown guard: the activity is being destroyed and this queued
    // runnable arrived after it; there is nothing to query.
    if app.activity_as_ptr().is_null() {
        return Ok(None);
    }
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

            // `Settings$Global.getFloat(resolver, name, default)` never
            // throws (unlike the 2-arg variant's SettingNotFoundException).
            // The 3-arg overload has existed since API 17; minSdk is 24.
            let reduce_motion = (|| -> jni::errors::Result<bool> {
                let resolver = env
                    .call_method(
                        &activity,
                        jni::jni_str!("getContentResolver"),
                        jni::jni_sig!("()Landroid/content/ContentResolver;"),
                        &[],
                    )?
                    .l()?;
                let key = env.new_string("animator_duration_scale")?;
                let settings = env.find_class(jni::jni_str!("android/provider/Settings$Global"))?;
                let scale = env
                    .call_static_method(
                        &settings,
                        jni::jni_str!("getFloat"),
                        jni::jni_sig!("(Landroid/content/ContentResolver;Ljava/lang/String;F)F"),
                        &[
                            jni::JValue::from(&resolver),
                            jni::JValue::from(&key),
                            jni::JValue::from(1.0f32),
                        ],
                    )?
                    .f()?;
                Ok(scale == 0.0)
            }())
            .unwrap_or(false);

            // `TimeZone.getDefault().getOffset(System.currentTimeMillis())`
            // in seconds. Both lookups are failures-of-nothing classes; if
            // either errors the report still ships with a UTC default.
            let utc_offset_secs = (|| -> jni::errors::Result<i32> {
                let tz_class = env.find_class(jni::jni_str!("java/util/TimeZone"))?;
                let tz = env
                    .call_static_method(
                        &tz_class,
                        jni::jni_str!("getDefault"),
                        jni::jni_sig!("()Ljava/util/TimeZone;"),
                        &[],
                    )?
                    .l()?;
                let sys = env.find_class(jni::jni_str!("java/lang/System"))?;
                let now_ms = env
                    .call_static_method(
                        &sys,
                        jni::jni_str!("currentTimeMillis"),
                        jni::jni_sig!("()J"),
                        &[],
                    )?
                    .j()?;
                let off_ms = env
                    .call_method(
                        &tz,
                        jni::jni_str!("getOffset"),
                        jni::jni_sig!("(J)I"),
                        &[jni::JValue::from(now_ms)],
                    )?
                    .i()?;
                Ok(off_ms / 1000)
            }())
            .unwrap_or(0);

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
                    .call_static_method(
                        &types,
                        jni::jni_str!("systemBars"),
                        jni::jni_sig!("()I"),
                        &[],
                    )?
                    .i()?;
                let cutout = env
                    .call_static_method(
                        &types,
                        jni::jni_str!("displayCutout"),
                        jni::jni_sig!("()I"),
                        &[],
                    )?
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
                let left = env
                    .get_field(&sys, jni::jni_str!("left"), jni::jni_sig!("I"))?
                    .i()?;
                let top = env
                    .get_field(&sys, jni::jni_str!("top"), jni::jni_sig!("I"))?
                    .i()?;
                let right = env
                    .get_field(&sys, jni::jni_str!("right"), jni::jni_sig!("I"))?
                    .i()?;
                let bottom = env
                    .get_field(&sys, jni::jni_str!("bottom"), jni::jni_sig!("I"))?
                    .i()?;
                let kb_bottom = env
                    .get_field(&kb, jni::jni_str!("bottom"), jni::jni_sig!("I"))?
                    .i()?;
                Ok([left, top, right, bottom.max(kb_bottom)])
            })();
            let [left, top, right, bottom] = match modern {
                Ok(v) => v,
                Err(_) => {
                    // API < 30: WindowInsets$Type does not exist.
                    env.exception_clear();
                    [
                        env.call_method(
                            &insets,
                            jni::jni_str!("getSystemWindowInsetLeft"),
                            jni::jni_sig!("()I"),
                            &[],
                        )?
                        .i()?,
                        env.call_method(
                            &insets,
                            jni::jni_str!("getSystemWindowInsetTop"),
                            jni::jni_sig!("()I"),
                            &[],
                        )?
                        .i()?,
                        env.call_method(
                            &insets,
                            jni::jni_str!("getSystemWindowInsetRight"),
                            jni::jni_sig!("()I"),
                            &[],
                        )?
                        .i()?,
                        env.call_method(
                            &insets,
                            jni::jni_str!("getSystemWindowInsetBottom"),
                            jni::jni_sig!("()I"),
                            &[],
                        )?
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
                reduce_motion,
                utc_offset_secs,
            }))
        })();
        // The activity jobject is a *borrowed global ref* owned by the
        // AndroidApp: jni 0.22's JObject has no Drop (never deletes refs),
        // so dropping it is a no-op — this forget is defense-in-depth
        // against a future owning wrapper, which would delete the borrowed
        // global and abort ART.
        std::mem::forget(activity);
        r
    })
}
