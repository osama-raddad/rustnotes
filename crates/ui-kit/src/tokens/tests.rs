//! Token tests: the generated values against the committed snapshot, the
//! brand-book anchors, the WCAG contrast floors and the approved scales.

use egui::Color32;

use crate::theme::ThemeId;
use crate::tokens::{
    breakpoints, containers, icons, leading, radii, shadows, spacing, tracking, type_scale,
    ThemeColors,
};

// WCAG helpers, the pattern of rustnotes-core's palette tests.

/// WCAG 2.x relative luminance of one channel.
fn channel(v: u8) -> f64 {
    let s = v as f64 / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(c: Color32) -> f64 {
    0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
}

/// WCAG contrast ratio (1..=21, bigger is better).
fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn at_least(fg: Color32, bg: Color32, ratio: f64, what: &str) {
    let got = contrast(fg, bg);
    assert!(
        got >= ratio,
        "{what}: contrast {got:.2} < {ratio} for {fg:?} on {bg:?}"
    );
}

fn parse_color(s: &str) -> Color32 {
    if let Some(hex) = s.strip_prefix('#') {
        Color32::from_rgb(
            u8::from_str_radix(&hex[0..2], 16).unwrap(),
            u8::from_str_radix(&hex[2..4], 16).unwrap(),
            u8::from_str_radix(&hex[4..6], 16).unwrap(),
        )
    } else {
        let inner = s
            .strip_prefix("rgba(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap();
        let n: Vec<u8> = inner
            .split(',')
            .map(|p| p.trim().parse().unwrap())
            .collect();
        Color32::from_rgba_unmultiplied_const(n[0], n[1], n[2], n[3])
    }
}

fn theme_by_id(id: &str) -> ThemeId {
    ThemeId::ALL
        .iter()
        .copied()
        .find(|t| t.id() == id)
        .unwrap_or_else(|| panic!("unknown theme id: {id}"))
}

/// Every role of every theme equals its snapshot line: the generated
/// values are exactly the JSON-resolved ones, rgba alphas included.
#[test]
fn snapshot_roles_match_the_generated_values() {
    let mut checked = 0;
    for line in include_str!("snapshot.txt").lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, ' ');
        let role = parts.next().unwrap();
        let theme = parts.next().unwrap();
        let value = parts.next().unwrap();
        let colors = theme_by_id(theme).colors();
        assert_eq!(
            colors.role(role),
            Some(parse_color(value)),
            "{theme}/{role} drifted from the snapshot"
        );
        checked += 1;
    }
    assert_eq!(checked, 54 * 6, "every role of every theme is pinned");
    // The name-keyed lookup agrees with the struct fields it wraps.
    let light = ThemeId::Light.colors();
    assert_eq!(light.role("md-sys-color-primary"), Some(light.primary));
    assert_eq!(light.role("no-such-role"), None);
}

/// Hand-checked anchors straight from the brand book, independent of the
/// generator: each direction's primary and canvas, plus one rgba role.
#[test]
fn anchors_match_the_brand_book() {
    assert_eq!(
        ThemeId::Light.colors().primary,
        parse_color("#1E3A2F"),
        "Deep Hunter Green"
    );
    assert_eq!(
        ThemeId::Light.colors().surface,
        parse_color("#EFE8DA"),
        "Warm Cream"
    );
    assert_eq!(
        ThemeId::Light.colors().tertiary,
        parse_color("#8B6F47"),
        "Bronze"
    );
    assert_eq!(
        ThemeId::Dark.colors().primary,
        parse_color("#88C3A8"),
        "forest-80"
    );
    assert_eq!(
        ThemeId::TealLight.colors().primary,
        parse_color("#145F64"),
        "Deep Teal"
    );
    assert_eq!(
        ThemeId::TealLight.colors().surface,
        parse_color("#F3F0E9"),
        "Cloud Dancer"
    );
    assert_eq!(
        ThemeId::TealDark.colors().primary,
        parse_color("#A2C8C9"),
        "teal-200"
    );
    assert_eq!(
        ThemeId::MonoLight.colors().primary,
        parse_color("#1C1917"),
        "Warm Charcoal"
    );
    assert_eq!(
        ThemeId::MonoLight.colors().surface,
        parse_color("#F5F1EA"),
        "Paper"
    );
    assert_eq!(
        ThemeId::MonoDark.colors().primary,
        parse_color("#ECE6D9"),
        "Parchment"
    );
    assert_eq!(
        ThemeId::MonoDark.colors().surface,
        parse_color("#14110F"),
        "Obsidian"
    );
    // rgba roles keep their translucency, alpha quantised to 255ths.
    assert_eq!(
        ThemeId::TealLight.colors().outline_variant,
        Color32::from_rgba_unmultiplied_const(13, 42, 44, 36),
        "teal-light outline-variant carries its 0.14 alpha"
    );
}

/// The contrast floors the brand book states: body pairs at 4.5:1, the
/// accent-text floor of 3:1 for on-primary (the pure-white exception)
/// and text-accent, and the non-text floor of 3:1 the README claims for
/// every border, ring and mark — focus-ring, outline and indicator on
/// their canvas — in all six themes.
#[test]
fn text_roles_clear_their_wcag_floors() {
    for theme in ThemeId::ALL {
        let c = theme.colors();
        let name = theme.name();
        at_least(
            c.on_primary,
            c.primary,
            3.0,
            &format!("{name}: on-primary on primary"),
        );
        at_least(
            c.on_surface,
            c.surface,
            4.5,
            &format!("{name}: on-surface on surface"),
        );
        at_least(
            c.on_surface_variant,
            c.surface_variant,
            4.5,
            &format!("{name}: on-surface-variant on surface-variant"),
        );
        at_least(
            c.text_accent,
            c.surface_container_low,
            3.0,
            &format!("{name}: text-accent on surface-container-low"),
        );
        at_least(
            c.focus_ring,
            c.surface,
            3.0,
            &format!("{name}: focus-ring on surface"),
        );
        at_least(
            c.outline,
            c.surface,
            3.0,
            &format!("{name}: outline on surface"),
        );
        at_least(
            c.indicator,
            c.surface,
            3.0,
            &format!("{name}: indicator on surface"),
        );
    }
}

/// The busy button's label ("Sending…") over its container over the
/// canvas, for every emphasis in all six themes, clears the 4.5:1 body
/// floor (2-A9, 2-I5). Busy is an active state, so the disabled label's
/// documented 38% does not apply to it; the three dark themes are the
/// night-mode half of the check.
#[test]
fn busy_label_clears_the_body_floor_in_all_six_themes() {
    use crate::components::{busy_look, ButtonEmphasis};
    use crate::states::composite_over;

    let mut failures = Vec::new();
    for theme in ThemeId::ALL {
        let c = theme.colors();
        for emphasis in ButtonEmphasis::ALL {
            let look = busy_look(*emphasis, *theme);
            let fill = composite_over(look.fill, c.surface);
            let label = composite_over(look.label, fill);
            let got = contrast(label, fill);
            if got < 4.5 {
                failures.push(format!(
                    "{} {emphasis:?}: busy label on its fill: contrast {got:.2} < 4.5 for {label:?} on {fill:?}",
                    theme.name()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The named spacing lookups return only approved values: the README
/// scale plus the documented 1px hairline exception, exactly.
#[test]
fn spacing_scale_is_exactly_the_approved_one() {
    let approved: &[f32] = &[
        0.0, 1.0, 2.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 40.0, 48.0, 64.0, 96.0, 128.0, 160.0,
    ];
    let values: Vec<f32> = spacing::ALL.iter().map(|(_, v)| *v).collect();
    assert_eq!(
        values, approved,
        "the README scale plus the space-px hairline exception"
    );
    assert_eq!(spacing::SPACE_1, 4.0, "the base unit");
    assert_eq!(spacing::SPACE_6, 24.0);
}

/// The radius lookups return only approved values: none, xs, sm, md, lg,
/// xl, 2xl and full, exactly.
#[test]
fn radius_scale_is_exactly_the_approved_one() {
    let approved: &[f32] = &[0.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0, 9999.0];
    let values: Vec<f32> = radii::ALL.iter().map(|(_, v)| *v).collect();
    assert_eq!(values, approved, "the README radius scale");
    assert_eq!(radii::RADIUS_MD, 8.0, "the default button and input radius");
    assert_eq!(radii::RADIUS_LG, 12.0, "cards");
}

/// The per-theme shadow strings parse into typed layers, multi-layer
/// scales and negative spreads included.
#[test]
fn shadow_layers_parse_from_the_css_strings() {
    let forest = shadows::LIGHT;
    assert_eq!(forest.l1.len(), 1, "forest L1 is a single layer");
    assert_eq!(forest.l1[0].x, 0.0);
    assert_eq!(forest.l1[0].y, 1.0);
    assert_eq!(forest.l1[0].blur, 2.0);
    assert_eq!(
        forest.l1[0].color,
        Color32::from_rgba_unmultiplied_const(30, 58, 47, 20),
        "forest ink at 0.08 alpha"
    );
    assert_eq!(
        shadows::TEAL_LIGHT.l2.len(),
        2,
        "teal light L2 keeps both layers"
    );
    assert_eq!(
        shadows::MONO_LIGHT.l1[0].spread,
        -1.0,
        "monochrome L1 shrinks"
    );
    assert!(
        shadows::GLASS_SPECULAR_LIGHT[0].inset,
        "the glass specular edge is inset"
    );
    // Every theme resolves all five levels through its ThemeId.
    for theme in ThemeId::ALL {
        let s = theme.shadows();
        for level in [s.l1, s.l2, s.l3, s.l4, s.l5] {
            assert!(
                !level.is_empty(),
                "{}: every level carries layers",
                theme.name()
            );
        }
    }
}

/// Spot-checks that the scalar modules parsed their units: px, em, unit
/// less ratios and negative axis values all survive generation.
#[test]
fn scalar_tokens_parse_with_their_units() {
    assert_eq!(type_scale::TEXT_BODY_MD, 16.0, "the scale anchor");
    assert_eq!(tracking::TRACKING_EYEBROW, 0.24, "the signature +24%");
    assert_eq!(leading::LEADING_NORMAL, 1.5);
    assert_eq!(breakpoints::BREAKPOINT_TABLET, 600.0);
    assert_eq!(
        containers::CONTAINER_MAX_CONTENT,
        720.0,
        "the reading line cap"
    );
    assert_eq!(icons::ICON_SIZE_LG, 24.0, "the default icon size");
    assert_eq!(
        icons::ICON_GRADE_DARK,
        -25.0,
        "grade thins dark-theme glyphs"
    );
    assert_eq!(ThemeColors::ROLES.len(), 54);
}
