//! Badge: the design system's state and kind pill, the looks of
//! `components/Badge` and the rules its preview carries.
//!
//! A badge names the state or kind of an item in one or two words on a
//! tinted pill: `radius-full`, 4px 10px padding, 12px text at weight
//! 600, a 6px gap beside a 16px leading icon (a 6px dot for the neutral
//! tone). The tones map onto the status roles: success
//! (`success-container` with `on-success-container` words and
//! `check_circle` in `success`), warning, info, error (`error-container`
//! and friends), neutral (`surface-container-high` with
//! `on-surface-variant` words and the dot, for quiet states such as
//! Draft) and accent (`tertiary-container` with `on-tertiary-container`
//! words, the surface's one tertiary accent). The icon shape and the
//! word carry the state, the tint backs them up; badges are not
//! controls, filters go through Chip.
//!
//! The count badge sits on an icon: [`IconButton`] is the preview's
//! 44px `.icon-btn`, resting transparent with an `on-surface` 24px
//! icon, filling `surface-container-high` on hover, the shared focus
//! ring at its own pill radius. Its badge-count is `error` with
//! `on-error` figures (Material 3's pairing, so it never competes with
//! the accent): 16px tall, 11px figures, a 2px ring in the surface it
//! sits on, and above 99 it shows 99+ ([`count_text`]). The dot variant
//! marks activity without a number. The button's accessibility label
//! carries the count the preview's way: "Notifications, 3 unread",
//! "Mail, more than 99 unread", "Messages, new activity".
//!
//! Documented deviations. The spec's tabular figures ship as the face's
//! own (egui exposes no numeric variant); the count's `font-sans` is
//! the theme's body face at weight 600, the same label voice the Button
//! sets; and `line-height: 16px` over the 12px words gives the 24px
//! pill the preview measures.

use egui::{Color32, CornerRadius, Response, Sense, Stroke, Ui, Vec2};

use crate::components::FocusRing;
use crate::icon::IconSize;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags, WidgetState};
use crate::theme::ThemeId;
use crate::tokens::type_scale;

/// The pill's vertical padding: 4px, the preview's own value.
pub const BADGE_PAD_Y: f32 = 4.0;
/// The pill's horizontal padding: 10px, the preview's own value.
pub const BADGE_PAD_X: f32 = 10.0;
/// The gap between the icon and the word: 6px, the preview's own value.
pub const BADGE_GAP: f32 = 6.0;
/// The words' line height: the preview's absolute 16px.
pub const BADGE_LINE_HEIGHT: f32 = 16.0;

/// The count badge's cap: above 99 it shows 99+.
pub const COUNT_CAP: u32 = 99;

/// The icon button's size: the preview's 44px, the button touch floor.
pub const ICON_BUTTON_SIZE: f32 = 44.0;

/// One badge tone, from the preview's six modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeTone {
    /// `badge--success`: `success-container`, `check_circle` in
    /// `success`.
    Success,
    /// `badge--warning`: `warning-container`, `warning` in `warning`.
    Warning,
    /// `badge--info`: `info-container`, `info` in `info`.
    Info,
    /// `badge--error`: `error-container`, `error` in `error`.
    Error,
    /// `badge--neutral`: `surface-container-high` with a 6px dot, for
    /// quiet states such as Draft.
    #[default]
    Neutral,
    /// `badge--accent`: `tertiary-container`. Counts as the surface's
    /// one tertiary accent.
    Accent,
}

impl BadgeTone {
    /// Every tone, preview order.
    pub const ALL: &[BadgeTone] = &[
        BadgeTone::Success,
        BadgeTone::Warning,
        BadgeTone::Info,
        BadgeTone::Error,
        BadgeTone::Neutral,
        BadgeTone::Accent,
    ];

    /// The classes the tone takes.
    pub const fn name(self) -> &'static str {
        match self {
            BadgeTone::Success => "badge badge--success",
            BadgeTone::Warning => "badge badge--warning",
            BadgeTone::Info => "badge badge--info",
            BadgeTone::Error => "badge badge--error",
            BadgeTone::Neutral => "badge badge--neutral",
            BadgeTone::Accent => "badge badge--accent",
        }
    }
}

/// The badge's resolved paints: pure data off the tokens, testable
/// without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeLook {
    /// The tinted fill.
    pub fill: Color32,
    /// The word colour.
    pub label: Color32,
    /// The leading 16px icon, when the tone carries one.
    pub icon: Option<Icon>,
    /// The icon's own colour (the tone's mark, one step stronger than
    /// the container's words).
    pub icon_color: Color32,
    /// The neutral tone's 6px dot instead of an icon.
    pub dot: bool,
}

/// Resolve one tone's paints, per theme.
pub fn badge_look(tone: BadgeTone, theme: ThemeId) -> BadgeLook {
    let colors = theme.colors();
    match tone {
        BadgeTone::Success => BadgeLook {
            fill: colors.success_container,
            label: colors.on_success_container,
            icon: Some(Icon::CheckCircle),
            icon_color: colors.success,
            dot: false,
        },
        BadgeTone::Warning => BadgeLook {
            fill: colors.warning_container,
            label: colors.on_warning_container,
            icon: Some(Icon::Warning),
            icon_color: colors.warning,
            dot: false,
        },
        BadgeTone::Info => BadgeLook {
            fill: colors.info_container,
            label: colors.on_info_container,
            icon: Some(Icon::Info),
            icon_color: colors.info,
            dot: false,
        },
        BadgeTone::Error => BadgeLook {
            fill: colors.error_container,
            label: colors.on_error_container,
            icon: Some(Icon::Error),
            icon_color: colors.error,
            dot: false,
        },
        BadgeTone::Neutral => BadgeLook {
            fill: colors.surface_container_high,
            label: colors.on_surface_variant,
            icon: None,
            icon_color: colors.on_surface_variant,
            dot: true,
        },
        BadgeTone::Accent => BadgeLook {
            fill: colors.tertiary_container,
            label: colors.on_tertiary_container,
            icon: None,
            icon_color: colors.on_tertiary_container,
            dot: false,
        },
    }
}

/// The pill's height: the 4px padding twice over the 16px line box.
pub const fn badge_height() -> f32 {
    2.0 * BADGE_PAD_Y + BADGE_LINE_HEIGHT
}

/// The pill's corner radius: `radius-full`, as the half-height.
pub fn badge_radius() -> CornerRadius {
    CornerRadius::same((badge_height() / 2.0) as u8)
}

/// The count badge's figures: the number itself through 99, 99+ above
/// (the spec: "Above 99 show 99+"). The preview's own aria-label spells
/// the capped case out as "more than 99 unread".
pub fn count_text(count: u32) -> String {
    if count > COUNT_CAP {
        format!("{COUNT_CAP}+")
    } else {
        count.to_string()
    }
}

/// The icon button's accessibility label, composed the preview's way:
/// "Notifications, 3 unread", "Mail, more than 99 unread" above the
/// cap, "Messages, new activity" for the dot.
pub fn count_accessibility_label(name: &str, count: Option<u32>) -> String {
    match count {
        Some(count) if count <= COUNT_CAP => format!("{name}, {count} unread"),
        Some(_) => format!("{name}, more than {COUNT_CAP} unread"),
        None => format!("{name}, new activity"),
    }
}

/// The count badge's minimum width for its figures: 16px square at
/// least, the figures plus 4px padding when they run wider.
pub fn count_min_width(figures_width: f32) -> f32 {
    (figures_width + 4.0).max(16.0)
}

/// The state or kind pill: one or two words on a tinted ground, a
/// static label (badges are not controls).
///
/// ```
/// use ui_kit::components::badge::{Badge, BadgeTone};
///
/// let shipped = Badge::new("Shipped", BadgeTone::Success);
/// let draft = Badge::new("Draft", BadgeTone::Neutral);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Badge<'a> {
    word: &'a str,
    tone: BadgeTone,
}

impl<'a> Badge<'a> {
    /// A badge in `tone`, reading `word`.
    pub const fn new(word: &'a str, tone: BadgeTone) -> Self {
        Self { word, tone }
    }

    /// The badge's tone.
    pub const fn tone(&self) -> BadgeTone {
        self.tone
    }

    /// Show the pill: laid out by its word, painted in the theme's
    /// 12px weight-600 label voice, hover-sensed only (it is not a
    /// control).
    pub fn ui(self, ui: &mut Ui, theme: ThemeId) -> Response {
        let look = badge_look(self.tone, theme);
        let font_id = egui::FontId {
            size: type_scale::TEXT_CAPTION,
            family: crate::typography::body_semibold_family(theme),
        };
        let galley = ui.ctx().fonts_mut(|fonts| {
            fonts.layout_delayed_color(self.word.to_owned(), font_id, f32::INFINITY)
        });
        let icon_px = IconSize::Sm.px();
        let leading = if look.dot || look.icon.is_some() {
            icon_px + BADGE_GAP
        } else {
            0.0
        };
        let width = 2.0 * BADGE_PAD_X + leading + galley.size().x;
        let height = badge_height();

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        painter.rect_filled(rect, badge_radius(), look.fill);

        let mut cursor = rect.left() + BADGE_PAD_X;
        if look.dot {
            painter.circle_filled(
                egui::Pos2::new(cursor + 3.0, rect.center().y),
                3.0,
                look.label,
            );
            cursor += icon_px + BADGE_GAP;
        } else if let Some(icon) = look.icon {
            painter.text(
                egui::Pos2::new(cursor + icon_px / 2.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                icon.as_char().to_string(),
                egui::FontId {
                    size: icon_px,
                    family: crate::icon::family_for_theme(theme, false),
                },
                look.icon_color,
            );
            cursor += icon_px + BADGE_GAP;
        }
        painter.galley(
            egui::Pos2::new(cursor, rect.center().y) - galley.size() / 2.0,
            galley,
            look.label,
        );

        response
    }
}

/// The preview's 44px `.icon-btn`: the count badge's host. Rests
/// transparent with an `on-surface` 24px icon, fills
/// `surface-container-high` on hover, takes the shared focus ring at
/// its own pill radius, and paints its count (or activity dot) at the
/// top right, ringed in the surface it sits on.
///
/// The name is the accessibility label the spec requires of an
/// icon-only control; the count rides it the preview's way.
///
/// ```
/// use ui_kit::components::badge::IconButton;
/// use ui_kit::icons::Icon;
///
/// let notifications = IconButton::new("Notifications", Icon::Notifications).count(3);
/// let activity = IconButton::new("Messages", Icon::Chat).activity_dot();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct IconButton<'a> {
    name: &'a str,
    icon: Icon,
    count: Option<u32>,
    dot: bool,
    enabled: bool,
}

impl<'a> IconButton<'a> {
    /// A 44px icon button named `name` for assistive technology.
    pub const fn new(name: &'a str, icon: Icon) -> Self {
        Self {
            name,
            icon,
            count: None,
            dot: false,
            enabled: true,
        }
    }

    /// Show the unread count beside the icon; above 99 it shows 99+.
    pub const fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Mark new activity without a number: the 8px dot.
    pub const fn activity_dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// Set enabled.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show the button and hand back its response.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> Response {
        let colors = theme.colors();
        let size = Vec2::splat(ICON_BUTTON_SIZE);
        let id = ui.next_auto_id().with("vynatix-icon-button");
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(
            rect,
            id,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let mut state = WidgetState::of(&response);
        if !self.enabled {
            state.disabled = true;
        }

        // The preview's own hover: the surface-container-high fill, with
        // the shared pressed overlay over it; disabled keeps the
        // transparent rest and drops the content to 38%.
        let fill = if state.disabled {
            Color32::TRANSPARENT
        } else if state.pressed {
            states::composite_over(
                states::pressed_overlay(colors.on_surface),
                colors.surface_container_high,
            )
        } else if state.hovered {
            colors.surface_container_high
        } else {
            Color32::TRANSPARENT
        };
        let icon_color = if state.disabled {
            states::disabled_content(colors.on_surface)
        } else {
            colors.on_surface
        };

        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        let radius = CornerRadius::same((size.x / 2.0) as u8);
        if fill != Color32::TRANSPARENT {
            painter.rect_filled(rect, radius, fill);
        }
        let icon_px = IconSize::Lg.px();
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.icon.as_char().to_string(),
            egui::FontId {
                size: icon_px,
                family: crate::icon::family_for_theme(theme, false),
            },
            icon_color,
        );

        // The count badge (or the activity dot) at the top right,
        // ringed in the surface it sits on.
        if let Some(count) = self.count {
            let figures = count_text(count);
            let font_id = egui::FontId {
                size: type_scale::TEXT_EYEBROW,
                family: crate::typography::body_semibold_family(theme),
            };
            let galley = ui.ctx().fonts_mut(|fonts| {
                fonts.layout_delayed_color(figures.clone(), font_id, f32::INFINITY)
            });
            let width = count_min_width(galley.size().x);
            let pill = egui::Rect::from_min_size(
                egui::Pos2::new(rect.left() + 22.0, rect.top() + 4.0),
                Vec2::new(width, 16.0),
            );
            painter.rect_stroke(
                pill,
                CornerRadius::same(8),
                Stroke::new(2.0, colors.surface),
                egui::StrokeKind::Outside,
            );
            painter.rect_filled(pill, CornerRadius::same(8), colors.error);
            painter.galley(
                egui::Pos2::new(pill.center().x, pill.center().y) - galley.size() / 2.0,
                galley,
                colors.on_error,
            );
        } else if self.dot {
            let center = egui::Pos2::new(rect.left() + 32.0, rect.top() + 14.0);
            painter.circle_filled(center, 4.0, colors.error);
            // The dot's own 2px surface ring, like the count's.
            painter.circle_stroke(center, 5.0, Stroke::new(2.0, colors.surface));
        }

        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                self.enabled,
                count_accessibility_label(self.name, self.count),
            )
        });

        FocusRing::new()
            .radius(radius)
            .paint(ui, &response, theme, flags);
        response
    }
}

#[cfg(test)]
mod tests {
    use crate::icons::Icon;
    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::type_scale;

    use super::{
        badge_height, badge_look, badge_radius, count_accessibility_label, count_min_width,
        count_text, Badge, BadgeTone, IconButton, BADGE_GAP, BADGE_LINE_HEIGHT, BADGE_PAD_X,
        BADGE_PAD_Y, COUNT_CAP,
    };

    /// Install the faces the badge paints with, the icon families and
    /// the theme's type faces, which bind at the pass boundary.
    fn install_faces(ctx: &egui::Context, theme: ThemeId) {
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
    }

    /// The count rule per the spec ("Above 99 show 99+"): the number
    /// itself through 99, 99+ from 100 on, whatever the magnitude.
    #[test]
    fn counts_show_the_number_through_99_then_cap() {
        assert_eq!(count_text(0), "0");
        assert_eq!(count_text(1), "1");
        assert_eq!(count_text(42), "42");
        assert_eq!(count_text(99), "99");
        assert_eq!(count_text(100), "99+");
        assert_eq!(count_text(1000), "99+");
        assert_eq!(count_text(u32::MAX), "99+");
        assert_eq!(COUNT_CAP, 99);
    }

    /// The accessibility labels compose the preview's way: the count
    /// spelled out under the cap, "more than 99 unread" above it, "new
    /// activity" for the dot.
    #[test]
    fn count_labels_compose_the_previews_way() {
        assert_eq!(
            count_accessibility_label("Notifications", Some(3)),
            "Notifications, 3 unread"
        );
        assert_eq!(
            count_accessibility_label("Mail", Some(99)),
            "Mail, 99 unread"
        );
        assert_eq!(
            count_accessibility_label("Mail", Some(120)),
            "Mail, more than 99 unread"
        );
        assert_eq!(
            count_accessibility_label("Messages", None),
            "Messages, new activity"
        );
    }

    /// The tone table per the Badge preview, in every theme: success,
    /// warning, info and error pair their containers with the tone's
    /// own mark colour and its icon; neutral fills
    /// surface-container-high with on-variant words and the dot, no
    /// icon; accent fills tertiary-container with no icon.
    #[test]
    fn tones_resolve_their_pairs_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let success = badge_look(BadgeTone::Success, *theme);
            assert_eq!(success.fill, colors.success_container, "{name}: fill");
            assert_eq!(success.label, colors.on_success_container, "{name}: words");
            assert_eq!(
                success.icon,
                Some(Icon::CheckCircle),
                "{name}: check_circle"
            );
            assert_eq!(
                success.icon_color, colors.success,
                "{name}: the mark colour"
            );
            assert!(!success.dot);

            let warning = badge_look(BadgeTone::Warning, *theme);
            assert_eq!(warning.fill, colors.warning_container);
            assert_eq!(warning.label, colors.on_warning_container);
            assert_eq!(warning.icon, Some(Icon::Warning));
            assert_eq!(warning.icon_color, colors.warning);

            let info = badge_look(BadgeTone::Info, *theme);
            assert_eq!(info.fill, colors.info_container);
            assert_eq!(info.label, colors.on_info_container);
            assert_eq!(info.icon, Some(Icon::Info));
            assert_eq!(info.icon_color, colors.info);

            let error = badge_look(BadgeTone::Error, *theme);
            assert_eq!(error.fill, colors.error_container);
            assert_eq!(error.label, colors.on_error_container);
            assert_eq!(error.icon, Some(Icon::Error));
            assert_eq!(error.icon_color, colors.error);

            let neutral = badge_look(BadgeTone::Neutral, *theme);
            assert_eq!(neutral.fill, colors.surface_container_high);
            assert_eq!(neutral.label, colors.on_surface_variant);
            assert_eq!(neutral.icon, None, "{name}: the dot stands in");
            assert!(neutral.dot, "{name}: the 6px dot");

            let accent = badge_look(BadgeTone::Accent, *theme);
            assert_eq!(accent.fill, colors.tertiary_container);
            assert_eq!(accent.label, colors.on_tertiary_container);
            assert_eq!(accent.icon, None);
            assert!(!accent.dot);
        }
    }

    /// The pill geometry: 4px 10px padding over the 16px line box (the
    /// 24px pill), the pill radius its half, the count 16px tall and
    /// square at least, the dot 8px across.
    #[test]
    fn geometry_matches_the_preview() {
        assert_eq!(BADGE_PAD_Y, 4.0);
        assert_eq!(BADGE_PAD_X, 10.0);
        assert_eq!(BADGE_GAP, 6.0);
        assert_eq!(BADGE_LINE_HEIGHT, 16.0);
        assert_eq!(type_scale::TEXT_CAPTION, 12.0);
        assert_eq!(badge_height(), 24.0);
        assert_eq!(badge_radius().nw, 12);
        assert_eq!(count_min_width(0.0), 16.0);
        assert_eq!(count_min_width(8.0), 16.0);
        assert_eq!(count_min_width(20.0), 24.0);
    }

    /// A badge lays out at the 24px pill height, its width the padding,
    /// the icon and the word; it senses no clicks (it is a label, not a
    /// control).
    #[test]
    fn badge_lays_out_as_a_24px_pill() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let mut heights = Vec::new();
        let mut clicked = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = Badge::new("Shipped", BadgeTone::Success).ui(ui, ThemeId::Light);
            heights.push(response.rect.height());
            clicked.push(response.clicked());
        })
        .drop_without_applying_deltas();
        assert_eq!(heights, vec![24.0]);
        assert_eq!(clicked, vec![false], "a badge is not a control");
    }

    /// The icon button is the preview's 44px square: it takes focus
    /// (Tab order), senses the click, and the count rides its
    /// accessibility label.
    #[test]
    fn icon_button_is_a_44px_focusable_control() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Dark);
        let flags = UiKitFlags::default();
        let mut id = None;
        let mut pos = None;
        let mut sizes = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = IconButton::new("Notifications", Icon::Notifications)
                .count(3)
                .ui(ui, ThemeId::Dark, &flags);
            id = Some(response.id);
            pos = Some(response.rect.center());
            sizes.push(response.rect.size());
        })
        .drop_without_applying_deltas();
        assert_eq!(sizes, vec![egui::Vec2::splat(44.0)], "the 44px square");

        ctx.memory_mut(|memory| memory.request_focus(id.unwrap()));
        let mut focused = false;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = IconButton::new("Notifications", Icon::Notifications)
                .count(3)
                .ui(ui, ThemeId::Dark, &flags);
            focused = response.has_focus();
        })
        .drop_without_applying_deltas();
        assert!(focused, "the button holds the requested focus");

        let mut clicked = false;
        for pressed in [true, false] {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::PointerMoved(pos.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: pos.unwrap(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let response = IconButton::new("Notifications", Icon::Notifications)
                    .count(3)
                    .ui(ui, ThemeId::Dark, &flags);
                clicked |= response.clicked();
            })
            .drop_without_applying_deltas();
        }
        assert!(clicked, "press then release fires");
    }
}
