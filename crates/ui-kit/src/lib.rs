//! Vynatix UI kit — the Vynatix design system as pure egui.
//!
//! The crate is token-driven: [`tokens`] carries the design tokens
//! generated from `.afk/design-system/tokens.json` (the generated modules
//! are committed; builds never run the generator), and [`theme`] resolves
//! them into the six themes and their egui [`egui::Visuals`]
//! (Forest/Teal/Monochrome, light and dark each).
//!
//! On top of the tokens sit the foundations: [`states`] (the interaction
//! overlays, the focus-ring geometry, the kit flags), [`icon`] with the
//! generated [`icons`] catalogue (Material Symbols subsets, the
//! one-glyph widget), [`logo`] (the mark and wordmark as build-time
//! triangulated role-coloured fills), and [`components`] (the widget
//! wrappers built on them, FocusRing first).
//!
//! Pure egui like `rustnotes-core`: no window, GL or platform glue. The
//! gallery (`examples/gallery.rs`) hosts itself on eframe for desktop
//! demo purposes only.

pub mod components;
pub mod icon;
pub mod icons;
pub mod logo;
pub mod states;
pub mod theme;
pub mod tokens;
pub mod typography;
