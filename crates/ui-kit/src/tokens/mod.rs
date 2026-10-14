//! Vynatix design tokens.
//!
//! Every module below except `tests` is GENERATED from
//! `.afk/design-system/tokens.json` by `tools/generate_tokens.py`
//! (python3, stdlib only). The generated files are committed: builds
//! never run the generator. After the token source changes, regenerate
//! from the crate directory (`python3 tools/generate_tokens.py`), run
//! `cargo fmt`, and commit the result.

pub mod breakpoints;
pub mod colors;
pub mod containers;
pub mod icons;
pub mod leading;
pub mod motion;
pub mod radii;
pub mod shadows;
pub mod spacing;
pub mod tracking;
pub mod type_scale;

pub use colors::ThemeColors;

#[cfg(test)]
mod tests;
