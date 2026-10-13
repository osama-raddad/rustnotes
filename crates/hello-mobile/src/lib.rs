//! Hello Mobile — the minimal second consumer of the `rustnotes` host.
//!
//! This crate answers the extraction plan's Q2 with code: the host's
//! platform glue (Android bootstrap, iOS observers, desktop entry
//! helpers) is reachable from another crate, generically. There is no
//! `rustnotes-core` dependency anywhere here — nothing of Rust Notes
//! leaks across the seam — and everything app-shaped sits behind
//! `rustnotes::App`, implemented below for [`HelloApp`].
//!
//! It is a proof, not a product: every trait method does the least
//! honest thing, and its doc comment says what that means for the seam
//! contract. The UI shows the two identity facts the host derives from
//! the trait (own app name, own data dir), so a screenshot is the
//! user-observable proof that the generalization is real.

use std::path::PathBuf;

/// The app. Stores only what the UI shows: where the host resolved its
/// data directory to. The seam's preference hooks are no-ops because a
/// stateless proof has no preferences to honour.
pub struct HelloApp {
    data_dir: PathBuf,
}

impl rustnotes::App for HelloApp {
    const APP_NAME: &'static str = "Hello Mobile";
    const DESKTOP_DATA_DIR: &'static str = ".hello-mobile";
    const ANDROID_DATA_DIR_FALLBACK: &'static str = "/data/data/com.example.hellomobile/files";
    const ANDROID_LOG_TAG: &'static str = "hellomobile";

    /// Nothing to open — no database, no fonts to install (a rejected
    /// `platform_font` means egui's bundled defaults stay, which
    /// `font_source` reports). The context would only be needed to
    /// install things into it.
    fn new(
        data_dir: PathBuf,
        _ctx: &egui::Context,
        _platform_font: Option<(Vec<u8>, String)>,
    ) -> Self {
        log::info!("starting in {}", data_dir.display());
        HelloApp { data_dir }
    }

    /// The host enables this on iOS; a proof app has no screen stack to
    /// pop, so the gesture would have nothing to drive.
    fn set_edge_swipe_back(&mut self, _enabled: bool) {}

    fn font_source(&self) -> &str {
        "egui default"
    }

    /// Nothing animates, so there is no motion to reduce.
    fn set_reduce_motion(&mut self, _reduce: bool) {}

    /// No dates are rendered, so the zone offset has nowhere to go.
    fn set_utc_offset_secs(&mut self, _offset_secs: i32) {}

    /// No edge swipe is claimed, so every event passes through to egui
    /// untouched.
    fn filter_input(&mut self, _raw: &mut egui::RawInput) {}

    /// The host still fires every save moment (focus loss, backgrounding,
    /// exit); a stateless app simply has no buffer to flush.
    fn save_current(&mut self) {}

    /// Nothing to go back to: the host may leave the app (on Android,
    /// back-at-root exits the eframe loop as native apps do).
    fn on_back(&mut self) -> bool {
        false
    }

    /// The proof itself: the identity facts the host derived from this
    /// impl, rendered. The `root` covers the whole window (edge-to-edge
    /// on mobile), exactly as the Rust Notes app gets it.
    fn ui(&mut self, root: &mut egui::Ui) {
        let top = root.available_height() / 3.0;
        root.vertical_centered(|ui| {
            ui.add_space(top);
            ui.heading("Hello from the seam");
            ui.add_space(12.0);
            ui.label(Self::APP_NAME);
            ui.label(self.data_dir.display().to_string());
        });
    }
}

/// The proof's own Android entry — the only `android_main` linked into
/// its cdylib, because the `rustnotes` dependency is built with
/// `default-features = false` (its own entry is compiled out; two
/// `#[no_mangle]` symbols of the same name would collide at link time).
/// Everything past this line is host framework code.
#[cfg(target_os = "android")]
pub mod android {
    use android_activity::AndroidApp;

    #[unsafe(no_mangle)]
    fn android_main(app: AndroidApp) {
        rustnotes::android::start::<crate::HelloApp>(app);
    }
}
