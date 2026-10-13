//! The Vynatix gallery: a theme switcher, the type specimen and a
//! palette swatch grid over the kit's tokens. Run with `cargo run -p
//! ui-kit --example gallery`. Each issue grows a section here.

use eframe::egui::{self, Color32, CornerRadius, Sense, Vec2};

use ui_kit::theme::ThemeId;
use ui_kit::tokens::{radii, spacing, ThemeColors};
use ui_kit::typography::{install, specimen_job, TrackedLabel, TypeStyle};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vynatix UI kit")
            .with_inner_size([1000.0, 780.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Vynatix UI kit",
        options,
        Box::new(|cc| {
            let theme = ThemeId::Light;
            install(theme, &cc.egui_ctx);
            theme.apply(&cc.egui_ctx);
            Ok(Box::new(Gallery { theme }))
        }),
    )
}

struct Gallery {
    theme: ThemeId,
}

/// A role name for display: strip the `md-sys-color-` prefix and set the
/// words in sentence case ("surface-container-low" becomes "Surface
/// container low").
fn sentence_case(role: &str) -> String {
    let stripped = role.strip_prefix("md-sys-color-").unwrap_or(role);
    let mut out = String::with_capacity(stripped.len());
    for (i, word) in stripped.split('-').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
        }
        out.push_str(chars.as_str());
    }
    out
}

/// The type-specimen samples, the design system's own copy from
/// `tokens.json` `type.groups`, in specimen order.
const SPECIMEN: &[(TypeStyle, &str)] = &[
    (TypeStyle::Display, "Quiet is the new loud."),
    (
        TypeStyle::Headline1,
        "We rebuilt Volvo's truck radio in sixteen weeks.",
    ),
    (
        TypeStyle::Headline2,
        "A premium consultancy moves the way a turntable moves: deliberate, damped, confident.",
    ),
    (
        TypeStyle::Headline3,
        "Why does most consulting writing sound the same?",
    ),
    (
        TypeStyle::Headline4,
        "Engineering judgement, sold by the senior person doing the work.",
    ),
    (TypeStyle::Lede, "Vynatix is a Nordic IT consultancy in Gothenburg."),
    (
        TypeStyle::Body,
        "The discipline of the system is the luxury signal: not ornament, not saturation, not motion.",
    ),
    (
        TypeStyle::Editorial,
        "Premium isn't a price point. It's a discipline. The hundred small decisions a competitor would have skipped, made deliberately and visibly, day after day.",
    ),
    (TypeStyle::Eyebrow, "Field notes · Gothenburg · April 2026"),
    (
        TypeStyle::Mono,
        "--md-sys-color-primary: #1E3A2F;",
    ),
    (
        TypeStyle::Caption,
        "Photography by Jens Lindqvist. Volvo Trucks workshop, Lundby, Gothenburg. April 2026.",
    ),
];

/// The specimen colour each style sets, following the stylesheet's
/// helpers: headlines and pull-quotes in text-display, ledes and captions
/// in text-muted, body in text-body, the eyebrow in text-accent.
fn specimen_color(style: TypeStyle, colors: &ThemeColors) -> Color32 {
    match style {
        TypeStyle::Display
        | TypeStyle::Headline1
        | TypeStyle::Headline2
        | TypeStyle::Headline3
        | TypeStyle::Headline4
        | TypeStyle::Editorial => colors.text_display(),
        TypeStyle::Lede | TypeStyle::Caption => colors.text_muted(),
        TypeStyle::Body | TypeStyle::Mono => colors.text_body(),
        TypeStyle::Eyebrow => colors.text_accent,
    }
}

/// A tracking value as the specimen sets it: "+0.24em", "0em", or the
/// typographic minus for the headlines' negative tracks.
fn format_tracking(tracking_em: f32) -> String {
    if tracking_em == 0.0 {
        "0em".to_owned()
    } else if tracking_em < 0.0 {
        format!("\u{2212}{}em", -tracking_em)
    } else {
        format!("+{tracking_em}em")
    }
}

/// The type specimen: every style in the active theme's faces, each with
/// its meta line (size over leading, tracking, family).
fn type_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        "Materials · Type · 2026",
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    for (style, sample) in SPECIMEN {
        let color = specimen_color(*style, &colors);
        if *style == TypeStyle::Eyebrow {
            ui.add(TrackedLabel::new(
                sample,
                style.font_id(theme),
                style.tracking(),
                color,
            ));
        } else {
            ui.add(egui::Label::new(specimen_job(sample, *style, theme, color)));
        }
        ui.add_space(spacing::SPACE_1);
        let meta = format!(
            "{} · {}/{} · {} · {}",
            sentence_case(style.name().trim_start_matches("type-")),
            trim_float(style.size()),
            trim_float(style.leading()),
            format_tracking(style.tracking()),
            style.family_name(theme),
        );
        ui.add(egui::Label::new(specimen_job(
            &meta,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
        )));
        ui.add_space(spacing::SPACE_4);
    }
}

/// A float without its trailing zeros: 96.0 renders "96", 1.08 "1.08".
fn trim_float(value: f32) -> String {
    let formatted = format!("{value}");
    formatted
        .strip_suffix(".0")
        .unwrap_or(&formatted)
        .to_owned()
}

impl eframe::App for Gallery {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        egui::CentralPanel::default().show(root, |ui| {
            ui.add(egui::Label::new(specimen_job(
                "Vynatix UI kit",
                TypeStyle::Headline2,
                self.theme,
                self.theme.colors().text_display(),
            )));
            ui.add_space(spacing::SPACE_4);

            ui.label("Themes");
            ui.add_space(spacing::SPACE_2);
            ui.horizontal(|ui| {
                for id in ThemeId::ALL {
                    let mut picked = self.theme;
                    ui.selectable_value(&mut picked, *id, id.name());
                    if picked != self.theme {
                        self.theme = picked;
                        install(picked, &ctx);
                        picked.apply(&ctx);
                    }
                }
            });

            ui.add_space(spacing::SPACE_6);
            egui::ScrollArea::vertical().show(ui, |ui| {
                type_section(ui, self.theme);

                ui.add_space(spacing::SPACE_4);
                ui.label(format!("Colour roles · {}", self.theme.name()));
                ui.add_space(spacing::SPACE_2);

                let colors = self.theme.colors();
                egui::Grid::new("colour-roles")
                    .num_columns(2)
                    .spacing([spacing::SPACE_4, spacing::SPACE_2])
                    .show(ui, |ui| {
                        for role in ThemeColors::ROLES {
                            let color = colors
                                .role(role)
                                .expect("ROLES names all resolve to fields");
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(radii::RADIUS_XS as u8),
                                color,
                            );
                            ui.label(sentence_case(role));
                            ui.end_row();
                        }
                    });
            });
        });
    }
}
