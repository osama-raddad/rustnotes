//! Rust Notes host: runs [`rustnotes_core::App`] with eframe.
//!
//! eframe and winit own the window, the event loop, rendering, density and
//! the soft keyboard. This crate supplies only what they do not:
//!
//! - Android (`android.rs`): `android_main`, window insets, night mode and
//!   font scale through JNI, and leaving the app on back at the root.
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
            leave_app();
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
    }
}

/// Back at the root: hand control back to the OS, as native apps do.
fn leave_app() {
    #[cfg(target_os = "android")]
    android::finish_activity();
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
