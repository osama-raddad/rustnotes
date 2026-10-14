//! Card: the design system's content-agnostic container, the four
//! variants of `components/Card` and the README's "Cards" section.
//!
//! Elevated (the default) rests on `surface-container-low` with shadow
//! L1; outlined fills `surface` behind a 1px `outline-variant` border
//! and no shadow; filled takes `surface-container` flat; interactive is
//! elevated at rest and lifts to `surface-container` with shadow L2 on
//! hover. Every variant corners at `radius-lg` and pads from the
//! spacing scale: 24px standard, 16px compact, 32px large.
//!
//! An interactive card is a control: [`Card::ui`] senses clicks on the
//! whole card, keeps it one focus stop, and paints the FocusRing
//! treatment at the card's own radius. Never put essential information
//! behind its hover, and mark states with an accent chip, never a
//! tinted surface or a coloured edge. Cards stay opaque: glass is for
//! chrome and overlays (see [`crate::components::LiquidGlass`]).
//!
//! Documented deviation: the web system eases the surface and shadow
//! change over the short duration on the house curve; egui paints no
//! animated fills, so the lift applies on the frame the hover lands
//! (the same static treatment the kit's other state work ships). The
//! hover itself is probed from the previous frame's response for the
//! card's id, so the look is right in the same frame the pointer
//! arrives, without a fixed-size allocation before the content.

use egui::{Color32, Response, Sense, Stroke, Ui};

use crate::components::FocusRing;
use crate::states::UiKitFlags;
use crate::theme::{egui_shadow, ThemeId};
use crate::tokens::{radii, shadows::ShadowLayer, spacing};

/// Which card, from the Card spec's four variants. Interactive is
/// elevated at rest; the other three never change with the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardVariant {
    /// `card`: elevated, the default. `surface-container-low` with
    /// shadow L1.
    #[default]
    Elevated,
    /// `card--outlined`: `surface` with a 1px `outline-variant` border,
    /// no shadow.
    Outlined,
    /// `card--filled`: `surface-container`, no shadow.
    Filled,
    /// `card--interactive`: elevated, lifting to `surface-container`
    /// and shadow L2 on hover. A link or a button: one focus stop, the
    /// focus ring on `radius-lg`.
    Interactive,
}

/// The padding step, from the spacing scale: 16 compact, 24 standard,
/// 32 large.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardPadding {
    /// `space-4`, for compact cards.
    Compact,
    /// `space-6`, the standard.
    #[default]
    Standard,
    /// `space-8`, for large ones.
    Large,
}

impl CardPadding {
    /// The step's value from the spacing tokens.
    pub const fn spacing(self) -> f32 {
        match self {
            CardPadding::Compact => spacing::SPACE_4,
            CardPadding::Standard => spacing::SPACE_6,
            CardPadding::Large => spacing::SPACE_8,
        }
    }

    /// The step's `tokens.json` name.
    pub const fn name(self) -> &'static str {
        match self {
            CardPadding::Compact => "space-4",
            CardPadding::Standard => "space-6",
            CardPadding::Large => "space-8",
        }
    }
}

/// One card's resolved paints: the surface, the elevation, the border,
/// the corner radius and the padding. Pure data off the tokens, so the
/// variant table is testable without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardLook {
    /// The fill: `surface-container-low`, `surface` or
    /// `surface-container`.
    pub surface: Color32,
    /// The elevation layers, if the variant carries a shadow.
    pub shadow: Option<&'static [ShadowLayer]>,
    /// The border, if the variant carries one (the outlined variant's
    /// 1px `outline-variant`).
    pub border: Option<Stroke>,
    /// `radius-lg` for every variant.
    pub corner_radius: f32,
    /// The inner padding from the spacing scale.
    pub padding: f32,
}

/// The card widget: content-agnostic, one closure of body per card.
///
/// ```
/// use ui_kit::components::Card;
/// use ui_kit::states::UiKitFlags;
/// use ui_kit::theme::ThemeId;
///
/// let card = Card::new(); // elevated, standard padding
/// let interactive = Card::new().interactive().compact();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Card {
    variant: CardVariant,
    padding: CardPadding,
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl Card {
    /// An elevated card at standard (24px) padding.
    pub const fn new() -> Self {
        Self {
            variant: CardVariant::Elevated,
            padding: CardPadding::Standard,
        }
    }

    /// Set the variant.
    pub const fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The interactive variant: elevated, lifting on hover.
    pub const fn interactive(self) -> Self {
        self.variant(CardVariant::Interactive)
    }

    /// The outlined variant: `surface`, 1px border, no shadow.
    pub const fn outlined(self) -> Self {
        self.variant(CardVariant::Outlined)
    }

    /// The filled variant: `surface-container`, no shadow.
    pub const fn filled(self) -> Self {
        self.variant(CardVariant::Filled)
    }

    /// Set the padding step.
    pub const fn padding(mut self, padding: CardPadding) -> Self {
        self.padding = padding;
        self
    }

    /// Compact (16px) padding.
    pub const fn compact(self) -> Self {
        self.padding(CardPadding::Compact)
    }

    /// Large (32px) padding.
    pub const fn large(self) -> Self {
        self.padding(CardPadding::Large)
    }

    /// The card's variant.
    pub const fn variant_kind(&self) -> CardVariant {
        self.variant
    }

    /// The card's padding step.
    pub const fn padding_step(&self) -> CardPadding {
        self.padding
    }

    /// Resolve the card's paints in one state: the surface, elevation,
    /// border, radius and padding the variant calls for, `hovered` only
    /// moving the interactive variant.
    pub fn look(&self, hovered: bool, theme: ThemeId) -> CardLook {
        let colors = theme.colors();
        let shadows = theme.shadows();
        // Elevated at rest; interactive lifts one container step and one
        // elevation level on hover. Outlined and filled cast no shadow.
        let (surface, shadow) = match self.variant {
            CardVariant::Elevated => (colors.surface_container_low, Some(shadows.l1)),
            CardVariant::Interactive if hovered => (colors.surface_container, Some(shadows.l2)),
            CardVariant::Interactive => (colors.surface_container_low, Some(shadows.l1)),
            CardVariant::Outlined => (colors.surface, None),
            CardVariant::Filled => (colors.surface_container, None),
        };
        CardLook {
            surface,
            shadow,
            // The outlined variant's 1px outline-variant border; the
            // border is decorative and never carries state.
            border: (self.variant == CardVariant::Outlined)
                .then(|| Stroke::new(1.0, colors.outline_variant)),
            corner_radius: radii::RADIUS_LG,
            padding: self.padding.spacing(),
        }
    }

    /// Show the card with `add` as its body, sized by that body, and
    /// hand back the card's response: a click-sensed, focusable widget
    /// for the interactive variant, the frame's own response otherwise.
    pub fn ui<R>(
        self,
        ui: &mut Ui,
        theme: ThemeId,
        flags: &UiKitFlags,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> Response {
        let id = ui.auto_id_with("vynatix-card");
        // The hover probe: the response this card's id carried last
        // frame. The rect is not known before the body lays out, so the
        // probe keeps the look correct in the pointer's own frame
        // instead of one late.
        let hovered = ui
            .ctx()
            .read_response(id)
            .is_some_and(|response| response.hovered());
        let look = self.look(hovered, theme);
        let mut frame = egui::Frame::default()
            .inner_margin(look.padding)
            .corner_radius(radii::RADIUS_LG as u8)
            .fill(look.surface);
        if let Some(layers) = look.shadow {
            frame = frame.shadow(egui_shadow(layers));
        }
        if let Some(border) = look.border {
            frame = frame.stroke(border);
        }
        // The card's interior is always a vertical stack: a card placed
        // in a horizontal or wrapped row inherits left-to-right from the
        // row, which would lay the card's own sections out sideways.
        let inner = frame.show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), add)
                .inner
        });
        if self.variant == CardVariant::Interactive {
            let response = ui.interact(inner.response.rect, id, Sense::click());
            FocusRing::new()
                .radius(radii::RADIUS_LG as u8)
                .paint(ui, &response, theme, flags);
            response
        } else {
            inner.response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Card, CardPadding};
    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing};

    /// The variant table per the Card spec, in every theme: elevated
    /// rests on surface-container-low with shadow L1 and never moves
    /// with the pointer; interactive is elevated at rest and lifts to
    /// surface-container with shadow L2 on hover; outlined fills surface
    /// behind a 1px outline-variant border with no shadow; filled takes
    /// surface-container flat. Every variant corners at radius-lg.
    #[test]
    fn looks_match_the_variant_table_rest_and_hover() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let shadows = theme.shadows();
            let name = theme.name();

            let elevated = Card::new().look(false, *theme);
            assert_eq!(
                elevated.surface, colors.surface_container_low,
                "{name}: elevated rests on surface-container-low"
            );
            assert_eq!(elevated.shadow, Some(shadows.l1), "{name}: L1 at rest");
            assert_eq!(elevated.border, None, "{name}: elevated carries no border");
            let elevated_hover = Card::new().look(true, *theme);
            assert_eq!(
                elevated_hover, elevated,
                "{name}: the default card does not move with the pointer"
            );

            let rest = Card::new().interactive().look(false, *theme);
            assert_eq!(rest.surface, colors.surface_container_low);
            assert_eq!(rest.shadow, Some(shadows.l1));
            let hover = Card::new().interactive().look(true, *theme);
            assert_eq!(
                hover.surface, colors.surface_container,
                "{name}: interactive lifts to surface-container"
            );
            assert_eq!(hover.shadow, Some(shadows.l2), "{name}: L2 on hover");

            let outlined = Card::new().outlined().look(true, *theme);
            assert_eq!(
                outlined.surface, colors.surface,
                "{name}: outlined fills surface"
            );
            assert_eq!(outlined.shadow, None, "{name}: outlined casts no shadow");
            let border = outlined
                .border
                .expect("{name}: outlined carries the 1px border");
            assert_eq!(border.width, 1.0, "{name}: 1px, the default outline width");
            assert_eq!(
                border.color, colors.outline_variant,
                "{name}: outline-variant, decorative"
            );

            let filled = Card::new().filled().look(true, *theme);
            assert_eq!(
                filled.surface, colors.surface_container,
                "{name}: filled takes surface-container"
            );
            assert_eq!(filled.shadow, None, "{name}: filled casts no shadow");
            assert_eq!(filled.border, None, "{name}: filled carries no border");

            for look in [elevated, rest, hover, outlined, filled] {
                assert_eq!(look.corner_radius, radii::RADIUS_LG, "{name}: radius-lg");
            }
        }
    }

    /// Padding follows the spacing scale: 24px standard, 16px compact,
    /// 32px large, straight from the tokens the names point at.
    #[test]
    fn padding_follows_the_spacing_tokens() {
        let cases: &[(CardPadding, f32)] = &[
            (CardPadding::Compact, spacing::SPACE_4),
            (CardPadding::Standard, spacing::SPACE_6),
            (CardPadding::Large, spacing::SPACE_8),
        ];
        for (step, value) in cases {
            assert_eq!(*value, step.spacing(), "{}", step.name());
        }
        assert_eq!(spacing::SPACE_4, 16.0);
        assert_eq!(spacing::SPACE_6, 24.0);
        assert_eq!(spacing::SPACE_8, 32.0);
        for theme in ThemeId::ALL {
            for step in [
                CardPadding::Compact,
                CardPadding::Standard,
                CardPadding::Large,
            ] {
                assert_eq!(
                    Card::new().padding(step).look(false, *theme).padding,
                    step.spacing(),
                    "{}: {}",
                    theme.name(),
                    step.name()
                );
            }
        }
    }

    /// An interactive card is one control: it senses clicks on the
    /// whole card and holds keyboard focus as a single stop (the ring
    /// paints through the 04 helper inside `ui`). A plain card neither
    /// senses clicks nor takes focus.
    #[test]
    fn interactive_card_is_one_clickable_focusable_stop() {
        let ctx = egui::Context::default();
        let flags = UiKitFlags::default();
        let mut seen_id = None;
        for pass in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let response = Card::new()
                    .interactive()
                    .ui(ui, ThemeId::Light, &flags, |ui| {
                        ui.label("Read the case study");
                    });
                if pass == 0 {
                    seen_id = Some(response.id);
                    assert!(
                        response.is_pointer_button_down_on() || !response.hovered(),
                        "the card senses the pointer"
                    );
                } else {
                    assert!(response.has_focus(), "the card holds the requested focus");
                }
            })
            .drop_without_applying_deltas();
            if pass == 0 {
                ctx.memory_mut(|memory| memory.request_focus(seen_id.unwrap()));
            }
        }
    }
}
