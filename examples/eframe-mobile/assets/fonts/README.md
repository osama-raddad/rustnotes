# Fonts

The three `.ttf`/`.otf` files here are empty placeholders so that
`include_bytes!` in `src/fonts.rs` compiles. Replace them with the real
fonts from <https://fonts.google.com/noto> (SIL Open Font License):

| File | Covers | Approx. size |
|---|---|---|
| `NotoSans-Regular.ttf` | Latin, Greek, Cyrillic | 500 KB |
| `NotoSansArabic-Regular.ttf` | Arabic | 200 KB |
| `NotoSansSC-Regular.otf` | Simplified Chinese (CJK) | 8 MB |

Until they are replaced the app logs a warning at startup and non-Latin
text renders as missing-glyph boxes.
