//! Font setup.
//!
//! eframe's bundled fonts cover Latin, Greek and Cyrillic only. Any string
//! that `t!()` returns in Arabic or Chinese would render as tofu boxes
//! without a fallback. egui resolves glyphs by walking the family list in
//! order, so appending Noto fonts *after* the default ones keeps the default
//! look for Latin and falls back per-glyph for everything else.
//!
//! Fonts are embedded at compile time: mobile bundles have no reliable
//! runtime font path (Android ships NotoSansCJK as a .ttc with no stable
//! name across vendors, iOS hides PingFang behind CoreText), and embedding
//! keeps rendering identical on every device. Download from
//! https://fonts.google.com/noto and drop into assets/fonts/.
//! Sizes: Noto Sans ~500 KB, Noto Sans Arabic ~200 KB, Noto Sans SC ~8 MB.
//! Replace the CJK font with a subset if binary size matters.

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// (family name registered with egui, font bytes)
const FALLBACK_FONTS: &[(&str, &[u8])] = &[
    ("NotoSans", include_bytes!("../assets/fonts/NotoSans-Regular.ttf")),
    (
        "NotoSansArabic",
        include_bytes!("../assets/fonts/NotoSansArabic-Regular.ttf"),
    ),
    ("NotoSansSC", include_bytes!("../assets/fonts/NotoSansSC-Regular.otf")),
];

/// Install the default fonts plus the Noto fallbacks. Call once at startup;
/// `set_fonts` rebuilds the glyph atlas, so never call it per frame.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    for (name, bytes) in FALLBACK_FONTS {
        if bytes.is_empty() {
            // Placeholder file (CI, fresh checkout): skip rather than panic.
            log::warn!("font {name} is empty; non-Latin text will not render");
            continue;
        }
        fonts
            .font_data
            .insert((*name).to_owned(), Arc::new(FontData::from_static(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push((*name).to_owned());
        }
    }

    ctx.set_fonts(fonts);
}
