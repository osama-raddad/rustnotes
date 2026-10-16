//! Rust Notes host: runs an [`App`] with eframe.
//!
//! eframe and winit own the window, the event loop, rendering and density.
//! This crate supplies only what they do not:
//!
//! - Android (`android.rs`): `android_main`, window insets, night mode,
//!   font scale, reduce motion and zone offset through JNI, and leaving
//!   the app on back at the root. Keyboard input is winit's under the
//!   default NativeActivity backend — with real limits: physical
//!   keycodes, basic-latin text and combining accents only. There is no
//!   IME composing or commit-text path on that backend, so CJK input,
//!   emoji panels and autocorrect never reach the app. The fix is
//!   shipped as an opt-in: the `android-game-activity` feature swaps the
//!   backend for GameActivity, whose GameTextInput gives winit commit
//!   text (see README "Android: the GameActivity opt-in"); NativeActivity
//!   stays the default because its keycode path works today.
//! - iOS (`ios.rs`): the system font through CoreText, safe-area and
//!   keyboard insets, appearance and Dynamic Type, background saving,
//!   wgpu limits that the iOS Metal devices meet.
//! - Desktop: nothing; `cargo run -p rustnotes` is the fast development loop.
//!
//! # The seam
//!
//! [`App`] — this crate's trait, not `eframe::App` (that one stays
//! qualified everywhere, as before) — is the framework's product: the
//! nine calls a host makes into the embedded app (`new`,
//! `set_edge_swipe_back`, `font_source`, `set_reduce_motion`,
//! `set_utc_offset_secs`, `filter_input`, `save_current`, `on_back`,
//! `ui`) plus the four identity consts (window title, desktop data dir,
//! Android data-dir fallback, logcat tag). `Host<A>`, [`run`],
//! [`native_options`] and `data_dir<A>` are generic over it, so a second
//! app implements the trait for its own type in its own crate —
//! `android::start` is its Android entry (the `android_main` symbol is
//! app identity, so it lives in the app's crate), while desktop and iOS
//! enter through [`run`] plus a few lines of the app's own main.rs. The
//! impl for `rustnotes_core::App` lives here and not in core because the
//! dependency arrow points host→core: core cannot name a host trait, let
//! alone implement it. Within this crate the concrete app is aliased to
//! `NotesApp`, keeping the bare name `App` for the trait — mirroring how
//! eframe names the thing it runs.

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "ios")]
mod ios;

use rustnotes_core::App as NotesApp;
#[cfg(target_os = "ios")]
use std::cell::Cell;
use std::path::PathBuf;

/// The host→app seam: everything the host may call on the app it runs.
///
/// The nine calls are exactly the ones this host makes into
/// `rustnotes-core` today; the four consts are the app-specific values
/// the platform glue needs. App policy — autosave debounce, error
/// surfacing, navigation — stays on the app side of the seam; the host
/// carries only the hooks (save moments, `on_back`, `filter_input`, the
/// `set_*` preferences) and never reaches past this trait.
pub trait App: 'static {
    /// Window title, and the app id `eframe::run_native` registers.
    const APP_NAME: &'static str;
    /// Desktop data dir under `$HOME`, leading dot included (".rustnotes").
    const DESKTOP_DATA_DIR: &'static str;
    /// Android fallback dir when `internal_data_path()` is `None`.
    const ANDROID_DATA_DIR_FALLBACK: &'static str;
    /// logcat tag.
    const ANDROID_LOG_TAG: &'static str;

    /// Open the app's storage in `data_dir` and set up the egui context;
    /// `platform_font` is the host-resolved system font, when the platform
    /// can name one.
    ///
    /// `Self: Sized` on purpose: construction is by-value and static, never
    /// through a trait object. (The trait as a whole is *not* dyn-compatible
    /// anyway — the identity consts above have no vtable entries — which is
    /// fine: identity is a compile-time fact, and the only type erasure the
    /// host needs, the iOS save hook, uses a thin pointer restored inside a
    /// monomorphized closure; see `HOST_PTR`.)
    fn new(
        data_dir: PathBuf,
        ctx: &egui::Context,
        platform_font: Option<(Vec<u8>, String)>,
    ) -> Self
    where
        Self: Sized;
    /// iOS only: whether a swipe from the left edge pops interactively.
    fn set_edge_swipe_back(&mut self, enabled: bool);
    /// Where the UI font came from, for logs and diagnostics.
    fn font_source(&self) -> &str;
    /// Honour the OS "reduce motion" preference.
    fn set_reduce_motion(&mut self, reduce: bool);
    /// Local zone offset east of UTC, as polled from the OS by the host.
    fn set_utc_offset_secs(&mut self, offset_secs: i32);
    /// Called from `raw_input_hook` every frame; the app records egui's
    /// clock and may claim or rewrite events.
    fn filter_input(&mut self, raw: &mut egui::RawInput);
    /// Save the current buffer, if any (focus loss, backgrounding, exit).
    fn save_current(&mut self);
    /// The platform back key; false when there is nothing to go back to,
    /// so the host may leave the app.
    fn on_back(&mut self) -> bool;
    /// Root of the UI for one frame.
    fn ui(&mut self, root: &mut egui::Ui);
}

/// Rust Notes as an [`App`] of this host: today's identity values,
/// delegating to the core's inherent methods (inherent impls win path
/// resolution, so each `NotesApp::…(self)` below is the core's method,
/// not the trait method being implemented — no recursion).
impl App for NotesApp {
    const APP_NAME: &'static str = "Rust Notes";
    const DESKTOP_DATA_DIR: &'static str = ".rustnotes";
    const ANDROID_DATA_DIR_FALLBACK: &'static str = "/data/data/com.osamaraddad.rustnotes/files";
    const ANDROID_LOG_TAG: &'static str = "rustnotes";

    fn new(
        data_dir: PathBuf,
        ctx: &egui::Context,
        platform_font: Option<(Vec<u8>, String)>,
    ) -> Self {
        NotesApp::new(data_dir, ctx, platform_font)
    }

    fn set_edge_swipe_back(&mut self, enabled: bool) {
        NotesApp::set_edge_swipe_back(self, enabled)
    }

    fn font_source(&self) -> &str {
        NotesApp::font_source(self)
    }

    fn set_reduce_motion(&mut self, reduce: bool) {
        NotesApp::set_reduce_motion(self, reduce)
    }

    fn set_utc_offset_secs(&mut self, offset_secs: i32) {
        NotesApp::set_utc_offset_secs(self, offset_secs)
    }

    fn filter_input(&mut self, raw: &mut egui::RawInput) {
        NotesApp::filter_input(self, raw)
    }

    fn save_current(&mut self) {
        NotesApp::save_current(self)
    }

    fn on_back(&mut self) -> bool {
        NotesApp::on_back(self)
    }

    fn ui(&mut self, root: &mut egui::Ui) {
        NotesApp::ui(self, root)
    }
}

#[cfg(target_os = "ios")]
thread_local! {
    /// Pointer to the live host's `app` field, erased to a thin `*mut ()`
    /// because a thread local cannot be generic over the host's `A`
    /// (statics admit no type parameters) and the trait is not
    /// dyn-compatible (its identity consts admit no vtable), so a fat
    /// pointer is not an option either. Re-established at the top of every
    /// frame (`eframe::App::ui`) on this (main) thread, and re-typed to
    /// `*mut A` only inside the save-hook closure, which statically knows
    /// `A` from its monomorphization. Only ever dereferenced there — see
    /// `Host::new` for the safety argument.
    static HOST_PTR: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
}

/// eframe adapter around the embedded app.
struct Host<A: App> {
    app: A,
}

impl<A: App> Host<A> {
    fn new(cc: &eframe::CreationContext<'_>, data_dir: PathBuf) -> Self {
        #[allow(unused_mut)]
        let mut app = A::new(data_dir, &cc.egui_ctx, platform_font());
        // iOS has no system back gesture for apps; the app supplies one.
        #[cfg(target_os = "ios")]
        {
            app.set_edge_swipe_back(true);
            // iOS never tells eframe the app is going to the background
            // (winit only emits Focused(false) on resignKeyWindow, and
            // on_exit is dead because UIApplicationMain never returns), so
            // the lifecycle observers in ios.rs call this hook to save
            // synchronously on WillResignActive / DidEnterBackground.
            let ctx = cc.egui_ctx.clone();
            ios::set_lifecycle_save_hook(move || {
                let app = HOST_PTR.with(Cell::get);
                if !app.is_null() {
                    // SAFETY: this closure runs on the UIKit main thread,
                    // from a lifecycle notification dispatched while
                    // eframe's event loop is parked inside
                    // UIApplicationMain *between* `App::ui` calls — no
                    // `&mut` borrow of the app is outstanding, and nothing
                    // else can touch it concurrently (the whole app is
                    // single-threaded through the main thread; this is the
                    // same main-thread TLS discipline ios.rs documents).
                    // The pointer was written by the most recent `ui()`
                    // call on this thread, and it targets the host's `app`
                    // field, which lives exactly as long as the `Host`
                    // holding it — boxed by `run_native` and alive until it
                    // returns, which on iOS it never does. It is erased to
                    // a thin `*mut ()` only because the thread local
                    // cannot carry `A`; this closure is monomorphized over
                    // the same `A` the host was built with, so the cast
                    // back to `*mut A` restores the exact type that was
                    // stored. That restore also relies on the process
                    // hosting exactly one `A` — one entry point, one
                    // `run_native`, so the `ui` that wrote this pointer
                    // was monomorphized over the same `A` this closure
                    // was; a second hosted type in one process would need
                    // a per-`A` channel. If eframe ever runs `ui` off the
                    // main thread or reenters it from the run loop, this
                    // breaks.
                    unsafe { &mut *(app as *mut A) }.save_current();
                }
                // Belt and braces: make sure a frame runs with the saved
                // state if the app returns to the foreground.
                ctx.request_repaint();
            });
        }
        log::info!("{} ready, font: {}", A::APP_NAME, app.font_source());
        Host { app }
    }
}

/// The system UI font, when the platform can name it. iOS devices have no
/// readable font at the core's candidate paths, so CoreText resolves it.
fn platform_font() -> Option<(Vec<u8>, String)> {
    #[cfg(target_os = "ios")]
    {
        ios::system_font_bytes()
    }
    #[cfg(not(target_os = "ios"))]
    {
        None
    }
}

impl<A: App> eframe::App for Host<A> {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        // The platform hooks hand back reduce-motion / UTC-offset values
        // only when they changed; applying them here (rather than inside
        // the platform modules) is what keeps them on the embedded app.
        #[cfg(target_os = "android")]
        {
            if let Some((reduce_motion, utc_offset_secs)) = android::raw_input_hook(ctx, raw) {
                self.app.set_reduce_motion(reduce_motion);
                self.app.set_utc_offset_secs(utc_offset_secs);
            }
        }
        #[cfg(target_os = "ios")]
        {
            if let Some((reduce_motion, utc_offset_secs)) = ios::raw_input_hook(ctx, raw) {
                self.app.set_reduce_motion(reduce_motion);
                self.app.set_utc_offset_secs(utc_offset_secs);
            }
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        let _ = ctx;
        self.app.filter_input(raw);
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        // Re-establish the pointer every frame for the iOS lifecycle save.
        // The two-step cast erases to a thin `*mut ()` (the static is not
        // generic and the trait is not dyn-compatible, so no fat pointer);
        // the save hook restores `*mut A` inside its monomorphized closure.
        #[cfg(target_os = "ios")]
        HOST_PTR.with(|p| p.set(&mut self.app as *mut A as *mut ()));
        // Losing focus is the last reliable save moment on Android (winit
        // maps LostFocus -> Focused(false) before Pause). It is NOT one on
        // iOS — backgrounding never resigns the key window there; the
        // lifecycle observers installed in `Host::new` cover iOS.
        if ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(false)))
        }) {
            self.app.save_current();
        }
        // Android's back key arrives as BrowserBack (winit's mapping).
        if ctx.input(|i| i.key_pressed(egui::Key::BrowserBack)) && !self.app.on_back() {
            leave_app(&ctx);
        }
        self.app.ui(root);
    }

    #[cfg(not(target_os = "ios"))]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.save_current();
    }

    #[cfg(target_os = "ios")]
    fn on_exit(&mut self) {
        self.app.save_current();
        // The lifecycle observers can no longer use the pointer.
        HOST_PTR.with(|p| p.set(std::ptr::null_mut()));
    }
}

/// Back at the root: hand control back to the OS, as native apps do. The
/// way out is eframe's, not JNI's: `ViewportCommand::Close` -> egui-winit
/// `ViewportEvent::Close` -> eframe `should_close()` -> `run_native`
/// returns -> `android_main` returns -> android-activity's glue calls
/// `ANativeActivity_finish()` and `onDestroy` unblocks. (Posting
/// `Activity.finish()` instead deadlocks the Java main thread: winit never
/// exits on `MainEvent::Destroy`, so `onDestroy` would block forever in the
/// glue and a singleTask relaunch of the cached process would hang.)
/// `on_exit` above persists the buffer when `run_native` returns. Residual
/// exposure: system-initiated destroys (don't-keep-activities) still need
/// winit to forward `MainEvent::Destroy`, which it does not — the process
/// is then simply killed, losing nothing here because Pause's
/// `WindowFocused(false)` save has already run.
#[cfg_attr(not(target_os = "android"), allow(unused_variables))]
fn leave_app(ctx: &egui::Context) {
    // Android-only, as before (the call site is shared; other platforms'
    // back-at-root is handled by the core or does not exist).
    #[cfg(target_os = "android")]
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}

/// Options shared by every platform.
pub fn native_options<A: App>() -> eframe::NativeOptions {
    eframe::NativeOptions {
        #[cfg(not(target_os = "ios"))]
        renderer: eframe::Renderer::Glow,
        #[cfg(target_os = "ios")]
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(A::APP_NAME)
            .with_inner_size([420.0, 860.0]),
        #[cfg(target_os = "ios")]
        wgpu_options: ios::wgpu_options(),
        ..Default::default()
    }
}

/// Where the app's data directory lives, on the platforms this binary
/// serves — Android resolves its own inside `android_main`.
#[cfg(not(target_os = "android"))]
pub fn data_dir<A: App>() -> PathBuf {
    // launchd always sets HOME in the app container; a silent fallback would
    // point at an unwritable path and fail every save with an opaque fs
    // error, so fail loudly instead.
    #[cfg(target_os = "ios")]
    let home = std::path::PathBuf::from(
        std::env::var("HOME").expect("HOME must be set in the app container"),
    );
    // Desktop: a missing HOME goes to the OS temp dir, not the process cwd
    // (which may be read-only or litter an unrelated directory).
    #[cfg(not(target_os = "ios"))]
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    if cfg!(target_os = "ios") {
        home.join("Documents") // the app container's Documents directory
    } else {
        home.join(A::DESKTOP_DATA_DIR) // desktop development data
    }
}

/// Start the app. Blocks until it exits.
pub fn run<A: App>(options: eframe::NativeOptions, data_dir: PathBuf) -> eframe::Result {
    // Earliest iOS-specific init: stderr is unreadable on device, so panics
    // must go through NSLog.
    #[cfg(target_os = "ios")]
    ios::install_panic_hook();
    eframe::run_native(
        A::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(Host::<A>::new(cc, data_dir)))),
    )
}
