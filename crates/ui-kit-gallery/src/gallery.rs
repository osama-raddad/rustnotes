//! The Vynatix gallery in its final shape (issues 09, 10 and 11): a
//! Liquid Glass TopAppBar carrying the wordmark, the primary nav, the
//! search affordance, the theme menu and the one call to action; a
//! Tabs strip switching the gallery's panels — the Tabs widget's own
//! real demo — over a scrolling page; and, on the Feedback tab, the
//! Banner demos at the panel top and the Snackbar host's trigger
//! buttons, the snackbar itself floating in near the window's bottom.
//! Run with `cargo run -p ui-kit --example gallery`.
//!
//! The panel grouping (the judgment issue 09 documented, extended by
//! issue 10): the Foundations tab holds the type specimen, the
//! interaction states, the foundation specimens (spacing, radii,
//! elevation), the icons, the logo and the colour-role swatches; the
//! Components tab holds the actions (buttons, chips, badges), the
//! surfaces (cards, metrics, editorial, glass), the text inputs and
//! the choice controls (checkboxes, radio groups, toggles); the
//! Feedback tab holds the banners and the snackbars; the Brand tab
//! holds the gallery's three brand-book pages. The bar's nav links
//! and the tab strip drive the same panel state — the link's
//! `aria-current`-style underline and the tab's indicator are the two
//! accents the specs give them. Themes and the reduced-motion and
//! reduced-transparency flags live in the bar's theme menu, all six
//! reachable.
//!
//! The Brand tab and the cover (issue 10's judgment, documented): the
//! issue wants the Cover as the gallery's first screen, and the
//! gallery's chrome is the bar-plus-tabs of issue 09. The
//! reconciliation is a Cover panel as the FIRST tab, named Brand: the
//! gallery opens on the cover composition (eyebrow, display
//! headline, standfirst, the mark, the band), and the Themes and
//! Brand showcases follow it on the same panel. The cover is a panel
//! rather than an intro overlay so the final chrome stays put (the
//! bar and the tab strip never disappear), and it is the first tab
//! rather than folded under Foundations so the brand book opens the
//! gallery the way the source system's brand book opens on its cover.
//! The theme cards on that panel need every theme's faces, so
//! startup installs all six themes' fonts (the installs are additive
//! by design); the theme menu's per-switch install stays as it was.
//!
//! The polish pass (issue 11): every string the gallery paints is
//! routed through `ui_kit::gallery_copy` as a constant, where the
//! voice lint scans it (no em dashes, no stray ellipses, no
//! exclamation marks, no emoji, no forbidden vocabulary, sentence
//! case for headings); the specimen rows that can outgrow a narrow
//! window (the state tiles, the radii, the metrics, the editorial
//! trio, the glass panes) scroll horizontally instead of clipping
//! their tails, the way the web previews scroll the same rows.
//!
//! This module is the app's whole UI, hosted by the `rustnotes` host
//! through `GalleryApp` (lib.rs): the host supplies the platform glue
//! (safe areas, keyboard, appearance, wgpu limits, insets) and calls
//! [`Gallery::chrome`] as the root UI each frame. Run it on desktop with
//! `cargo run -p ui-kit-gallery`, on the iOS simulator with
//! `just run-ios app=ui-kit-gallery`, on a connected iPhone with
//! `just ios-install app=ui-kit-gallery`, and on Android with
//! `just run-android app=ui-kit-gallery`.

use egui::{self, Align2, Color32, CornerRadius, Rect, Sense, Vec2};

use ui_kit::components::badge::{Badge, IconButton};
use ui_kit::components::banner::{Banner, BannerEvent, BannerTone};
use ui_kit::components::button::{Button, ButtonEmphasis};
use ui_kit::components::checkbox::{Checkbox, CheckboxState};
use ui_kit::components::chip::{Chip, ChipEvent};
use ui_kit::components::liquid_glass::{glass_recipe, paint_glass, GlassThickness, GlassTint};
use ui_kit::components::radio::{option, RadioGroup, RadioOption};
use ui_kit::components::select::{popup_frame, Select};
use ui_kit::components::snackbar::{Snackbar, SnackbarEvent, SnackbarHost};
use ui_kit::components::tabs::Tabs;
use ui_kit::components::text_field::{hint_row, label_row, TextField};
use ui_kit::components::toggle::Toggle;
use ui_kit::components::top_app_bar::{
    NavLink, TopAppBar, ACTIONS_GAP, GROUP_GAP, LINK_GAP, LOGO_HEIGHT,
};
use ui_kit::components::{
    BrandShowcase, Card, Cover, EditorialCard, FocusRing, LiquidGlass, MetricCard, ThemesShowcase,
};
use ui_kit::gallery_copy as copy;
use ui_kit::icon::{icon, icon_active, IconSize};
use ui_kit::icons::{Icon, CATEGORY_GROUPS};
use ui_kit::logo::Logo;
use ui_kit::states::{self, UiKitFlags};
use ui_kit::theme::{egui_shadow, ThemeId};
use ui_kit::tokens::{radii, spacing, ThemeColors};
use ui_kit::typography::{install, specimen_job, TrackedLabel, TypeStyle};

/// The width under which the gallery switches to its phone layout: the
/// bar drops its nav links (the Tabs strip below already switches
/// panels) and its call to action, and two-column grids stack. 600 is
/// the design system's own compact breakpoint (`breakpoint-compact`).
const NARROW_WIDTH: f32 = 600.0;

impl Gallery {
    /// The gallery's starting state: the Brand tab, every demo
    /// populated.
    pub(crate) fn new() -> Self {
        Self {
            theme: ThemeId::Light,
            flags: UiKitFlags::default(),
            filters: [false; 4],
            saved_chip: true,
            form: ProjectForm::populated(),
            choices: ChoiceDemos::populated(),
            tab: 0,
            scroll_offset: 0.0,
            touch_scrolling: false,
            last_scroll_offset: egui::Vec2::ZERO,
            banners: BannerDemos::default(),
            snackbar: SnackbarHost::new(),
            search: String::new(),
            search_open: false,
        }
    }
}

/// A specimen row that can outgrow the page: laid out horizontally
/// inside a horizontal scroll, so a narrow window scrolls the row
/// instead of clipping its tail. The web previews scroll the same
/// rows; `id` keeps the scroll areas distinct.
fn scrolling_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::horizontal()
        .id_salt(id)
        .show(ui, |ui| ui.horizontal(add));
}

/// Pair the option enums with their labels from `gallery_copy`: the
/// labels are copy, the enums are the form's state.
fn pairs<T: Copy>(values: &[T], labels: &[&'static str]) -> Vec<(T, &'static str)> {
    values
        .iter()
        .zip(labels)
        .map(|(value, label)| (*value, *label))
        .collect()
}

/// The radio flavour of [`pairs`]: `RadioOption` rows for a group.
fn radio_options<T: Copy>(values: &[T], labels: &[&'static str]) -> Vec<RadioOption<'static, T>> {
    values
        .iter()
        .zip(labels)
        .map(|(value, label)| option(*value, label))
        .collect()
}

/// The gallery's whole UI state and every panel. Constructed with
/// [`Gallery::new`], driven each frame by [`Gallery::chrome`].
pub struct Gallery {
    /// The live theme; read by the app crate to paint the viewport
    /// margins behind the safe areas.
    pub(crate) theme: ThemeId,
    /// The kit flags this host passes down, standing in for the web
    /// system's reduced-motion and reduced-transparency media queries;
    /// the host maps the OS reduce-motion preference onto them.
    pub(crate) flags: UiKitFlags,
    /// The practice-area filter chips' selected states, so the chips
    /// demo section toggles live.
    filters: [bool; 4],
    /// Whether the dismissible chip demo still holds its chip.
    saved_chip: bool,
    /// The project-details form the text inputs section edits.
    form: ProjectForm,
    /// The choice-controls demos' state (issue 06): the notification
    /// checkboxes, the practice-area children whose parent derives,
    /// and the local toggle's held value.
    choices: ChoiceDemos,
    /// The active gallery panel, driven by the tab strip and the
    /// bar's nav links alike.
    pub(crate) tab: usize,
    /// The page's scroll offset: driven by the touch-scroll shim while
    /// a finger drags (`touch_scrolling`), and fed back from the
    /// ScrollArea every frame so the wheel path and clamping hold.
    pub(crate) scroll_offset: f32,
    pub(crate) touch_scrolling: bool,
    /// The main scroll area's offset after the last frame (tests).
    pub(crate) last_scroll_offset: egui::Vec2,
    /// Which of the dismissible banner demos still hold their
    /// banners; the warning and the error stay until resolved.
    banners: BannerDemos,
    /// The snackbar host: one brief operational message at a time.
    snackbar: SnackbarHost,
    /// The bar's search field buffer.
    search: String,
    /// Whether the bar's search popover is open.
    search_open: bool,
}

/// The dismissible banner demos' state: the info and success banners
/// come back on demand; the warning and the error never leave on
/// their own.
#[derive(Default)]
struct BannerDemos {
    info: bool,
    success: bool,
}

/// The choice-controls demos' state: the notification checkboxes, the
/// practice-area children (their parent's tri-state derives from
/// them), and the one local toggle's held value.
struct ChoiceDemos {
    weekly_digest: bool,
    blocked_builds: bool,
    areas: [bool; 3],
    remember_filters: bool,
}

impl ChoiceDemos {
    /// The Checkbox preview's own starting states: the digest on, the
    /// practice areas partly chosen, so the parent opens
    /// indeterminate.
    fn populated() -> Self {
        Self {
            weekly_digest: true,
            blocked_builds: false,
            areas: [true, false, true],
            remember_filters: false,
        }
    }

    /// The practice-area parent's state over its children, the
    /// preview's sync script.
    fn parent(&self) -> CheckboxState {
        CheckboxState::from_children(
            self.areas.len(),
            self.areas.iter().filter(|checked| **checked).count(),
        )
    }
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
        copy::TYPE_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    for (style, sample) in copy::SPECIMEN {
        let color = specimen_color(*style, &colors);
        if *style == TypeStyle::Eyebrow {
            ui.add(TrackedLabel::new(
                sample,
                style.font_id(theme),
                style.tracking(),
                color,
            ));
        } else {
            ui.add(egui::Label::new(specimen_job(
                sample,
                *style,
                theme,
                color,
                Some(ui.available_width()),
            )));
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
            Some(ui.available_width()),
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
            copy::START_A_PROJECT,
            TypeStyle::Body.font_id(theme),
            text,
        );
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            caption,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(ui.available_width()),
        )));
    });
}

/// The interaction states: hover, press, focus and disabled as
/// specimens beside a control that really takes focus via Tab.
fn states_section(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::STATES_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    let rest = colors.primary;
    let hover = states::composite_over(states::hover_overlay(colors.on_primary), rest);
    let pressed = states::composite_over(states::pressed_overlay(colors.on_primary), rest);
    let disabled = states::disabled(colors.on_surface, colors.surface_container);

    // The captions outsize their tiles, so the row scrolls rather
    // than clip on a narrow window.
    scrolling_row(ui, "states-tiles", |ui| {
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[0],
            rest,
            1.0,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[1],
            hover,
            1.0,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[2],
            pressed,
            states::PRESS_SCALE,
            colors.on_primary,
            false,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[3],
            rest,
            1.0,
            colors.on_primary,
            true,
        );
        ui.add_space(spacing::SPACE_2);
        state_tile(
            ui,
            theme,
            copy::STATE_CAPTIONS[4],
            disabled.container,
            1.0,
            disabled.content,
            false,
        );
    });

    ui.add_space(spacing::SPACE_2);
    FocusRing::new().wrap(ui, theme, flags, |ui| {
        ui.add(egui::Button::new(copy::STATES_TAB_BUTTON))
    });
    ui.add(egui::Label::new(specimen_job(
        copy::STATES_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The foundations as specimens: the spacing scale, all eight radii and
/// the active theme's five elevation levels.
fn foundations_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::FOUNDATIONS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    ui.label(copy::SPACING_LABEL);
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
                    Some(ui.available_width()),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });

    ui.add_space(spacing::SPACE_4);
    ui.label(copy::RADII_LABEL);
    ui.add_space(spacing::SPACE_2);
    // Eight labelled swatches run wide; the row scrolls.
    scrolling_row(ui, "radii-row", |ui| {
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
                    Some(ui.available_width()),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });

    ui.add_space(spacing::SPACE_4);
    ui.label(format!("{} · {}", copy::ELEVATION_LABEL, theme.name()));
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
                    Some(ui.available_width()),
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
        copy::ICONS_EYEBROW,
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
                    Some(ui.available_width()),
                )));
            });
            ui.add_space(spacing::SPACE_2);
        }
    });
    ui.add(egui::Label::new(specimen_job(
        copy::ICONS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
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
                        Some(ui.available_width()),
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
        Some(ui.available_width()),
    )));
}

/// The logo: the mark and wordmark on the canvas, reversed on the
/// primary and on an inverse band, the accent moment, and the size
/// floors in the caption.
fn logo_section(ui: &mut egui::Ui, theme: ThemeId) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::LOGO_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    logo_band(ui, theme, copy::LOGO_ON_CANVAS, None, |ui| {
        ui.add(Logo::mark().height(32.0));
        ui.add_space(spacing::SPACE_6);
        ui.add(Logo::wordmark());
    });
    ui.add_space(spacing::SPACE_2);
    logo_band(
        ui,
        theme,
        copy::LOGO_ON_PRIMARY,
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
        copy::LOGO_ON_INVERSE,
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
    logo_band(ui, theme, copy::LOGO_ACCENT_MOMENT, None, |ui| {
        ui.add(Logo::mark().height(32.0).ink(ui_kit::logo::LogoInk::Accent));
    });
    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        copy::LOGO_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
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
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_1);
        ui.add(egui::Label::new(specimen_job(
            description,
            TypeStyle::Caption,
            theme,
            colors.text_muted(),
            Some(ui.available_width()),
        )));
    });
}

/// The four card variants in the Card preview's two-column grid:
/// elevated (the default), outlined, filled and interactive, the last
/// one a real control to hover and tab to.
fn cards_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    ui.label(copy::CARDS_LABEL);
    ui.add_space(spacing::SPACE_2);
    egui::Grid::new("surfaces-cards")
        .num_columns(2)
        .spacing([spacing::SPACE_5, spacing::SPACE_5])
        .show(ui, |ui| {
            card_specimen(
                ui,
                theme,
                flags,
                copy::CARD_DEFAULT_CLASS,
                copy::CARD_DEFAULT_DESC,
                Card::new(),
            );
            ui.end_row();
            card_specimen(
                ui,
                theme,
                flags,
                copy::CARD_OUTLINED_CLASS,
                copy::CARD_OUTLINED_DESC,
                Card::new().outlined(),
            );
            card_specimen(
                ui,
                theme,
                flags,
                copy::CARD_FILLED_CLASS,
                copy::CARD_FILLED_DESC,
                Card::new().filled().compact(),
            );
            ui.end_row();
            card_specimen(
                ui,
                theme,
                flags,
                copy::CARD_INTERACTIVE_CLASS,
                copy::CARD_INTERACTIVE_DESC,
                Card::new().interactive(),
            );
            ui.end_row();
            card_specimen(
                ui,
                theme,
                flags,
                copy::CARD_LARGE_CLASS,
                copy::CARD_LARGE_DESC,
                Card::new().large(),
            );
            ui.end_row();
        });
}

/// The metric trio: one specific, sourced number per card, per the
/// MetricCard preview's own copy.
fn metrics_group(ui: &mut egui::Ui, theme: ThemeId) {
    ui.label(copy::METRICS_LABEL);
    ui.add_space(spacing::SPACE_2);
    // Three cards side by side run near the page width; the row
    // scrolls on a narrower window.
    scrolling_row(ui, "metrics-row", |ui| {
        MetricCard::new(
            copy::METRIC_FLEET_LABEL,
            copy::METRIC_FLEET_FIGURE,
            copy::METRIC_FLEET_SOURCE,
        )
        .ui(ui, theme);
        ui.add_space(spacing::SPACE_5);
        MetricCard::new(
            copy::METRIC_TENURE_LABEL,
            copy::METRIC_TENURE_FIGURE,
            copy::METRIC_TENURE_SOURCE,
        )
        .ui(ui, theme);
        ui.add_space(spacing::SPACE_5);
        MetricCard::new(
            copy::METRIC_SENIOR_LABEL,
            copy::METRIC_SENIOR_FIGURE,
            copy::METRIC_SENIOR_SOURCE,
        )
        .unit(copy::METRIC_SENIOR_UNIT)
        .ui(ui, theme);
    });
}

/// The editorial row: a case study, a field note and an open role,
/// image areas as the placeholder tiles until photography exists.
fn editorial_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    ui.label(copy::EDITORIAL_LABEL);
    ui.add_space(spacing::SPACE_2);
    scrolling_row(ui, "editorial-row", |ui| {
        EditorialCard::new(
            copy::EDITORIAL_CASE_EYEBROW,
            copy::EDITORIAL_CASE_HEADLINE,
            copy::EDITORIAL_CASE_BODY,
        )
        .action(copy::EDITORIAL_CASE_ACTION)
        .ui(ui, theme, flags);
        ui.add_space(spacing::SPACE_5);
        EditorialCard::new(
            copy::EDITORIAL_FIELD_EYEBROW,
            copy::EDITORIAL_FIELD_HEADLINE,
            copy::EDITORIAL_FIELD_BODY,
        )
        .action(copy::EDITORIAL_FIELD_ACTION)
        .ui(ui, theme, flags);
        ui.add_space(spacing::SPACE_5);
        EditorialCard::new(
            copy::EDITORIAL_ROLE_EYEBROW,
            copy::EDITORIAL_ROLE_HEADLINE,
            copy::EDITORIAL_ROLE_BODY,
        )
        .action(copy::EDITORIAL_ROLE_ACTION)
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

/// The type-specimen glyph the glass scenes set behind their panes:
/// display-scale letters for the tint to play against.
const GLASS_SCENE_GLYPH: &str = "Aa";

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
        GLASS_SCENE_GLYPH,
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
    ui.label(copy::GLASS_LABEL);
    ui.add_space(spacing::SPACE_2);
    // Five panes with their captions run past the page width; the
    // row scrolls.
    scrolling_row(ui, "glass-thicknesses", |ui| {
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
    ui.label(copy::GLASS_TINTS_LABEL);
    ui.add_space(spacing::SPACE_1);
    let solid_flags = UiKitFlags {
        reduced_transparency: true,
        ..Default::default()
    };
    let tints: &[(GlassTint, &str, &UiKitFlags)] = &[
        (GlassTint::Surface, copy::GLASS_SURFACE_CLASS, flags),
        (GlassTint::Primary, copy::GLASS_PRIMARY_CLASS, flags),
        (GlassTint::Accent, copy::GLASS_ACCENT_CLASS, flags),
        (GlassTint::Surface, copy::GLASS_FALLBACK_NAME, &solid_flags),
    ];
    scrolling_row(ui, "glass-tints", |ui| {
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
            egui::RichText::new(copy::GLASS_CHROME_LABEL)
                .color(recipe.label)
                .size(14.0)
                .family(TypeStyle::Headline4.family(theme)),
        ));
    });

    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        copy::GLASS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

// ---- the action components (issue 05) --------------------------------------

/// The buttons group: the four emphases in the Button preview's own
/// copy, the three sizes, and the icon variants, every one a live
/// control (hover, hold, Tab) with the disabled and busy pair.
fn buttons_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.label(copy::BUTTONS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        Button::new(copy::START_A_PROJECT).ui(ui, theme, flags);
        Button::new(copy::BUTTON_READ_CASE_STUDY)
            .emphasis(ButtonEmphasis::Secondary)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_HIRE_A_TEAM)
            .emphasis(ButtonEmphasis::Tertiary)
            .icon_trailing(Icon::ArrowForward)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_CANCEL)
            .emphasis(ButtonEmphasis::Ghost)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_DISABLED)
            .disabled()
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_BUSY)
            .disabled()
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::BUTTONS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.label(copy::SIZES_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        Button::new(copy::BUTTON_OPEN_DASHBOARD)
            .large()
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_SAVE_CHANGES).ui(ui, theme, flags);
        Button::new(copy::BUTTON_FILTER_RESULTS)
            .small()
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::SIZES_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.label(copy::BUTTON_ICONS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        Button::new(copy::BUTTON_SEARCH_ARCHIVE)
            .icon_leading(Icon::Search)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_GET_PROPOSAL)
            .icon_trailing(Icon::ArrowForward)
            .ui(ui, theme, flags);
        Button::new(copy::BUTTON_VISIT_STUDIO)
            .icon_trailing(Icon::ArrowOutward)
            .ui(ui, theme, flags);
        Button::new("")
            .icon_only(Icon::Search, copy::BUTTON_SEARCH_EVERYTHING)
            .ui(ui, theme, flags);
        Button::new("")
            .icon_only(Icon::ArrowOutward, copy::BUTTON_OPEN_EXTERNAL)
            .emphasis(ButtonEmphasis::Secondary)
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::BUTTON_ICONS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.add(egui::Label::new(specimen_job(
        copy::TERTIARY_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The chips group: the preview's practice-area filter row as live
/// toggles, the featured label, and a dismissible chip reporting its
/// typed event.
fn chips_group(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    filters: &mut [bool; 4],
    saved: &mut bool,
) {
    let colors = theme.colors();
    ui.label(copy::CHIPS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.add(TrackedLabel::new(
        copy::CHIPS_PRACTICE_AREA,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_1);
    ui.horizontal_wrapped(|ui| {
        for (label, selected) in copy::CHIP_FILTERS.iter().zip(filters.iter_mut()) {
            if let Some(ChipEvent::Toggled(now)) = Chip::new(label)
                .set_selected(*selected)
                .ui(ui, theme, flags)
                .event
            {
                *selected = now;
            }
        }
        ui.add_space(spacing::SPACE_4);
        Chip::new(copy::CHIP_FEATURED).accent().ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::CHIPS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));

    ui.add_space(spacing::SPACE_3);
    ui.horizontal(|ui| {
        if *saved {
            if let Some(event) = Chip::new(copy::CHIP_CLIENT_WORK)
                .dismissible()
                .ui(ui, theme, flags)
                .event
            {
                if event == ChipEvent::Dismissed {
                    *saved = false;
                }
            }
        } else {
            if Button::new(copy::CHIP_RESTORE)
                .emphasis(ButtonEmphasis::Ghost)
                .ui(ui, theme, flags)
                .clicked
            {
                *saved = true;
            }
        }
    });
    ui.add(egui::Label::new(specimen_job(
        copy::CHIP_DISMISS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The badges group: the six tones in the Badge preview's own words,
/// and the count badges on their 44px icon buttons.
fn badges_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.label(copy::BADGES_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        for (label, tone) in copy::BADGES {
            Badge::new(label, *tone).ui(ui, theme);
        }
    });
    ui.add_space(spacing::SPACE_2);
    ui.horizontal(|ui| {
        IconButton::new(copy::BADGE_NOTIFICATIONS, Icon::Notifications)
            .count(3)
            .ui(ui, theme, flags);
        IconButton::new(copy::BADGE_MAIL, Icon::Mail)
            .count(120)
            .ui(ui, theme, flags);
        IconButton::new(copy::BADGE_MESSAGES, Icon::Chat)
            .activity_dot()
            .ui(ui, theme, flags);
    });
    ui.add(egui::Label::new(specimen_job(
        copy::BADGES_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The actions section: every Button, Chip and Badge variant as live,
/// Tab-focusable instances in the active theme.
fn actions_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    filters: &mut [bool; 4],
    saved: &mut bool,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::ACTIONS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    buttons_group(ui, theme, flags);
    ui.add_space(spacing::SPACE_4);
    chips_group(ui, theme, flags, filters, saved);
    ui.add_space(spacing::SPACE_4);
    badges_group(ui, theme, flags);
}

/// The components section: the surface components issue 08 adds, in
/// the preview's own copy.
fn components_section(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::SURFACES_EYEBROW,
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

// ---- the text input components (issue 07) ----------------------------------

/// The region options, the Radio preview's example domain, shared with
/// this section's select.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Nordics,
    Europe,
    Global,
}

const REGION_VALUES: &[Region] = &[Region::Nordics, Region::Europe, Region::Global];

/// The team sizes, the Select preview's own options.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TeamSize {
    OneToTwo,
    ThreeToFive,
    SixOrMore,
}

const TEAM_SIZE_VALUES: &[TeamSize] = &[
    TeamSize::OneToTwo,
    TeamSize::ThreeToFive,
    TeamSize::SixOrMore,
];

/// The delivery offices: nine of them, so the list scrolls inside the
/// eight rows the kit caps it at.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Office {
    Gothenburg,
    Stockholm,
    Copenhagen,
    Oslo,
    Helsinki,
    Berlin,
    Amsterdam,
    London,
    Edinburgh,
}

const OFFICE_VALUES: &[Office] = &[
    Office::Gothenburg,
    Office::Stockholm,
    Office::Copenhagen,
    Office::Oslo,
    Office::Helsinki,
    Office::Berlin,
    Office::Amsterdam,
    Office::London,
    Office::Edinburgh,
];

/// One invoice currency, locked by the contract: the disabled select.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Currency {
    Sek,
}

/// The project-details form the section edits, with the email's
/// touched flag carrying the validate-on-leave rule.
#[derive(Default)]
struct ProjectForm {
    name: String,
    email: String,
    reference: String,
    search: String,
    brief: String,
    /// Whether the person has left the email field yet: the field
    /// validates on leave, not while they type.
    email_touched: bool,
    region: Option<Region>,
    team_size: Option<TeamSize>,
    office: Option<Office>,
}

impl ProjectForm {
    fn populated() -> Self {
        Self {
            reference: copy::REFERENCE_DEFAULT.to_owned(),
            brief: copy::BRIEF_DEFAULT.to_owned(),
            region: Some(Region::Nordics),
            office: Some(Office::Gothenburg),
            ..Default::default()
        }
    }
}

/// A work email is valid when it has one @ with something before it and
/// a domain with a dot inside it: loose enough for the demo, strict
/// enough to be worth fixing.
fn email_is_valid(email: &str) -> bool {
    match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain
                    .split_once('.')
                    .is_some_and(|(head, tail)| !head.is_empty() && !tail.is_empty())
        }
        None => false,
    }
}

/// The form's left column: the project name and the email with its
/// validate-on-leave rule.
fn fields_left(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    TextField::new(copy::FIELD_NAME_LABEL, &mut form.name)
        .placeholder(copy::FIELD_NAME_PLACEHOLDER)
        .hint(copy::FIELD_NAME_HINT)
        .ui(ui, theme, flags);

    // The email validates when the person leaves the field, not
    // while they type: the first leave arms it, and the error
    // clears as soon as the address is fixed.
    let invalid = form.email_touched && !email_is_valid(&form.email);
    let mut email = TextField::new(copy::FIELD_EMAIL_LABEL, &mut form.email)
        .placeholder(copy::FIELD_EMAIL_PLACEHOLDER)
        .hint(copy::FIELD_EMAIL_HINT);
    if invalid {
        email = email.error(copy::EMAIL_ERROR);
    }
    let email_field = email.ui(ui, theme, flags);
    if email_field.lost_focus {
        form.email_touched = true;
    }
}

/// The form's right column: the search field and the locked reference.
fn fields_right(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    TextField::new(copy::FIELD_SEARCH_LABEL, &mut form.search)
        .placeholder(copy::SEARCH_PLACEHOLDER)
        .leading_icon(Icon::Search)
        .ui(ui, theme, flags);

    TextField::new(copy::FIELD_REFERENCE_LABEL, &mut form.reference)
        .disabled()
        .hint(copy::SET_BY_CONTRACT)
        .ui(ui, theme, flags);
}

/// The fields group: the project-details form's text fields, the
/// preview's own grid of variants as live controls. Wide windows set
/// the two columns side by side as the preview does; phone widths
/// stack them.
fn fields_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let colors = theme.colors();
    ui.label(copy::FIELDS_LABEL);
    ui.add_space(spacing::SPACE_2);
    if ui.available_width() < NARROW_WIDTH {
        fields_left(ui, theme, flags, form);
        ui.add_space(spacing::SPACE_4);
        fields_right(ui, theme, flags, form);
    } else {
        ui.columns(2, |columns| {
            let (left, right) = columns.split_at_mut(1);
            fields_left(&mut left[0], theme, flags, form);
            fields_right(&mut right[0], theme, flags, form);
        });
    }

    // The textarea, spanning both columns.
    TextField::new(copy::FIELD_BRIEF_LABEL, &mut form.brief)
        .placeholder(copy::FIELD_BRIEF_PLACEHOLDER)
        .multiline(3)
        .ui(ui, theme, flags);

    ui.add(egui::Label::new(specimen_job(
        copy::FIELDS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The selects' left column: the region and the delivery office.
fn selects_left(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let regions = pairs(REGION_VALUES, copy::REGIONS);
    let offices = pairs(OFFICE_VALUES, copy::OFFICES);
    let region = Select::new(copy::SELECT_REGION_LABEL)
        .options(&regions)
        .selected(form.region)
        .hint(copy::SELECT_REGION_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = region.committed {
        form.region = Some(picked);
    }

    let office = Select::new(copy::SELECT_OFFICE_LABEL)
        .options(&offices)
        .selected(form.office)
        .hint(copy::SELECT_OFFICE_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = office.committed {
        form.office = Some(picked);
    }
}

/// The selects' right column: the team size and the locked currency.
fn selects_right(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let teams = pairs(TEAM_SIZE_VALUES, copy::TEAM_SIZES);
    let currencies = pairs(&[Currency::Sek], copy::CURRENCIES);
    let team = Select::new(copy::SELECT_TEAM_LABEL)
        .options(&teams)
        .selected(form.team_size)
        .placeholder(copy::SELECT_TEAM_PLACEHOLDER)
        .hint(copy::SELECT_TEAM_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = team.committed {
        form.team_size = Some(picked);
    }

    Select::new(copy::SELECT_CURRENCY_LABEL)
        .options(&currencies)
        .selected(Some(Currency::Sek))
        .disabled()
        .hint(copy::SET_BY_CONTRACT)
        .ui(ui, theme, flags);
}

/// The selects group: the region with a choice made, the team size on
/// its placeholder, a nine-office list that scrolls, and the locked
/// currency. Wide windows pair the columns as the preview does; phone
/// widths stack them.
fn selects_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    let colors = theme.colors();
    ui.label(copy::SELECTS_LABEL);
    ui.add_space(spacing::SPACE_2);
    if ui.available_width() < NARROW_WIDTH {
        selects_left(ui, theme, flags, form);
        ui.add_space(spacing::SPACE_4);
        selects_right(ui, theme, flags, form);
    } else {
        ui.columns(2, |columns| {
            let (left, right) = columns.split_at_mut(1);
            selects_left(&mut left[0], theme, flags, form);
            selects_right(&mut right[0], theme, flags, form);
        });
    }

    ui.add(egui::Label::new(specimen_job(
        copy::SELECTS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The text inputs section: a believable project-details form, every
/// field and select a live control in the active theme.
fn text_inputs_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    form: &mut ProjectForm,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::INPUTS_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    fields_group(ui, theme, flags, form);
    ui.add_space(spacing::SPACE_4);
    selects_group(ui, theme, flags, form);
}

// ---- the choice controls (issue 06) ----------------------------------------

/// The start options, the Radio preview's invalid-group specimen. The
/// group carries no state: the error is the demo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Start {
    ThisQuarter,
    NextQuarter,
    Undecided,
}

const START_VALUES: &[Start] = &[Start::ThisQuarter, Start::NextQuarter, Start::Undecided];

/// One nested checkbox row at the preview's 28px indent, its typed
/// toggle handed back.
fn nested_checkbox_row(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    label: &str,
    checked: bool,
) -> Option<CheckboxState> {
    let mut toggled = None;
    ui.horizontal(|ui| {
        ui.add_space(
            ui_kit::components::checkbox::BOX_SIZE + ui_kit::components::checkbox::LABEL_GAP,
        );
        toggled = Checkbox::new(label)
            .checked(checked)
            .ui(ui, theme, flags)
            .toggled;
    });
    toggled
}

/// The checkboxes group: the practice areas with their tri-state
/// parent, and the notifications with the hint and disabled
/// specimens — the Checkbox preview's own two columns.
fn checkboxes_group(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    choices: &mut ChoiceDemos,
) {
    ui.label(copy::CHECKBOXES_LABEL);
    ui.add_space(spacing::SPACE_2);

    label_row(ui, theme, copy::CHECK_AREAS_LABEL);
    hint_row(ui, theme, copy::CHECK_AREAS_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(toggled) = Checkbox::new(copy::CHECK_PARENT_LABEL)
        .state(choices.parent())
        .ui(ui, theme, flags)
        .toggled
    {
        // The parent's click sets every child, the preview's script.
        choices.areas = [toggled.is_checked(); 3];
    }
    for (index, area) in copy::PRACTICE_AREAS.iter().enumerate() {
        ui.add_space(ui_kit::components::checkbox::ROW_GAP);
        if let Some(toggled) = nested_checkbox_row(ui, theme, flags, area, choices.areas[index]) {
            choices.areas[index] = toggled.is_checked();
        }
    }

    ui.add_space(spacing::SPACE_4);
    label_row(ui, theme, copy::CHECK_NOTIFICATIONS_LABEL);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(toggled) = Checkbox::new(copy::CHECK_DIGEST_LABEL)
        .checked(choices.weekly_digest)
        .ui(ui, theme, flags)
        .toggled
    {
        choices.weekly_digest = toggled.is_checked();
    }
    hint_row(ui, theme, copy::CHECK_DIGEST_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(toggled) = Checkbox::new(copy::CHECK_BUILDS_LABEL)
        .checked(choices.blocked_builds)
        .ui(ui, theme, flags)
        .toggled
    {
        choices.blocked_builds = toggled.is_checked();
    }
    hint_row(ui, theme, copy::CHECK_BUILDS_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    Checkbox::new(copy::CHECK_TEXTS_LABEL)
        .disabled()
        .ui(ui, theme, flags);
    hint_row(ui, theme, copy::CHECK_TEXTS_HINT);
}

/// The radio groups: the region, live against the same field the
/// select edits, and the invalid start group, the preview's error
/// specimen.
fn radios_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, form: &mut ProjectForm) {
    ui.label(copy::RADIOS_LABEL);
    ui.add_space(spacing::SPACE_2);

    let regions = radio_options(REGION_VALUES, copy::REGIONS);
    let region = RadioGroup::new(copy::SELECT_REGION_LABEL)
        .options(&regions)
        .selected(form.region)
        .hint(copy::SELECT_REGION_HINT)
        .ui(ui, theme, flags);
    if let Some(picked) = region.committed {
        form.region = Some(picked);
    }

    ui.add_space(spacing::SPACE_4);
    let starts = radio_options(START_VALUES, copy::START_OPTIONS);
    RadioGroup::new(copy::RADIO_START_LABEL)
        .options(&starts)
        .error(copy::RADIO_START_ERROR)
        .ui(ui, theme, flags);
}

/// The toggles: the spec's own example use, reduced motion, wired to
/// the gallery's flags so flipping it changes every component at once;
/// a local setting; and the disabled pair at both positions.
fn toggles_group(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &mut UiKitFlags,
    choices: &mut ChoiceDemos,
) {
    ui.label(copy::TOGGLES_LABEL);
    ui.add_space(spacing::SPACE_2);

    if let Some(on) = Toggle::new(copy::REDUCED_MOTION)
        .checked(flags.reduced_motion)
        .ui(ui, theme, flags)
        .toggled
    {
        // The switch that is its own example: the flag the theme menu
        // also sets, honoured across the kit.
        flags.reduced_motion = on;
    }
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    if let Some(on) = Toggle::new(copy::TOGGLE_FILTERS_LABEL)
        .checked(choices.remember_filters)
        .ui(ui, theme, flags)
        .toggled
    {
        choices.remember_filters = on;
    }
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    Toggle::new(copy::TOGGLE_SYNC_LABEL)
        .checked(true)
        .disabled()
        .ui(ui, theme, flags);
    hint_row(ui, theme, copy::TOGGLE_SYNC_HINT);
    ui.add_space(ui_kit::components::checkbox::ROW_GAP);
    Toggle::new(copy::TOGGLE_TIMESHEETS_LABEL)
        .checked(false)
        .disabled()
        .ui(ui, theme, flags);
    hint_row(ui, theme, copy::TOGGLE_TIMESHEETS_HINT);
}

/// The choice-controls section: the checkboxes, the radio groups and
/// the toggles, every control live in the active theme.
fn choice_controls_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &mut UiKitFlags,
    form: &mut ProjectForm,
    choices: &mut ChoiceDemos,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::CHOICE_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    checkboxes_group(ui, theme, flags, choices);
    ui.add_space(spacing::SPACE_4);
    radios_group(ui, theme, flags, form);
    ui.add_space(spacing::SPACE_4);
    toggles_group(ui, theme, flags, choices);

    ui.add_space(spacing::SPACE_4);
    ui.add(egui::Label::new(specimen_job(
        copy::CHOICE_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

// ---- the feedback components (issue 09) ------------------------------------

/// The banner demos: the four tones in the Banner preview's own copy,
/// the error's message carrying the four-part voice (what is wrong,
/// why it matters, what to do next, where to get help). Anchored at
/// the panel top, as the spec places them.
fn banners_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, demos: &mut BannerDemos) {
    let colors = theme.colors();
    ui.label(copy::BANNERS_LABEL);
    ui.add_space(spacing::SPACE_2);

    if demos.info {
        let response = Banner::new(
            BannerTone::Info,
            copy::BANNER_INFO_TITLE,
            copy::BANNER_INFO_MESSAGE,
        )
        .ui(ui, theme, flags);
        if response.event == Some(BannerEvent::Dismissed) {
            demos.info = false;
        }
    }
    if demos.success {
        let response = Banner::new(
            BannerTone::Success,
            copy::BANNER_SUCCESS_TITLE,
            copy::BANNER_SUCCESS_MESSAGE,
        )
        .ui(ui, theme, flags);
        if response.event == Some(BannerEvent::Dismissed) {
            demos.success = false;
        }
    }
    Banner::new(
        BannerTone::Warning,
        copy::BANNER_WARNING_TITLE,
        copy::BANNER_WARNING_MESSAGE,
    )
    .ui(ui, theme, flags);
    Banner::new(
        BannerTone::Error,
        copy::BANNER_ERROR_TITLE,
        copy::BANNER_ERROR_MESSAGE,
    )
    .ui(ui, theme, flags);

    if !demos.info || !demos.success {
        ui.add_space(spacing::SPACE_2);
        if Button::new(copy::BANNERS_RESTORE)
            .emphasis(ButtonEmphasis::Ghost)
            .ui(ui, theme, flags)
            .clicked
        {
            *demos = BannerDemos {
                info: true,
                success: true,
            };
        }
    }

    ui.add_space(spacing::SPACE_2);
    ui.add(egui::Label::new(specimen_job(
        copy::BANNERS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The snackbar triggers: buttons whose actions enqueue the host's
/// brief operational messages, one at a time, near the bottom of the
/// window.
fn snackbars_group(ui: &mut egui::Ui, theme: ThemeId, flags: &UiKitFlags, host: &mut SnackbarHost) {
    let colors = theme.colors();
    ui.label(copy::SNACKBARS_LABEL);
    ui.add_space(spacing::SPACE_2);
    ui.horizontal_wrapped(|ui| {
        if Button::new(copy::SNACK_SAVE_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(Snackbar::success(copy::SNACK_SAVED));
        }
        if Button::new(copy::SNACK_COPY_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(Snackbar::info(copy::SNACK_LINK_COPIED));
        }
        if Button::new(copy::SNACK_SUBMIT_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(Snackbar::info(copy::SNACK_SUBMITTED).action(copy::SNACK_VIEW_ACTION));
        }
        if Button::new(copy::SNACK_ARCHIVE_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(
                Snackbar::success(copy::SNACK_ARCHIVED_TITLE)
                    .title(copy::SNACK_ARCHIVED_TITLE)
                    .line(copy::SNACK_ARCHIVED_LINE)
                    .action(copy::SNACK_UNDO_ACTION),
            );
        }
        if Button::new(copy::SNACK_SYNC_TRIGGER)
            .ui(ui, theme, flags)
            .clicked
        {
            host.show(
                Snackbar::error(copy::SNACK_SYNC_FAILED_TITLE)
                    .title(copy::SNACK_SYNC_FAILED_TITLE)
                    .line(copy::SNACK_SYNC_FAILED_LINE)
                    .action(copy::SNACK_RETRY_ACTION),
            );
        }
    });
    ui.add(egui::Label::new(specimen_job(
        copy::SNACKBARS_NOTE,
        TypeStyle::Caption,
        theme,
        colors.text_muted(),
        Some(ui.available_width()),
    )));
}

/// The feedback section: the banners at the panel top, the snackbar
/// triggers below, the snackbar itself floating near the window's
/// bottom edge (painted by the host in `Gallery::ui`).
fn feedback_section(
    ui: &mut egui::Ui,
    theme: ThemeId,
    flags: &UiKitFlags,
    banners: &mut BannerDemos,
    snackbar: &mut SnackbarHost,
) {
    let colors = theme.colors();
    ui.add(TrackedLabel::new(
        copy::FEEDBACK_EYEBROW,
        TypeStyle::Eyebrow.font_id(theme),
        TypeStyle::Eyebrow.tracking(),
        colors.text_accent,
    ));
    ui.add_space(spacing::SPACE_4);

    banners_group(ui, theme, flags, banners);
    ui.add_space(spacing::SPACE_4);
    snackbars_group(ui, theme, flags, snackbar);
}

impl Gallery {
    /// The Brand panel (issue 10): the cover composition as the
    /// gallery's opening screen, then the Themes showcase and the
    /// Brand showcase, the brand book's three pages on one panel. The
    /// cover leads, so the panel needs no headline of its own.
    fn brand_panel(&mut self, ui: &mut egui::Ui) {
        Cover::new().ui(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        ThemesShowcase::new().ui(ui, self.theme, &self.flags);
        ui.add_space(spacing::SPACE_6);
        BrandShowcase::new().ui(ui, self.theme, &self.flags);
    }

    /// The window's chrome: the TopAppBar on Liquid Glass in a top
    /// panel, so it stays put while the page scrolls beneath (the
    /// web bar's sticky behaviour; the stylesheet's 96px
    /// scroll-padding-top reserve is the bar-and-clearance the panel
    /// provides here). The wordmark, the nav links, the search
    /// affordance, the theme menu and the one call to action.
    fn app_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, narrow: bool) {
        let theme = self.theme;
        let flags = self.flags;
        TopAppBar::new().ui(ui, theme, &flags, |ui| {
            ui.horizontal(|ui| {
                // The brand: the wordmark at its 24px chrome height,
                // in primary.
                ui.add(Logo::wordmark().height(LOGO_HEIGHT));
                ui.add_space(GROUP_GAP);

                // The primary nav: the panel links, the current one
                // carrying the indicator underline. Phone-width windows
                // skip them (the Tabs strip switches panels); the links
                // return the moment the window widens.
                if !narrow {
                    for (index, label) in copy::TABS.iter().enumerate() {
                        if NavLink::new(label, self.tab == index)
                            .ui(ui, theme, &flags)
                            .clicked()
                        {
                            self.tab = index;
                        }
                        ui.add_space(LINK_GAP);
                    }
                }

                // The actions, flush right: the one CTA (wide windows
                // only), the theme menu, the search affordance.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !narrow {
                        Button::new(copy::START_A_PROJECT)
                            .small()
                            .emphasis(ButtonEmphasis::Tertiary)
                            .ui(ui, theme, &flags);
                        ui.add_space(ACTIONS_GAP);
                    }

                    // The theme menu: all six themes reachable from
                    // the bar, the reduced flags beside them. The
                    // choice holds for the session (the Gallery's own
                    // field), and switching installs the theme's
                    // faces before applying it.
                    ui.menu_button(copy::THEME_MENU, |ui| {
                        let mut picked = theme;
                        for id in ThemeId::ALL {
                            ui.selectable_value(&mut picked, *id, id.name());
                        }
                        if picked != theme {
                            self.theme = picked;
                            install(picked, ctx);
                            picked.apply(ctx);
                        }
                        ui.separator();
                        ui.checkbox(&mut self.flags.reduced_motion, copy::REDUCED_MOTION);
                        ui.checkbox(
                            &mut self.flags.reduced_transparency,
                            copy::REDUCED_TRANSPARENCY,
                        );
                    });
                    ui.add_space(ACTIONS_GAP);

                    // The search affordance: the 44px icon button
                    // opening the reused TextField in its popover.
                    let search = IconButton::new(copy::SEARCH_BUTTON_NAME, Icon::Search)
                        .ui(ui, theme, &flags);
                    if search.clicked() {
                        self.search_open = !self.search_open;
                    }
                    let popup_id = search.id.with("search");
                    if self.search_open {
                        egui::Popup::open_id(ctx, popup_id);
                        egui::Popup::from_response(&search)
                            .id(popup_id)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .frame(popup_frame(theme))
                            .width(280.0)
                            .show(|ui| {
                                TextField::new(copy::SEARCH_FIELD_LABEL, &mut self.search)
                                    .placeholder(copy::SEARCH_PLACEHOLDER)
                                    .leading_icon(Icon::Search)
                                    .hint(copy::SEARCH_FIELD_HINT)
                                    .ui(ui, theme, &flags);
                            });
                    } else {
                        egui::Popup::close_id(ctx, popup_id);
                    }
                });
            });
        });
    }

    /// The Foundations panel: the type specimen, the interaction
    /// states, the foundation specimens, the icons, the logo and the
    /// colour-role swatches.
    fn foundations_panel(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Label::new(specimen_job(
            copy::FOUNDATIONS_HEADLINE,
            TypeStyle::Headline2,
            self.theme,
            self.theme.colors().text_display(),
            Some(ui.available_width()),
        )));
        ui.add_space(spacing::SPACE_4);

        type_section(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        states_section(ui, self.theme, &self.flags);
        ui.add_space(spacing::SPACE_6);
        foundations_section(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        icons_section(ui, self.theme);
        ui.add_space(spacing::SPACE_6);
        logo_section(ui, self.theme);

        ui.add_space(spacing::SPACE_6);
        ui.label(format!(
            "{} · {}",
            copy::COLOUR_ROLES_LABEL,
            self.theme.name()
        ));
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
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(radii::RADIUS_XS as u8),
                        color,
                    );
                    ui.label(sentence_case(role));
                    ui.end_row();
                }
            });
    }

    /// The Components panel: the actions, the surfaces, the text
    /// inputs and the choice controls.
    fn components_panel(&mut self, ui: &mut egui::Ui) {
        actions_section(
            ui,
            self.theme,
            &self.flags,
            &mut self.filters,
            &mut self.saved_chip,
        );
        ui.add_space(spacing::SPACE_6);
        components_section(ui, self.theme, &self.flags);
        ui.add_space(spacing::SPACE_6);
        text_inputs_section(ui, self.theme, &self.flags, &mut self.form);
        ui.add_space(spacing::SPACE_6);
        choice_controls_section(
            ui,
            self.theme,
            &mut self.flags,
            &mut self.form,
            &mut self.choices,
        );
    }

    /// The Feedback panel: the banner demos at the panel top, the
    /// snackbar triggers below.
    fn feedback_panel(&mut self, ui: &mut egui::Ui) {
        feedback_section(
            ui,
            self.theme,
            &self.flags,
            &mut self.banners,
            &mut self.snackbar,
        );
    }
}

impl Gallery {
    /// The window's chrome for one frame: the TopAppBar on Liquid Glass
    /// in a top panel, so it stays put while the page scrolls beneath
    /// (the web bar's sticky behaviour; the stylesheet's 96px
    /// scroll-padding-top reserve is the bar-and-clearance the panel
    /// provides here), the tabbed content panel, and the snackbar
    /// floating near the bottom.
    ///
    /// The root `Ui` eframe hands the app covers the whole viewport,
    /// safe areas included (egui's panels do not inset themselves —
    /// `rustnotes-core`'s scaffold does this for Rust Notes): the
    /// insets are read here and folded into the panels' outer margins,
    /// so the bar clears the status bar and Dynamic Island, the content
    /// clears the home indicator, and the snackbar rides above it. The
    /// viewport margins behind the insets are painted with the theme's
    /// canvas by `GalleryApp::ui`, so the fold is seamless.
    pub(crate) fn chrome(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        let insets = ctx.input(|i| i.safe_area_insets()).0;
        let chrome_margin = |top: f32, bottom: f32| egui::Margin {
            left: (spacing::SPACE_3 + insets.left) as i8,
            right: (spacing::SPACE_3 + insets.right) as i8,
            top: (spacing::SPACE_3 + top) as i8,
            bottom: (spacing::SPACE_3 + bottom) as i8,
        };
        // On a phone-width window the bar carries the brand, the theme
        // menu and the search affordance only: the nav links and the
        // call to action would not fit beside the wordmark, and the
        // Tabs strip below the bar already switches every panel.
        let narrow = root.available_width() < NARROW_WIDTH;
        // The sticky bar, floating on the canvas with its clearance.
        egui::Panel::top("vynatix-app-bar")
            .frame(egui::Frame::default().outer_margin(chrome_margin(insets.top, 0.0)))
            .show(root, |ui| self.app_bar(ui, &ctx, narrow));

        egui::CentralPanel::default()
            .frame(egui::Frame::default().outer_margin(egui::Margin {
                left: insets.left as i8,
                right: insets.right as i8,
                top: 0,
                bottom: insets.bottom as i8,
            }))
            .show(root, |ui| {
                egui::Frame::default()
                    .inner_margin(spacing::SPACE_6)
                    .show(ui, |ui| {
                        // The Tabs strip: the widget's own real demo, the
                        // gallery's panel switcher. It stays above the
                        // scroll, as navigation does.
                        let tabs = Tabs::new(copy::TABS).selected(self.tab).ui(
                            ui,
                            self.theme,
                            &self.flags,
                        );
                        if let Some(moved_to) = tabs.changed {
                            self.tab = moved_to;
                        }
                        ui.add(egui::Label::new(specimen_job(
                            copy::TABS_NOTE,
                            TypeStyle::Caption,
                            self.theme,
                            self.theme.colors().text_muted(),
                            Some(ui.available_width()),
                        )));

                        ui.add_space(spacing::SPACE_4);
                        // The default scroll source: touch drags arrive as
                        // Event::Scroll through the app's scroll shim
                        // (GalleryApp::filter_input), so the ScrollArea
                        // needs neither egui's drag-to-scroll (unreliable
                        // on the mobile backends) nor DragScroll::Always
                        // (which eats clicks in egui 0.36).
                        let scroll = if self.touch_scrolling {
                            egui::ScrollArea::vertical().vertical_scroll_offset(self.scroll_offset)
                        } else {
                            egui::ScrollArea::vertical()
                        };
                        let out = scroll.show(ui, |ui| {
                            match self.tab {
                                0 => self.brand_panel(ui),
                                1 => self.foundations_panel(ui),
                                2 => self.components_panel(ui),
                                _ => self.feedback_panel(ui),
                            }
                            ui.add_space(spacing::SPACE_6);
                        });
                        self.scroll_offset = out.state.offset.y;
                        self.touch_scrolling = false;
                        self.last_scroll_offset = out.state.offset;
                    });
            });

        // The snackbar host: the one visible message floats near the
        // window's bottom, centred, 24px above the edge. Its action
        // runs here, where the host knows what it enqueued.
        let action = self
            .snackbar
            .current()
            .and_then(|snackbar| snackbar.action.clone());
        if let Some(event) = self.snackbar.paint(&ctx, self.theme, &self.flags) {
            if event == SnackbarEvent::Action {
                match action.as_deref() {
                    Some(copy::SNACK_UNDO_ACTION) => {
                        self.snackbar
                            .show(Snackbar::success(copy::SNACK_RESULT_RESTORED));
                    }
                    Some(copy::SNACK_RETRY_ACTION) => {
                        self.snackbar
                            .show(Snackbar::success(copy::SNACK_RESULT_RUNNING));
                    }
                    Some(copy::SNACK_VIEW_ACTION) => {
                        self.snackbar
                            .show(Snackbar::info(copy::SNACK_RESULT_OPENING));
                    }
                    _ => {}
                }
            }
        }
    }
}
