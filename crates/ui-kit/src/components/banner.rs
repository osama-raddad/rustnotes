//! Banner: the design system's important-information strip, the
//! `.banner` rules the `components/Banner` preview carries and its
//! README.
//!
//! A banner carries important information at the top of a page or
//! section: a 20px leading icon, a short bold lead, the message and an
//! optional dismiss button. The four tones map onto the status roles —
//! info (`info-container` with `on-info-container` text and the `info`
//! icon in `info`), success, warning, and error (`error-container` and
//! friends) — at `radius-md` with 16px by 20px padding, 14px text at
//! line height 1.5 and a 12px gap between the icon and the text. The
//! icon and the words carry the state, never the colour alone.
//!
//! A banner stays until it is dismissed or the state changes, and the
//! dismiss button rides only the tones that resolve: info and success
//! by default, not warnings and errors that stay until they are
//! resolved ([`BannerTone::dismissible_by_default`]; the builder
//! overrides). Dismissing reports a typed event
//! ([`BannerEvent::Dismissed`]) so the host drops the banner from its
//! state. The dismiss control is a 32px `close` icon button in the
//! banner's text colour with the shared focus ring.
//!
//! Copy voice: lead with the outcome in bold, then say what happens
//! next. Error copy is reassuring and never falsely cheerful, four
//! parts — what is wrong, why it matters, what to do next, where to
//! get help.

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Ui, Vec2};

use crate::components::focus_ring::FocusRing;
use crate::icon::IconSize;
use crate::icons::Icon;
use crate::states::{self, UiKitFlags};
use crate::theme::ThemeId;
use crate::tokens::{leading, radii, spacing, type_scale};

/// The banner's vertical padding: 16px (`space-4`).
pub const BANNER_PAD_Y: f32 = spacing::SPACE_4;
/// The banner's horizontal padding: 20px (`space-5`).
pub const BANNER_PAD_X: f32 = spacing::SPACE_5;
/// The gap between the icon and the text: 12px (`space-3`).
pub const BANNER_GAP: f32 = spacing::SPACE_3;
/// The text size: `text-body-sm`.
pub const BANNER_TEXT_SIZE: f32 = type_scale::TEXT_BODY_SM;
/// The line height: `leading-normal` over the text size.
pub const BANNER_LINE_HEIGHT: f32 = BANNER_TEXT_SIZE * leading::LEADING_NORMAL;
/// The leading icon: 20px, the preview's `ms--md`.
pub const BANNER_ICON_PX: f32 = IconSize::Md.px();
/// The icon's 1px optical drop from the text's top.
pub const BANNER_ICON_TOP_OFFSET: f32 = 1.0;
/// The dismiss control: a 32px icon button.
pub const DISMISS_SIZE: f32 = 32.0;

/// One banner tone, the preview's four modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BannerTone {
    /// `banner--info`: `info-container`, the `info` icon in `info`.
    #[default]
    Info,
    /// `banner--success`: `success-container`, `check_circle` in
    /// `success`.
    Success,
    /// `banner--warning`: `warning-container`, the `warning` icon in
    /// `warning`. Stays until the state resolves.
    Warning,
    /// `banner--error`: `error-container`, the `error` icon in
    /// `error`. Stays until the state resolves.
    Error,
}

impl BannerTone {
    /// Every tone, preview order.
    pub const ALL: &[BannerTone] = &[
        BannerTone::Info,
        BannerTone::Success,
        BannerTone::Warning,
        BannerTone::Error,
    ];

    /// The classes the tone takes.
    pub const fn name(self) -> &'static str {
        match self {
            BannerTone::Info => "banner banner--info",
            BannerTone::Success => "banner banner--success",
            BannerTone::Warning => "banner banner--warning",
            BannerTone::Error => "banner banner--error",
        }
    }

    /// Whether the tone carries a dismiss button by default: info and
    /// success resolve, warnings and errors stay until they are
    /// resolved.
    pub const fn dismissible_by_default(self) -> bool {
        matches!(self, BannerTone::Info | BannerTone::Success)
    }
}

/// The banner's resolved paints: pure data off the tokens, testable
/// without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BannerLook {
    /// The container fill.
    pub fill: Color32,
    /// The text colour.
    pub label: Color32,
    /// The leading icon.
    pub icon: Icon,
    /// The icon's own colour, the tone's mark.
    pub icon_color: Color32,
}

/// Resolve one tone's paints, per theme.
pub fn banner_look(tone: BannerTone, theme: ThemeId) -> BannerLook {
    let colors = theme.colors();
    match tone {
        BannerTone::Info => BannerLook {
            fill: colors.info_container,
            label: colors.on_info_container,
            icon: Icon::Info,
            icon_color: colors.info,
        },
        BannerTone::Success => BannerLook {
            fill: colors.success_container,
            label: colors.on_success_container,
            icon: Icon::CheckCircle,
            icon_color: colors.success,
        },
        BannerTone::Warning => BannerLook {
            fill: colors.warning_container,
            label: colors.on_warning_container,
            icon: Icon::Warning,
            icon_color: colors.warning,
        },
        BannerTone::Error => BannerLook {
            fill: colors.error_container,
            label: colors.on_error_container,
            icon: Icon::Error,
            icon_color: colors.error,
        },
    }
}

/// The typed events a banner reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerEvent {
    /// The dismiss control fired; the host drops the banner.
    Dismissed,
}

/// The Banner widget: the toned container, its icon, the bold lead
/// and the message, and the dismiss control when the tone carries one.
///
/// ```
/// use ui_kit::components::banner::{Banner, BannerTone};
///
/// let saved = Banner::new(
///     BannerTone::Info,
///     "Saved automatically.",
///     "We kept your last six edits: refresh to see them.",
/// );
/// // if saved.ui(ui, theme, flags).event == Some(BannerEvent::Dismissed) {
/// //     show_saved_banner = false;
/// // }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Banner<'a> {
    tone: BannerTone,
    lead: &'a str,
    message: &'a str,
    dismissible: Option<bool>,
}

impl<'a> Banner<'a> {
    /// A banner of `tone` whose text opens with the bold `lead` and
    /// runs on into `message`.
    pub const fn new(tone: BannerTone, lead: &'a str, message: &'a str) -> Self {
        Self {
            tone,
            lead,
            message,
            dismissible: None,
        }
    }

    /// Override whether the dismiss control rides: set it true on a
    /// warning that resolves, false to pin an info banner.
    pub const fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = Some(dismissible);
        self
    }

    /// Show the banner in `ui` and hand back its typed response.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> BannerResponse {
        let look = banner_look(self.tone, theme);
        let dismissible = self
            .dismissible
            .unwrap_or_else(|| self.tone.dismissible_by_default());
        let width = ui.available_width();

        // The text: the lead in the body face at 600, the message in
        // the body face, one wrapping job across both.
        let reserved = BANNER_ICON_PX
            + BANNER_GAP
            + if dismissible {
                DISMISS_SIZE + BANNER_GAP
            } else {
                0.0
            };
        let wrap_width = (width - 2.0 * BANNER_PAD_X - reserved).max(0.0);
        let mut job = egui::text::LayoutJob::default();
        let lead_format = egui::TextFormat::simple(
            egui::FontId {
                size: BANNER_TEXT_SIZE,
                family: crate::typography::body_semibold_family(theme),
            },
            look.label,
        );
        let message_format = egui::TextFormat::simple(
            egui::FontId {
                size: BANNER_TEXT_SIZE,
                family: crate::typography::TypeStyle::Body.family(theme),
            },
            look.label,
        );
        job.append(self.lead, 0.0, lead_format);
        job.append(" ", 0.0, message_format.clone());
        job.append(self.message, 0.0, message_format);
        job.wrap.max_width = wrap_width;
        let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));

        let content_height = galley
            .size()
            .y
            .max(BANNER_LINE_HEIGHT)
            .max(BANNER_ICON_PX)
            .max(if dismissible { DISMISS_SIZE } else { 0.0 });
        let height = 2.0 * BANNER_PAD_Y + content_height;
        let id = ui.next_auto_id().with("vynatix-banner");
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        let radius = CornerRadius::same(radii::RADIUS_MD as u8);
        painter.rect_filled(rect, radius, look.fill);

        // The leading icon, dropped 1px from the text's top.
        painter.text(
            Pos2::new(
                rect.left() + BANNER_PAD_X + BANNER_ICON_PX / 2.0,
                rect.top() + BANNER_PAD_Y + BANNER_ICON_TOP_OFFSET + BANNER_ICON_PX / 2.0,
            ),
            egui::Align2::CENTER_CENTER,
            look.icon.as_char().to_string(),
            egui::FontId {
                size: BANNER_ICON_PX,
                family: crate::icon::family_for_theme(theme, false),
            },
            look.icon_color,
        );
        painter.galley(
            Pos2::new(
                rect.left() + BANNER_PAD_X + BANNER_ICON_PX + BANNER_GAP,
                rect.top() + BANNER_PAD_Y,
            ),
            galley,
            look.label,
        );

        // The dismiss control: a 32px circle in the banner's text
        // colour, the shared focus ring, and a typed event on click.
        let mut event = None;
        if dismissible {
            let dismiss_rect = Rect::from_min_size(
                Pos2::new(
                    rect.right() - BANNER_PAD_X - DISMISS_SIZE,
                    rect.center().y - DISMISS_SIZE / 2.0,
                ),
                Vec2::splat(DISMISS_SIZE),
            );
            let dismiss = ui.interact(dismiss_rect, id.with("dismiss"), Sense::click());
            let dismiss_state = states::WidgetState::of(&dismiss);
            let fill = if dismiss_state.pressed {
                states::composite_over(states::pressed_overlay(look.label), look.fill)
            } else if dismiss_state.hovered {
                states::composite_over(states::hover_overlay(look.label), look.fill)
            } else {
                look.fill
            };
            let dismiss_painter = ui.painter().clone().with_layer_id(dismiss.layer_id);
            dismiss_painter.rect_filled(
                dismiss_rect,
                CornerRadius::same((DISMISS_SIZE / 2.0) as u8),
                fill,
            );
            dismiss_painter.text(
                dismiss_rect.center(),
                egui::Align2::CENTER_CENTER,
                Icon::Close.as_char().to_string(),
                egui::FontId {
                    size: BANNER_ICON_PX,
                    family: crate::icon::family_for_theme(theme, false),
                },
                look.label,
            );
            FocusRing::new()
                .radius(radii::RADIUS_FULL as u8)
                .paint(ui, &dismiss, theme, flags);
            dismiss.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Dismiss")
            });
            if dismiss.clicked() {
                event = Some(BannerEvent::Dismissed);
            }
        }

        // The banner is a status region, not a control: the dismiss
        // carries the widget info, the text speaks for itself.
        BannerResponse { event, response }
    }
}

/// The banner's response: the wrapped egui response plus the typed
/// dismiss event.
#[derive(Clone, Debug)]
pub struct BannerResponse {
    /// The event this frame, `None` on an idle frame.
    pub event: Option<BannerEvent>,
    /// The banner's response.
    pub response: Response,
}

#[cfg(test)]
mod tests {
    use egui::RawInput;

    use crate::icons::Icon;
    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::{leading, radii, spacing, type_scale};

    use super::{
        banner_look, Banner, BannerEvent, BannerTone, BANNER_GAP, BANNER_ICON_PX,
        BANNER_LINE_HEIGHT, BANNER_PAD_X, BANNER_PAD_Y, BANNER_TEXT_SIZE, DISMISS_SIZE,
    };

    /// Install the faces the banner paints with.
    fn install_faces(ctx: &egui::Context, theme: ThemeId) {
        let out = ctx.run_ui(RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
    }

    /// The tone table per theme: each tone fills its status container,
    /// sets its text in the matching on-colour, and leads with its own
    /// icon in the tone's mark colour — the icon shape and the words
    /// carrying the state, never the colour alone.
    #[test]
    fn tone_table_maps_the_status_roles_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let info = banner_look(BannerTone::Info, *theme);
            assert_eq!(info.fill, colors.info_container, "{name}: info container");
            assert_eq!(info.label, colors.on_info_container, "{name}: on-info text");
            assert_eq!(info.icon, Icon::Info, "{name}: the info icon");
            assert_eq!(info.icon_color, colors.info, "{name}: info mark");

            let success = banner_look(BannerTone::Success, *theme);
            assert_eq!(success.fill, colors.success_container, "{name}: success");
            assert_eq!(
                success.label, colors.on_success_container,
                "{name}: on text"
            );
            assert_eq!(success.icon, Icon::CheckCircle, "{name}: check_circle");
            assert_eq!(success.icon_color, colors.success, "{name}: success mark");

            let warning = banner_look(BannerTone::Warning, *theme);
            assert_eq!(warning.fill, colors.warning_container, "{name}: warning");
            assert_eq!(
                warning.label, colors.on_warning_container,
                "{name}: on text"
            );
            assert_eq!(warning.icon, Icon::Warning, "{name}: the warning icon");
            assert_eq!(warning.icon_color, colors.warning, "{name}: warning mark");

            let error = banner_look(BannerTone::Error, *theme);
            assert_eq!(error.fill, colors.error_container, "{name}: error");
            assert_eq!(error.label, colors.on_error_container, "{name}: on text");
            assert_eq!(error.icon, Icon::Error, "{name}: the error icon");
            assert_eq!(error.icon_color, colors.error, "{name}: error mark");
        }
    }

    /// The preview's own numbers: 16px by 20px padding, a 12px gap, a
    /// 20px icon, 14px text at line height 1.5 (21px), `radius-md`,
    /// and the 32px dismiss button.
    #[test]
    fn banner_layout_constants_match_the_preview() {
        assert_eq!(BANNER_PAD_Y, 16.0, "space-4");
        assert_eq!(BANNER_PAD_X, 20.0, "space-5");
        assert_eq!(BANNER_GAP, 12.0, "space-3");
        assert_eq!(BANNER_ICON_PX, 20.0, "ms--md");
        assert_eq!(BANNER_TEXT_SIZE, type_scale::TEXT_BODY_SM);
        assert_eq!(BANNER_LINE_HEIGHT, 14.0 * leading::LEADING_NORMAL);
        assert_eq!(BANNER_LINE_HEIGHT, 21.0);
        assert_eq!(DISMISS_SIZE, 32.0);
        assert_eq!(radii::RADIUS_MD, 8.0);
        assert_eq!(spacing::SPACE_3, 12.0);
    }

    /// Dismiss rides info and success by default, and stays off
    /// warnings and errors, which resolve rather than close; the
    /// builder overrides both ways.
    #[test]
    fn dismiss_rides_info_and_success_by_default() {
        assert!(BannerTone::Info.dismissible_by_default());
        assert!(BannerTone::Success.dismissible_by_default());
        assert!(!BannerTone::Warning.dismissible_by_default());
        assert!(!BannerTone::Error.dismissible_by_default());
    }

    /// The widget round trip, headless: the dismiss click reports the
    /// typed `Dismissed` event, and a warning (no dismiss control)
    /// never reports one no matter where the click lands.
    #[test]
    fn dismiss_click_reports_the_typed_event() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let flags = UiKitFlags::default();
        let theme = ThemeId::Light;

        let mut dismiss_center = None;
        ctx.run_ui(RawInput::default(), |ui| {
            let response = Banner::new(
                BannerTone::Info,
                "Saved automatically.",
                "We kept your last six edits: refresh to see them.",
            )
            .ui(ui, theme, &flags);
            // The dismiss control: 32px, flush to the banner's right
            // padding, vertically centred.
            dismiss_center = Some(egui::Pos2::new(
                response.response.rect.right() - super::BANNER_PAD_X - super::DISMISS_SIZE / 2.0,
                response.response.rect.center().y,
            ));
        })
        .drop_without_applying_deltas();

        let mut events = Vec::new();
        for pressed in [true, false] {
            let mut input = RawInput::default();
            input
                .events
                .push(egui::Event::PointerMoved(dismiss_center.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: dismiss_center.unwrap(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let response = Banner::new(
                    BannerTone::Info,
                    "Saved automatically.",
                    "We kept your last six edits: refresh to see them.",
                )
                .ui(ui, theme, &flags);
                events.push(response.event);
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(
            events,
            vec![None, Some(BannerEvent::Dismissed)],
            "the release fires the typed dismiss"
        );

        // A warning carries no dismiss control: clicks on its body
        // report nothing.
        let mut warning_center = None;
        ctx.run_ui(RawInput::default(), |ui| {
            let response = Banner::new(
                BannerTone::Warning,
                "Your assignment ends in 14 days.",
                "Talk to your account lead about an extension.",
            )
            .ui(ui, theme, &flags);
            warning_center = Some(response.response.rect.center());
        })
        .drop_without_applying_deltas();
        let mut events = Vec::new();
        for pressed in [true, false] {
            let mut input = RawInput::default();
            input
                .events
                .push(egui::Event::PointerMoved(warning_center.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: warning_center.unwrap(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let response = Banner::new(
                    BannerTone::Warning,
                    "Your assignment ends in 14 days.",
                    "Talk to your account lead about an extension.",
                )
                .ui(ui, theme, &flags);
                events.push(response.event);
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(events, vec![None, None], "a pinned banner reports nothing");
    }

    /// The banner lays out at the spec's padding over its text and
    /// paints headless in every theme, the pinned and the dismissible
    /// tones both, without touching the output deltas.
    #[test]
    fn banner_paints_headless_in_every_theme() {
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            install_faces(&ctx, *theme);
            let flags = UiKitFlags::default();
            ctx.run_ui(RawInput::default(), |ui| {
                for tone in BannerTone::ALL {
                    let response = Banner::new(*tone, "Saved automatically.", "One line of body.")
                        .ui(ui, *theme, &flags);
                    let min = super::BANNER_PAD_Y * 2.0 + super::BANNER_LINE_HEIGHT;
                    assert!(
                        response.response.rect.height() >= min,
                        "{} {:?}: at least the padding over the line box",
                        theme.name(),
                        tone
                    );
                    assert!(response.response.rect.width() > 0.0);
                }
            })
            .drop_without_applying_deltas();
        }
    }
}
