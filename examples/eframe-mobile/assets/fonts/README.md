# Fonts

The two `.ttf`/`.otf` files here are empty placeholders so that
`include_bytes!` in `src/fonts.rs` compiles. Replace them with the real
fonts from <https://fonts.google.com/noto> (SIL Open Font License):

| File | Covers | Approx. size |
|---|---|---|
| `NotoSansArabic-Regular.ttf` | Arabic (glyphs only; egui 0.36 has no right-to-left reordering) | 200 KB |
| `NotoSansSC-Regular.otf` | Simplified Chinese (CJK) | 8 MB |

Latin, Greek and Cyrillic come from eframe's default fonts, so no Latin
Noto file is needed.

Until they are replaced the app logs a warning at startup and non-Latin
text renders as missing-glyph boxes.
