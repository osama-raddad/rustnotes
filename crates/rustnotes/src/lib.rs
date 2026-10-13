//! Rust Notes host: runs [`rustnotes_core::App`] with eframe.
//!
//! eframe and winit own the window, the event loop, rendering, density and
//! the soft keyboard. This crate supplies only what they do not:
//!
//! - Android (`android.rs`): `android_main`, window insets, night mode and
//!   font scale through JNI, and exiting the eframe loop on back at the
//!   root (the activity is finished from `on_exit`, after the loop ends).
//! - iOS (`ios.rs`): the system font through CoreText, safe-area and
//!   keyboard insets, appearance and Dynamic Type, wgpu limits that the
//!   iOS Metal devices meet.
//! - Desktop: nothing; `cargo run -p rustnotes` is the fast development loop.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use rustnotes_core::App;
use std::path::PathBuf;

pub const APP_NAME: &str = "Rust Notes";

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
        app.set_edge_swipe_back(true);
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
        #[cfg(target_os = "android")]
        android::raw_input_hook(ctx, raw);
        #[cfg(target_os = "ios")]
        ios::raw_input_hook(ctx, raw);
        let _ = ctx;
        self.app.filter_input(raw);
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        // Losing focus is the last reliable moment before Android or iOS may
        // suspend or kill the process, so save the open note then.
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
        // eframe calls this from its teardown after ViewportCommand::Close,
        // with the loop's state already gone. Only now may Android finish
        // the activity — see `leave_app` for why doing it earlier deadlocks.
        #[cfg(target_os = "android")]
        android::finish_activity();
    }

    #[cfg(target_os = "ios")]
    fn on_exit(&mut self) {
        self.app.save_current();
    }
}

/// Back at the root: hand control back to the OS, as native apps do.
///
/// Android: close the viewport, which runs eframe's exit path (ending with
/// [`Host::on_exit`], where the activity is finished) and then exits the
/// winit loop so `android_main` returns. Finishing the activity directly
/// from here instead would deadlock: NativeActivity's `onDestroy` blocks
/// the Java main thread until `android_main` returns, and the eframe/winit
/// loop never exits on its own (winit ignores Android's destroy event), so
/// the manifest's singleTask would serve the next launch from a process
/// stuck in that wait. iOS apps may not exit programmatically, and desktop
/// has no back-at-root contract, so only Android acts.
fn leave_app(ctx: &egui::Context) {
    #[cfg(target_os = "android")]
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    #[cfg(not(target_os = "android"))]
    let _ = ctx;
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
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(Host::new(cc, data_dir)))),
    )
}
