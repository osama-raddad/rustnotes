//! Rust Notes host: runs [`rustnotes_core::App`] with eframe.
//!
//! eframe and winit own the window, the event loop, rendering and density.
//! This crate supplies only what they do not:
//!
//! - Android (`android.rs`): `android_main`, window insets, night mode,
//!   font scale, reduce motion and zone offset through JNI, and leaving
//!   the app on back at the root. Keyboard input is winit's under
//!   NativeActivity — with real limits: physical keycodes, basic-latin
//!   text and combining accents only. There is no IME composing or
//!   commit-text path on this backend, so CJK input, emoji panels and
//!   autocorrect never reach the app. The real fix is eframe's
//!   `android-game-activity` feature (GameActivity supports commit text
//!   through winit); until that switch, the limitation is accepted.
//! - iOS (`ios.rs`): the system font through CoreText, safe-area and
//!   keyboard insets, appearance and Dynamic Type, background saving,
//!   wgpu limits that the iOS Metal devices meet.
//! - Desktop: nothing; `cargo run -p rustnotes` is the fast development loop.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use rustnotes_core::App;
#[cfg(target_os = "ios")]
use std::cell::Cell;
use std::path::PathBuf;

pub const APP_NAME: &str = "Rust Notes";

#[cfg(target_os = "ios")]
thread_local! {
    /// Pointer to the live `Host`, re-established at the top of every
    /// `App::ui` call on this (main) thread. Only ever dereferenced from
    /// the lifecycle save hook — see `Host::new` for the safety argument.
    static HOST_PTR: Cell<*mut Host> = const { Cell::new(std::ptr::null_mut()) };
}

/// eframe adapter around the core app.
struct Host {
    app: App,
}

impl Host {
    fn new(cc: &eframe::CreationContext<'_>, data_dir: PathBuf) -> Self {
        #[allow(unused_mut)]
        let mut app = App::new(data_dir, &cc.egui_ctx, platform_font());
        // iOS has no system back gesture for apps; the core supplies one.
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
                let host = HOST_PTR.with(Cell::get);
                if !host.is_null() {
                    // SAFETY: this closure runs on the UIKit main thread,
                    // from a lifecycle notification dispatched while
                    // eframe's event loop is parked inside
                    // UIApplicationMain *between* `App::ui` calls — no
                    // `&mut Host` borrow is outstanding, and nothing else
                    // can touch the Host concurrently (the whole app is
                    // single-threaded through the main thread; this is the
                    // same main-thread TLS discipline ios.rs documents).
                    // The pointer was written by the most recent `ui()`
                    // call on this thread, and the Host stays alive until
                    // `run_native` returns — which on iOS it never does.
                    // If eframe ever runs `ui` off the main thread or
                    // reenters it from the run loop, this breaks.
                    unsafe { &mut *host }.app.save_current();
                }
                // Belt and braces: make sure a frame runs with the saved
                // state if the app returns to the foreground.
                ctx.request_repaint();
            });
        }
        log::info!("Rust Notes ready, font: {}", app.font_source());
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

impl eframe::App for Host {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        // The platform hooks hand back reduce-motion / UTC-offset values
        // only when they changed; applying them here (rather than inside
        // the platform modules) is what keeps them on the core `App`.
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
        #[cfg(target_os = "ios")]
        HOST_PTR.with(|p| p.set(self as *mut Host));
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
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        #[cfg(not(target_os = "ios"))]
        renderer: eframe::Renderer::Glow,
        #[cfg(target_os = "ios")]
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size([420.0, 860.0]),
        #[cfg(target_os = "ios")]
        wgpu_options: ios::wgpu_options(),
        ..Default::default()
    }
}

/// Start the app. Blocks until it exits.
pub fn run(options: eframe::NativeOptions, data_dir: PathBuf) -> eframe::Result {
    // Earliest iOS-specific init: stderr is unreadable on device, so panics
    // must go through NSLog.
    #[cfg(target_os = "ios")]
    ios::install_panic_hook();
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(Host::new(cc, data_dir)))),
    )
}
