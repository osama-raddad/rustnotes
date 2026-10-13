//! The Vynatix gallery: a theme switcher, the type specimen, a palette
//! swatch grid, the interaction states and the foundation specimens
//! (spacing, radii, elevation). Run with `cargo run -p ui-kit --example
//! gallery`. Each issue grows a section here.

use eframe::egui::{self, Align2, Color32, CornerRadius, Rect, Sense, Vec2};

use ui_kit::components::FocusRing;
use ui_kit::states::{self, UiKitFlags};
use ui_kit::theme::{egui_shadow, ThemeId};
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
            Ok(Box::new(Gallery {
                theme,
                flags: UiKitFlags::default(),
            }))
        }),
    )
}

struct Gallery {
    theme: ThemeId,
    /// The kit flags this host passes down, standing in for the web
    /// system's reduced-motion and reduced-transparency media queries.
    flags: UiKitFlags,
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

/// One state specimen: a button-shaped rect at `scale`, its caption,
/// and the focus ring when the state is focus.
fn state_tile(
    ui: &mut egui::Ui,
    theme: ThemeId,
    caption: &str,
    fill: Color32,
    scale: f32,
    text: Color32,
    focused: bool,
) {
    let colors = theme.colors();
    let size = Vec2::new(spacing::SPACE_24, spacing::SPACE_10);
    ui.vertical(|ui| {
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let body = Rect::from_center_size(rect.center(), size * scale);
        let radius = CornerRadius::same(radii::RADIUS_MD as u8);
        ui.painter().rect_filled(body, radius, fill);
        if focused {
            states::focus_ring_at(ui.painter(), rect, radius, theme);
        }
        ui.painter().text(
            body.center(),
            Align2::CENTER_CENTER,
            "Start a project",
            TypeStyle::Body.font_id(theme),
            text,
        );
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            caption,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
        )));
    });
}

/// The interaction states: hover, press, focus and disabled as
/// specimens beside a control that really takes focus via Tab.
fn states_section(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        "States · Hover, press, focus",
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    let rest = colors.primary;
    let hover = states::composite_over(states::hover_overlay(colors.on_primary), rest);
    let pressed = states::composite_over(states::pressed_overlay(colors.on_primary), rest);
    let disabled = states::disabled(colors.on_surface, colors.surface_container);

    ui.horizontal(|ui| {
        state_tile(ui, theme, "Rest", rest, 1.0, colors.on_primary, false);
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            "Hover · 8% on-colour overlay",
            hover,
            1.0,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            "Pressed · 12% overlay · 98.5% scale · 100ms",
            pressed,
            states::PRESS_SCALE,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            "Focus · 2px ring · 2px offset · 4px halo · 120ms",
            rest,
            1.0,
            colors.on_primary,
            true,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            "Disabled · content 38% · container 12%",
            disabled.container,
            1.0,
            disabled.content,
            false,
        );
    });

    ui.add_space(spacing::SPACE_2);
    FocusRing::new().wrap(ui, theme, flags, |ui| {
        ui.add(egui::Button::new("Tab to me"))
    });
    ui.add(egui::Label::new(specimen_job(
        "Press Tab until the ring lands here. Focus is carried by the ring, not a colour change.",
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
    )));
}

/// The foundations as specimens: the spacing scale, all eight radii and
/// the active theme's five elevation levels.
fn foundations_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        "Foundations · Spacing, radii, elevation",
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    ui.label("Spacing · the approved scale · 4px base");
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        for (name, value) in spacing::ALL {
            ui.vertical(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(spacing::SPACE_24, spacing::SPACE_2),
                    Sense::hover(),
                );
                if *value > 0.0 {
                    let bar = Rect::from_min_size(rect.min, Vec2::new(*value, rect.height()));
                    ui.painter()
                        .rect_filled(bar, CornerRadius::same(0), colors.primary);
                }
                ui.add(egui::Label::new(specimen_job(
                    &format!("{name} · {}", trim_float(*value)),
                    TypeStyle::Caption,
                    theme,
                    colors.text_muted(),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });

    ui.add_space(spacing::SPACE_4);
    ui.label("Radii · all eight");
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        for (name, value) in radii::ALL {
            ui.vertical(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::splat(spacing::SPACE_16), Sense::hover());
                let radius = CornerRadius::same(value.min(255.0) as u8);
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    egui::Stroke::new(1.0, colors.outline_variant),
                    egui::StrokeKind::Outside,
                );
                ui.painter()
                    .rect_filled(rect.shrink(1.0), radius, colors.surface_container_high);
                ui.add(egui::Label::new(specimen_job(
                    &format!("{} · {}", sentence_case(name), trim_float(*value)),
                    TypeStyle::Caption,
                    theme,
                    colors.text_muted(),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });

    ui.add_space(spacing::SPACE_4);
    ui.label(format!("Elevation · L1 to L5 · {}", theme.name()));
    ui.add_space(spacing::SPACE_2);
    let shadows = theme.shadows();
    let levels = [
        ("L1", shadows.l1),
        ("L2", shadows.l2),
        ("L3", shadows.l3),
        ("L4", shadows.l4),
        ("L5", shadows.l5),
    ];
    ui.horizontal(|ui| {
        for (level, layers) in levels {
            ui.vertical(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(spacing::SPACE_24, spacing::SPACE_24),
                    Sense::hover(),
                );
                let card = Rect::from_center_size(
                    rect.center(),
                    Vec2::new(spacing::SPACE_16, spacing::SPACE_10),
                );
                let radius = CornerRadius::same(radii::RADIUS_LG as u8);
                ui.painter().add(egui_shadow(layers).as_shape(card, radius));
                ui.painter()
                    .rect_filled(card, radius, colors.surface_container_low);
                let caption = layers
                    .first()
                    .map(|layer| {
                        format!(
                            "{} · {} {} {} · {:.0}%",
                            level,
                            trim_float(layer.x),
                            trim_float(layer.y),
                            trim_float(layer.blur),
                            layer.color.a() as f32 * 100.0 / 255.0,
                        )
                    })
                    .unwrap_or_else(|| level.to_owned());
                ui.add(egui::Label::new(specimen_job(
                    &caption,
                    TypeStyle::Caption,
                    theme,
                    colors.text_muted(),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });
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
            ui.add_space(spacing::SPACE_1);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.flags.reduced_motion, "Reduced motion");
                ui.checkbox(&mut self.flags.reduced_transparency, "Reduced transparency");
            });

            ui.add_space(spacing::SPACE_6);
            egui::ScrollArea::vertical().show(ui, |ui| {
                type_section(ui, self.theme);
                states_section(ui, self.theme, &self.flags);
                foundations_section(ui, self.theme);
                ui.add_space(spacing::SPACE_6);

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
