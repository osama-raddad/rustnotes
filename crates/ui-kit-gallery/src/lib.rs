//! The Vynatix Gallery app: the ui-kit's demo surface hosted on the
//! `rustnotes` host, the way `hello-mobile` is hosted — the host supplies
//! the platform glue the kit deliberately does not have (iOS safe-area and
//! keyboard observers, Android window insets, night mode and font scale,
//! the per-platform renderers), and this crate supplies only the app:
//! [`rustnotes::App`] implemented for [`GalleryApp`], wrapping the
//! gallery UI in `gallery.rs`.
//!
//! Nothing of Rust Notes leaks across the seam (no `rustnotes-core`
//! dependency here), and nothing of the host leaks into the kit: the kit
//! stays pure egui, and this crate is the only place the two meet.
//!
//! Input reaches egui untouched. egui-winit turns every finger into an
//! `Event::Touch` (plus a parallel pointer stream for the first finger,
//! which is what taps and focus ride on), and the touches are what tell
//! egui the screen is a touch screen: from then on its own
//! drag-to-scroll moves the page and the sideways specimen rows under
//! the finger, with the kinetic fling on release. The gallery's labels
//! are not selectable (a gallery is not a document), so a drag that
//! starts on text scrolls instead of selecting it.

use std::path::PathBuf;

use egui::Context;

mod gallery;
#[cfg(test)]
mod input_tests;

pub use gallery::Gallery;

/// The gallery as a `rustnotes::App`: identity consts for the platform
/// glue, the kit's fonts and default theme installed at creation, the
/// host's OS-level preferences mapped onto the kit's flags, and the
/// gallery's chrome as the root UI.
pub struct GalleryApp {
    gallery: Gallery,
}

impl rustnotes::App for GalleryApp {
    const APP_NAME: &'static str = "Vynatix Gallery";
    const DESKTOP_DATA_DIR: &'static str = ".vynatix-gallery";
    const ANDROID_DATA_DIR_FALLBACK: &'static str =
        "/data/data/com.osamaraddad.vynatixgallery/files";
    const ANDROID_LOG_TAG: &'static str = "vynatixgallery";

    /// Install every theme's faces (the Brand tab's theme cards set their
    /// own themes' display faces; the installs are additive by design),
    /// the icon subsets, and apply the default theme with the gallery's
    /// own style on top (see `gallery::apply_theme`).
    fn new(_data_dir: PathBuf, ctx: &Context, _platform_font: Option<(Vec<u8>, String)>) -> Self {
        for id in ui_kit::theme::ThemeId::ALL {
            ui_kit::typography::install(*id, ctx);
        }
        ui_kit::icon::install(ctx);
        gallery::apply_theme(ui_kit::theme::ThemeId::Light, ctx);
        Self {
            gallery: Gallery::new(),
        }
    }

    /// The gallery's panels are tabs, not a stack; there is nothing to pop.
    fn set_edge_swipe_back(&mut self, _enabled: bool) {}

    /// The kit bundles every face it sets; the host's system font is not
    /// consulted.
    fn font_source(&self) -> &str {
        "ui-kit bundled faces"
    }

    /// The OS preference drives the kit's own reduced-motion flag, so the
    /// toggle in the theme menu and the OS setting agree for the session.
    fn set_reduce_motion(&mut self, reduce: bool) {
        self.gallery.flags.reduced_motion = reduce;
    }

    /// No dates are rendered from the OS clock.
    fn set_utc_offset_secs(&mut self, _offset_secs: i32) {}

    /// Nothing is filtered: the touches reach egui as egui-winit
    /// delivers them, and egui's own drag-to-scroll does the scrolling.
    fn filter_input(&mut self, _raw: &mut egui::RawInput) {}

    /// Nothing persists; every save moment is a no-op.
    fn save_current(&mut self) {}

    /// Tabs are siblings, not a stack: back always leaves the app.
    fn on_back(&mut self) -> bool {
        false
    }

    /// The root UI for one frame. The viewport's safe-area margins are
    /// painted with the live theme's canvas first (egui lays panels out
    /// inside the insets and paints nothing behind them), then the
    /// gallery's chrome: the app bar panel, the tabbed content panel and
    /// the floating snackbar.
    fn ui(&mut self, root: &mut egui::Ui) {
        let viewport = root.ctx().input(|i| i.viewport_rect());
        let canvas = self.gallery.theme.colors().background;
        root.painter().rect_filled(viewport, 0.0, canvas);
        self.gallery.chrome(root);
    }
}

/// The gallery's own Android entry — the only `android_main` linked into
/// its cdylib, because the `rustnotes` dependency is built with
/// `default-features = false` (its own entry is compiled out; two
/// `#[no_mangle]` symbols of the same name would collide at link time).
/// Everything past this line is host framework code.
#[cfg(target_os = "android")]
pub mod android {
    use android_activity::AndroidApp;

    #[unsafe(no_mangle)]
    fn android_main(app: AndroidApp) {
        rustnotes::android::start::<crate::GalleryApp>(app);
    }
}
