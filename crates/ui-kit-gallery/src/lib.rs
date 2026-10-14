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

use std::cell::RefCell;
use std::path::PathBuf;

use egui::Context;

mod gallery;
#[cfg(test)]
mod input_tests;

pub use gallery::Gallery;

#[cfg(test)]
/// A `GalleryApp` for the input tests: a gallery state plus a live
/// context, the pieces `filter_input` needs.
pub(crate) fn test_app(gallery: Gallery, ctx: egui::Context) -> GalleryApp {
    GalleryApp {
        gallery,
        ctx: Some(ctx),
    }
}

thread_local! {
    /// The last touch position while a finger is down, the scroll
    /// shim's drag anchor.
    static SCROLL_TOUCH: RefCell<Option<egui::Pos2>> = const { RefCell::new(None) };
}

/// The gallery as a `rustnotes::App`: identity consts for the platform
/// glue, the kit's fonts and default theme installed at creation, the
/// host's OS-level preferences mapped onto the kit's flags, and the
/// gallery's chrome as the root UI.
pub struct GalleryApp {
    gallery: Gallery,
    /// The live egui context, captured each frame so the touch-scroll
    /// shim in `filter_input` (which has no context argument) can
    /// reach the scroll state between frames.
    ctx: Option<egui::Context>,
}

impl rustnotes::App for GalleryApp {
    const APP_NAME: &'static str = "Vynatix Gallery";
    const DESKTOP_DATA_DIR: &'static str = ".vynatix-gallery";
    const ANDROID_DATA_DIR_FALLBACK: &'static str =
        "/data/data/com.osamaraddad.vynatixgallery/files";
    const ANDROID_LOG_TAG: &'static str = "vynatixgallery";

    /// Install every theme's faces (the Brand tab's theme cards set their
    /// own themes' display faces; the installs are additive by design),
    /// the icon subsets, and apply the default theme.
    fn new(_data_dir: PathBuf, ctx: &Context, _platform_font: Option<(Vec<u8>, String)>) -> Self {
        for id in ui_kit::theme::ThemeId::ALL {
            ui_kit::typography::install(*id, ctx);
        }
        ui_kit::icon::install(ctx);
        ui_kit::theme::ThemeId::Light.apply(ctx);
        Self {
            gallery: Gallery::new(),
            ctx: None,
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

    /// Touch filtering: the scroll shim (see `filter_input_shim`).
    fn filter_input(&mut self, raw: &mut egui::RawInput) {
        self.filter_input_shim(raw);
    }

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
        self.ctx = Some(root.ctx().clone());
        let viewport = root.ctx().input(|i| i.viewport_rect());
        let canvas = self.gallery.theme.colors().background;
        root.painter().rect_filled(viewport, 0.0, canvas);
        self.gallery.chrome(root);
        // TEMP-DEBUG
        std::fs::write(
            std::env::temp_dir().join("vynatix-offset-debug.log"),
            format!(
                "offset {:?}
",
                self.gallery.last_scroll_offset
            ),
        )
        .ok();
    }
}

impl GalleryApp {
    /// Touch-scroll shim: finger drags drive the page's scroll offset
    /// directly. egui's wheel path is pointer-hover-gated (dead during
    /// a touch drag), and its own touch-to-scroll fights these writes
    /// to a standstill — the exact "scrolling does not work" symptom —
    /// so on iOS (and in the tests) the touches are tracked and then
    /// swallowed; egui-winit still delivers its parallel pointer
    /// stream, so taps, drags on widgets and focus keep working. On
    /// Android the touches pass through untouched: its injected input
    /// reaches egui as one stream, and egui's native touch scroll
    /// works there.
    pub(crate) fn filter_input_shim(&mut self, raw: &mut egui::RawInput) {
        let mut drag: Option<egui::Vec2> = None;
        let mut rest: Vec<egui::Event> = Vec::new();
        // iOS (and the tests) swallow the touches after tracking; every
        // other platform passes them through untouched.
        let keep_touches = !cfg!(any(target_os = "ios", test));
        for event in std::mem::take(&mut raw.events) {
            if let egui::Event::Touch { phase, pos, .. } = &event {
                match phase {
                    egui::TouchPhase::Start => {
                        SCROLL_TOUCH.with(|c| *c.borrow_mut() = Some(*pos));
                    }
                    egui::TouchPhase::Move => {
                        let last = SCROLL_TOUCH.with(|c| c.borrow_mut().replace(*pos));
                        if let Some(last) = last {
                            let delta = *pos - last;
                            drag = Some(drag.map_or(delta, |so_far| so_far + delta));
                        }
                    }
                    _ => {
                        SCROLL_TOUCH.with(|c| *c.borrow_mut() = None);
                        // The release frame keeps the explicit offset
                        // path alive once more so the final position
                        // lands and is read back.
                        self.gallery.touch_scrolling = true;
                    }
                }
                if keep_touches {
                    rest.push(event);
                }
            } else {
                rest.push(event);
            }
        }
        raw.events = rest;
        // Natural direction: the content follows the finger (a finger
        // moving up, negative dy, grows the offset and shows the later
        // content). The next frame's ScrollArea takes the offset via
        // `vertical_scroll_offset` (see `chrome`), and feeds the live
        // value back so the wheel path and clamping keep working.
        if let Some(delta) = drag {
            self.gallery.scroll_offset -= delta.y;
            self.gallery.touch_scrolling = true;
        }
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
