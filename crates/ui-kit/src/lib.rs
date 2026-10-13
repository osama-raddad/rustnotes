//! Vynatix UI kit — the Vynatix design system as pure egui.
//!
//! The crate is token-driven: [`tokens`] carries the design tokens
//! generated from `.afk/design-system/tokens.json` (the generated modules
//! are committed; builds never run the generator), and [`theme`] resolves
//! them into the six themes and their egui [`egui::Visuals`]
//! (Forest/Teal/Monochrome, light and dark each).
//!
//! Pure egui like `rustnotes-core`: no window, GL or platform glue. The
//! gallery (`examples/gallery.rs`) hosts itself on eframe for desktop
//! demo purposes only.

pub mod theme;
pub mod tokens;
