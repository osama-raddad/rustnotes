//! The Vynatix gallery: a theme switcher, the type specimen, a palette
//! swatch grid, the interaction states, the foundation specimens
//! (spacing, radii, elevation), the icons, the logo and the surface
//! components (cards, metrics, editorial, glass). Run with `cargo run
//! -p ui-kit --example gallery`. Each issue grows a section here.

use eframe::egui::{self, Align2, Color32, CornerRadius, Rect, Sense, Vec2};

use ui_kit::components::liquid_glass::{glass_recipe, paint_glass, GlassThickness, GlassTint};
use ui_kit::components::{Card, EditorialCard, FocusRing, LiquidGlass, MetricCard};
use ui_kit::icon::{icon, icon_active, install as install_icons, IconSize};
use ui_kit::icons::{Icon, CATEGORY_GROUPS};
use ui_kit::logo::Logo;
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
            install_icons(&cc.egui_ctx);
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

/// The icons: the five token sizes, then the whole catalogue grouped by
/// its categories, the fill-capable icons shown in their filled cut
/// (filled is the selected or toggled state; everything else rests
/// outlined).
fn icons_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        "Icons · Material Symbols Outlined · wght 300",
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    ui.horizontal(|ui| {
        for size in IconSize::ALL {
            ui.vertical(|ui| {
                icon(ui, Icon::Search, *size, colors.primary);
                ui.add(egui::Label::new(specimen_job(
                    &format!(
                        "{} · {}",
                        trim_float(size.px()),
                        size.name().trim_start_matches("icon-size-")
                    ),
                    TypeStyle::Caption,
                    theme,
                    colors.text_muted(),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });
    ui.add(egui::Label::new(specimen_job(
        "One library in every theme. Weight 300; grade -25 on the dark canvases so light glyphs don't bloom. The optical size sits at 24 in every subset. location_on has no glyph in font 0.47.5 and paints push_pin, the catalogue's pin.",
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
    )));

    for (category, icons) in CATEGORY_GROUPS {
        ui.add_space(spacing::SPACE_4);
        ui.label(format!("{category} · {}", icons.len()));
        ui.add_space(spacing::SPACE_1);
        ui.horizontal_wrapped(|ui| {
            for one in *icons {
                ui.vertical(|ui| {
                    if one.has_fill() {
                        icon_active(ui, *one, IconSize::Lg, colors.primary);
                    } else {
                        icon(ui, *one, IconSize::Lg, colors.primary);
                    }
                    let caption = if one.has_fill() {
                        format!("{} · fills", one.name())
                    } else {
                        one.name().to_owned()
                    };
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
}

/// One logo band: a filled ground carrying the logo in the placement's
/// ink, with its caption.
fn logo_band(
    ui: &mut egui::Ui,
    theme: ThemeId,
    caption: &str,
    ground: Option<Color32>,
    add: impl FnOnce(&mut egui::Ui),
) {
    let colors = theme.colors();
    let mut frame = egui::Frame::default()
        .inner_margin(spacing::SPACE_4)
        .corner_radius(radii::RADIUS_MD as u8);
    if let Some(ground) = ground {
        frame = frame.fill(ground);
    }
    frame.show(ui, |ui| ui.horizontal(|ui| add(ui)));
    ui.add(egui::Label::new(specimen_job(
        caption,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
    )));
}

/// The logo: the mark and wordmark on the canvas, reversed on the
/// primary and on an inverse band, the accent moment, and the size
/// floors in the caption.
fn logo_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        "Logo · The mark and the wordmark",
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    logo_band(ui, theme, "On the canvas · primary", None, |ui| {
        ui.add(Logo::mark().height(32.0));
        ui.add_space(spacing::SPACE_6);
        ui.add(Logo::wordmark());
    });
    ui.add_space(spacing::SPACE_2);
    logo_band(
        ui,
        theme,
        "On a primary fill · on-primary",
        Some(colors.primary),
        |ui| {
            ui.add(
                Logo::mark()
                    .height(32.0)
                    .ink(ui_kit::logo::LogoInk::OnPrimary),
            );
            ui.add_space(spacing::SPACE_6);
            ui.add(Logo::wordmark().ink(ui_kit::logo::LogoInk::OnPrimary));
        },
    );
    ui.add_space(spacing::SPACE_2);
    logo_band(
        ui,
        theme,
        "On an inverse band · inverse-on-surface",
        Some(colors.inverse_surface),
        |ui| {
            ui.add(
                Logo::mark()
                    .height(32.0)
                    .ink(ui_kit::logo::LogoInk::Inverse),
            );
            ui.add_space(spacing::SPACE_6);
            ui.add(Logo::wordmark().ink(ui_kit::logo::LogoInk::Inverse));
        },
    );
    ui.add_space(spacing::SPACE_2);
    logo_band(ui, theme, "The accent moment · text-accent", None, |ui| {
        ui.add(Logo::mark().height(32.0).ink(ui_kit::logo::LogoInk::Accent));
    });
    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        "Smallest on screen: the wordmark 96px wide, the mark 24px in product chrome and 96px in a brand placement. Clear space on every side: the height of the V.",
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
    )));
}

// ---- the surface components (issue 08) ------------------------------------

/// A card variant specimen: the class name in mono, one line of what
/// the variant is, per the Card preview.
fn card_specimen(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    class: &str,
    description: &str,
    card: Card,
) {
    let colors = theme.colors();
    card.ui(ui, theme, flags, |ui| {
        ui.set_width(spacing::SPACE_40 + spacing::SPACE_24);
        ui.add(egui::Label::new(specimen_job(
            class,
            TypeStyle::Mono,
            theme,
            colors.text_body(),
        )));
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            description,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
        )));
    });
}

/// The four card variants in the Card preview's two-column grid:
/// elevated (the default), outlined, filled and interactive, the last
/// one a real control to hover and tab to.
fn cards_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    ui.label("Cards · four variants · radius-lg · padding 24");
    ui.add_space(spacing::SPACE_2);
    egui::Grid::new("surfaces-cards")
        .num_columns(2)
        .spacing([spacing::SPACE_5, spacing::SPACE_5])
        .show(ui, |ui| {
            card_specimen(
                ui,
                theme,
                flags,
                ".card",
                "Elevated, the default: surface-container-low with shadow-l1.",
                Card::new(),
            );
            ui.end_row();
            card_specimen(
                ui,
                theme,
                flags,
                ".card--outlined",
                "Surface with a 1px outline-variant border, no shadow.",
                Card::new().outlined(),
            );
            card_specimen(
                ui,
                theme,
                flags,
                ".card--filled",
                "Surface-container, no shadow.",
                Card::new().filled().compact(),
            );
            ui.end_row();
            card_specimen(
                ui,
                theme,
                flags,
                ".card--interactive",
                "Lifts to surface-container and shadow-l2 on hover. Tab to it for the ring.",
                Card::new().interactive(),
            );
            ui.end_row();
            card_specimen(
                ui,
                theme,
                flags,
                ".card · large padding",
                "The same elevated card at space-8, 32px, for pull-quotes and folios.",
                Card::new().large(),
            );
            ui.end_row();
        });
}

/// The metric trio: one specific, sourced number per card, per the
/// MetricCard preview's own copy.
fn metrics_group(ui: &mut egui::Ui, theme: ThemeId) {
    ui.label("Metrics · one number each, sourced");
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        MetricCard::new(
            "Trucks running our code",
            "714k",
            "Volvo FH, FM, FMX. April 2026 fleet count.",
        )
        .ui(ui, theme);
        ui.add_space(spacing::SPACE_5);
        MetricCard::new(
            "Average engagement length",
            "19 mo",
            "Across 27 engagements since 2022.",
        )
        .ui(ui, theme);
        ui.add_space(spacing::SPACE_5);
        MetricCard::new(
            "Senior people, percent",
            "100",
            "No juniors on client work. Ever.",
        )
        .unit("%")
        .ui(ui, theme);
    });
}

/// The editorial row: a case study, a field note and an open role,
/// image areas as the placeholder tiles until photography exists.
fn editorial_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    ui.label("Editorial · case study, field note, open role");
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        EditorialCard::new(
            "Case study",
            "Rebuilding the truck radio in sixteen weeks.",
            "Three engineers, one designer, one product partner. Shipped to seven hundred thousand cabs by week thirty-one.",
        )
        .action("Read the case study")
        .ui(ui, theme, flags);
        ui.add_space(spacing::SPACE_5);
        EditorialCard::new(
            "Field notes",
            "Premium isn't a price point. It's a discipline.",
            "Why every Vynatix interface starts from a sixty-thirty-ten ratio, even when the brief asks for something louder.",
        )
        .action("Read the field note")
        .ui(ui, theme, flags);
        ui.add_space(spacing::SPACE_5);
        EditorialCard::new(
            "Open role",
            "Senior platform engineer, Gothenburg.",
            "Android Automotive, Kotlin, OTA pipelines. Permanent. Pay band published. No negotiation theatre.",
        )
        .action("Read the role")
        .ui(ui, theme, flags);
    });
}

/// One glass demo tile: a scene of palette content (display type, a
/// tertiary dot, a primary block on the canvas scenes) with a pane over
/// its lower part, carrying the pane's name, spec and use. The scene
/// supplies the backdrop the tint and its opacity play against.
struct GlassScene {
    /// The scene's ground: `surface-container-high` for the material
    /// tiles, the page canvas for the tint tiles.
    fill: Color32,
    /// Whether the scene carries the primary block (the canvas scenes).
    with_block: bool,
}

/// The tile's pane: which pane to paint and what to call it.
struct GlassPaneSpec {
    thickness: GlassThickness,
    tint: GlassTint,
    name: String,
}

/// Paint one glass demo tile: the scene, then the pane over its lower
/// part, chrome radius md, through the recipe so the specular edge and
/// the L2 shadow ride along.
fn glass_tile(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    width: f32,
    scene: GlassScene,
    pane: GlassPaneSpec,
) {
    let colors = theme.colors();
    let height = 212.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(radii::RADIUS_LG as u8);
    ui.painter().rect_filled(rect, radius, scene.fill);
    // The scene's art: display type and a dot, the palette content the
    // web preview scrolls under its panes.
    ui.painter().text(
        rect.left_top() + Vec2::new(spacing::SPACE_2, -spacing::SPACE_1),
        Align2::LEFT_TOP,
        "Aa",
        egui::FontId {
            size: 128.0,
            family: TypeStyle::Display.family(theme),
        },
        colors.primary,
    );
    ui.painter().circle_filled(
        rect.right_top() + Vec2::new(-spacing::SPACE_4, spacing::SPACE_8),
        spacing::SPACE_8,
        colors.tertiary,
    );
    if scene.with_block {
        ui.painter().rect_filled(
            Rect::from_min_size(
                rect.right_top(),
                Vec2::new(spacing::SPACE_12, spacing::SPACE_10),
            ),
            CornerRadius::same(0),
            colors.primary,
        );
    }
    let pane_rect = Rect::from_min_max(
        rect.left_bottom() + Vec2::new(spacing::SPACE_2, -96.0),
        rect.right_bottom() - Vec2::new(spacing::SPACE_2, spacing::SPACE_2),
    );
    let recipe = glass_recipe(pane.thickness, pane.tint, theme, flags);
    paint_glass(ui.painter(), pane_rect, radii::RADIUS_MD as u8, &recipe);
    let text = pane_rect.left_top() + Vec2::new(spacing::SPACE_3, spacing::SPACE_3);
    let line = spacing::SPACE_3 + 14.0;
    ui.painter().text(
        text,
        Align2::LEFT_TOP,
        pane.name,
        egui::FontId {
            size: 14.0,
            family: TypeStyle::Headline4.family(theme),
        },
        recipe.label,
    );
    ui.painter().text(
        text + Vec2::new(0.0, line),
        Align2::LEFT_TOP,
        format!(
            "blur {} · sat {:.0}%",
            pane.thickness.blur_px(),
            pane.thickness.saturation_percent()
        ),
        egui::FontId {
            size: 12.0,
            family: TypeStyle::Mono.family(theme),
        },
        recipe.label,
    );
    ui.painter().text(
        text + Vec2::new(0.0, line * 2.0),
        Align2::LEFT_TOP,
        pane.thickness.usage(),
        egui::FontId {
            size: 12.0,
            family: TypeStyle::Caption.family(theme),
        },
        recipe.label,
    );
}

/// The glass panes: the five thicknesses over palette content, then the
/// three tints beside the reduced-transparency fallback, one live pane
/// laid out as chrome, and the usage rules in the caption.
fn glass_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.label("Liquid glass · five thicknesses · 1px specular edge · shadow-l2");
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        for thickness in GlassThickness::ALL {
            glass_tile(
                ui,
                theme,
                flags,
                176.0,
                GlassScene {
                    fill: colors.surface_container_high,
                    with_block: false,
                },
                GlassPaneSpec {
                    thickness: *thickness,
                    tint: GlassTint::Surface,
                    name: sentence_case(thickness.name()),
                },
            );
            ui.add_space(spacing::SPACE_3);
        }
    });

    ui.add_space(spacing::SPACE_2);
    ui.label("Tints and the fallback · regular thickness · over the page canvas");
    ui.add_space(spacing::SPACE_1);
    let solid_flags = UiKitFlags {
        reduced_transparency: true,
        ..Default::default()
    };
    let tints: &[(GlassTint, &str, &UiKitFlags)] = &[
        (GlassTint::Surface, ".glass", flags),
        (GlassTint::Primary, ".glass--primary", flags),
        (GlassTint::Accent, ".glass--accent", flags),
        (GlassTint::Surface, "Reduced transparency", &solid_flags),
    ];
    ui.horizontal(|ui| {
        for (tint, name, pane_flags) in tints {
            glass_tile(
                ui,
                theme,
                pane_flags,
                224.0,
                GlassScene {
                    fill: colors.background,
                    with_block: true,
                },
                GlassPaneSpec {
                    thickness: GlassThickness::Regular,
                    tint: *tint,
                    name: (*name).to_owned(),
                },
            );
            ui.add_space(spacing::SPACE_3);
        }
    });

    // One live pane, laid out as chrome with content in it; the host's
    // reduced-transparency checkbox flips it solid in place.
    ui.add_space(spacing::SPACE_2);
    let recipe = LiquidGlass::new().recipe(theme, flags);
    LiquidGlass::new().ui(ui, theme, flags, |ui| {
        ui.set_width(spacing::SPACE_40 + spacing::SPACE_10);
        ui.add(egui::Label::new(
            egui::RichText::new("A pane as chrome, its labels in the recipe's colour")
                .color(recipe.label)
                .size(14.0)
                .family(TypeStyle::Headline4.family(theme)),
        ));
    });

    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        "Chrome and overlays only: top app bars, sheets, popovers, modal headers. Never body content, and never two panes of the same thickness stacked. Under reduced transparency every pane turns solid, as the fourth tile shows. egui composites no backdrop blur: the tint and its opacity carry the glass, and the 1px specular edge marks the pane.",
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
    )));
}

/// The components section: the surface components issue 08 adds, in
/// the preview's own copy.
fn components_section(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        "Components · Surfaces · Gothenburg · 2026",
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    cards_group(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);
    metrics_group(ui, theme);
    ui.add_space(spacing::SPACE_4);
    editorial_group(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);
    glass_group(ui, theme, flags);
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
                icons_section(ui, self.theme);
                ui.add_space(spacing::SPACE_6);
                logo_section(ui, self.theme);
                ui.add_space(spacing::SPACE_6);
                components_section(ui, self.theme, &self.flags);
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
