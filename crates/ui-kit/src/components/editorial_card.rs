//! EditorialCard: a case study, field note or open role previewed as an
//! image, an eyebrow, a headline and a summary, per
//! `components/EditorialCard` and the preview's `.ec` rules.
//!
//! The container is `surface-container-low` at `radius-lg` with the
//! corners clipped by the content: the image area is a 4:3 placeholder
//! tile (`surface-container-highest` with the `image` icon in
//! `text-muted`) until approved documentary photography exists, per the
//! README's photography rules: a flat tint from the palette, clearly a
//! placeholder, never an invented illustration. The body carries the
//! standard eyebrow (the +0.24em signature, in `text-accent`), a
//! headline in the display voice at 24px weight 700 tracked −2% (the
//! bold display family, [`display_bold_family`]), and a 14px summary in
//! `text-muted` at the preview's 1.6 leading.
//!
//! A card with an [`action`](EditorialCard::action) is one link: the
//! whole card is a single click-sensed, focusable stop with the action
//! as its short name (the web pattern's stretched `<a>`), the focus
//! ring painted at the card's own radius, and the `north_east` icon
//! ending the eyebrow row. Without one it is static copy.
//!
//! [`display_bold_family`]: crate::typography::display_bold_family

use egui::epaint::CornerRadius;
use egui::{Color32, FontId, Response, Sense, Ui, Vec2};

use crate::components::FocusRing;
use crate::icon::{icon, IconSize};
use crate::icons::Icon;
use crate::states::UiKitFlags;
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, tracking, type_scale};
use crate::typography::{display_bold_family, tracked_text, TrackedLabel, TypeStyle};

/// The headline's tracking: the headline track, −2%.
pub const HEADLINE_TRACKING_EM: f32 = tracking::TRACKING_HEADLINE;
/// The headline's leading, from the preview's `.ec h3` rule.
pub const HEADLINE_LEADING: f32 = 1.18;
/// The summary's leading, from the preview's `.ec p` rule.
pub const SUMMARY_LEADING: f32 = 1.6;
/// The image area's aspect ratio: 4:3.
pub const IMAGE_ASPECT: (f32, f32) = (4.0, 3.0);
/// The card's default width, the preview's grid column.
pub const DEFAULT_WIDTH: f32 = 288.0;

/// The headline font: the display voice in bold at the type scale's h5
/// (24px, `--text-h5`).
pub fn headline_font_id(theme: ThemeId) -> FontId {
    FontId {
        size: type_scale::TEXT_H5,
        family: display_bold_family(theme),
    }
}

/// The summary font: the body face at the type scale's body-sm (14px,
/// `--text-body-sm`).
pub fn summary_font_id(theme: ThemeId) -> FontId {
    FontId {
        size: type_scale::TEXT_BODY_SM,
        family: TypeStyle::Body.family(theme),
    }
}

/// The placeholder tile's look: the flat tint, the icon and its colour
/// and size, clearly a placeholder.
pub struct ImagePlaceholder {
    /// `surface-container-highest`, the muted palette tint.
    pub fill: Color32,
    /// The `image` icon, saying "photography goes here".
    pub icon: Icon,
    /// The icon's colour: `text-muted`.
    pub icon_color: Color32,
    /// The icon's size: xl, 32px.
    pub icon_size: IconSize,
}

/// Resolve the placeholder tile from the theme.
pub fn placeholder(theme: ThemeId) -> ImagePlaceholder {
    let colors = theme.colors();
    ImagePlaceholder {
        fill: colors.surface_container_highest,
        icon: Icon::Image,
        icon_color: colors.text_muted(),
        icon_size: IconSize::Xl,
    }
}

/// The image area's height at `width`: 4:3.
pub fn image_height(width: f32) -> f32 {
    width * IMAGE_ASPECT.1 / IMAGE_ASPECT.0
}

/// The headline over the summary, the card's own component sizes
/// (24px over 14px). The system's 2.5x floor governs the display
/// specimen against body copy; this is a component headline, and the
/// display voice it borrows clears the floor through
/// [`display_pairing_holds`](crate::typography::display_pairing_holds).
pub fn headline_to_summary_ratio() -> f32 {
    type_scale::TEXT_H5 / type_scale::TEXT_BODY_SM
}

/// The editorial card widget.
///
/// ```
/// use ui_kit::components::EditorialCard;
///
/// let card = EditorialCard::new(
///     "Case study",
///     "Rebuilding the truck radio in sixteen weeks.",
///     "Three engineers, one designer, one product partner.",
/// )
/// .action("Read the case study");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EditorialCard<'a> {
    eyebrow: &'a str,
    headline: &'a str,
    summary: &'a str,
    action: Option<&'a str>,
    width: f32,
}

impl<'a> EditorialCard<'a> {
    /// The card: `eyebrow`, `headline` and `summary` around the
    /// placeholder image area, at the default width.
    pub const fn new(eyebrow: &'a str, headline: &'a str, summary: &'a str) -> Self {
        Self {
            eyebrow,
            headline,
            summary,
            action: None,
            width: DEFAULT_WIDTH,
        }
    }

    /// Make the card one link with `name` as its short name: a single
    /// click-sensed, focusable stop with the focus ring and the
    /// `north_east` icon ending the eyebrow row.
    pub const fn action(mut self, name: &'a str) -> Self {
        self.action = Some(name);
        self
    }

    /// Set the card's width (the image area's 4:3 follows it).
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Show the card.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> Response {
        let colors = theme.colors();
        let tile = placeholder(theme);
        let linked = self.action.is_some();
        let id = ui
            .auto_id_with("vynatix-editorial-card")
            .with(self.action.unwrap_or("static"));
        let radius = CornerRadius {
            nw: radii::RADIUS_LG as u8,
            ne: radii::RADIUS_LG as u8,
            sw: 0,
            se: 0,
        };
        let body_radius = CornerRadius {
            nw: 0,
            ne: 0,
            sw: radii::RADIUS_LG as u8,
            se: radii::RADIUS_LG as u8,
        };
        let width = self.width;
        let outer = ui.vertical(|ui| {
            ui.set_width(width);
            // The image area: 4:3, clipped to the card's top corners.
            let height = image_height(width);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
            ui.painter().rect_filled(rect, radius, tile.fill);
            // The placeholder's mark: the image icon centred in the
            // tile, painted straight to the painter (the `icon` widget
            // allocates at the cursor; the tile is already placed).
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tile.icon.as_char().to_string(),
                egui::FontId {
                    size: tile.icon_size.px(),
                    family: crate::icon::family_for_theme(theme, false),
                },
                tile.icon_color,
            );
            // The body: 24px of padding on the same fill, the card's
            // bottom corners.
            egui::Frame::default()
                .fill(colors.surface_container_low)
                .inner_margin(spacing::SPACE_6)
                .corner_radius(body_radius)
                .show(ui, |ui| {
                    // The body spans the card: the min width pins the
                    // frame to the full column, not its longest line.
                    ui.set_min_width(width - spacing::SPACE_6 * 2.0);
                    // The eyebrow row: the signature label, the link's
                    // north_east icon at the far end.
                    ui.horizontal(|ui| {
                        ui.add(TrackedLabel::new(
                            self.eyebrow,
                            TypeStyle::Eyebrow.font_id(theme),
                            TypeStyle::Eyebrow.tracking(),
                            colors.text_accent,
                        ));
                        if linked {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    icon(ui, Icon::NorthEast, IconSize::Md, colors.text_accent);
                                },
                            );
                        }
                    });
                    // The headline's 12px margins around it.
                    ui.add_space(spacing::SPACE_3);
                    ui.add(egui::Label::new(tracked_text(
                        self.headline,
                        headline_font_id(theme),
                        HEADLINE_TRACKING_EM,
                        Some(type_scale::TEXT_H5 * HEADLINE_LEADING),
                        colors.text_display(),
                    )));
                    ui.add_space(spacing::SPACE_3);
                    ui.add(egui::Label::new(tracked_text(
                        self.summary,
                        summary_font_id(theme),
                        0.0,
                        Some(type_scale::TEXT_BODY_SM * SUMMARY_LEADING),
                        colors.text_muted(),
                    )));
                });
        });
        if linked {
            let response = ui.interact(outer.response.rect, id, Sense::click());
            FocusRing::new()
                .radius(radii::RADIUS_LG as u8)
                .paint(ui, &response, theme, flags);
            response
        } else {
            outer.response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        headline_font_id, headline_to_summary_ratio, image_height, placeholder, summary_font_id,
        EditorialCard, ImagePlaceholder, HEADLINE_LEADING, HEADLINE_TRACKING_EM, IMAGE_ASPECT,
        SUMMARY_LEADING,
    };
    use crate::icon::IconSize;
    use crate::icons::Icon;
    use crate::theme::ThemeId;
    use crate::tokens::{tracking, type_scale};
    use crate::typography::{display_bold_family_name, display_pairing_holds, TypeStyle};
    use egui::FontFamily;

    /// The headline is the display voice in bold at 24px (`--text-h5`),
    /// tracked −2% at the preview's 1.18 leading.
    #[test]
    fn headline_is_the_bold_display_voice_at_24px() {
        assert_eq!(HEADLINE_TRACKING_EM, tracking::TRACKING_HEADLINE);
        assert_eq!(HEADLINE_TRACKING_EM, -0.02);
        assert_eq!(HEADLINE_LEADING, 1.18);
        for theme in ThemeId::ALL {
            let font_id = headline_font_id(*theme);
            assert_eq!(font_id.size, type_scale::TEXT_H5, "24px");
            assert_eq!(font_id.size, 24.0);
            assert_eq!(
                font_id.family,
                FontFamily::Name(display_bold_family_name(*theme).into()),
                "{}: the theme's bold display family",
                theme.name()
            );
        }
    }

    /// The summary is 14px (`--text-body-sm`) in the body face at the
    /// preview's 1.6 leading, and the eyebrow above it is the standard
    /// signature style, unmodified.
    #[test]
    fn summary_is_14px_body_and_the_eyebrow_stays_the_signature() {
        assert_eq!(SUMMARY_LEADING, 1.6);
        for theme in ThemeId::ALL {
            let font_id = summary_font_id(*theme);
            assert_eq!(font_id.size, type_scale::TEXT_BODY_SM);
            assert_eq!(font_id.size, 14.0);
            assert_eq!(
                font_id.family,
                TypeStyle::Body.family(*theme),
                "{}: the body face",
                theme.name()
            );
            // The eyebrow is the specimen's own style: 11px, tracked
            // +0.24em, the system sans.
            assert_eq!(TypeStyle::Eyebrow.size(), 11.0);
            assert_eq!(TypeStyle::Eyebrow.tracking(), 0.24);
            assert_eq!(
                TypeStyle::Eyebrow.family_name(*theme),
                "Geist SemiBold",
                "{}: the system sans",
                theme.name()
            );
        }
    }

    /// The image area is 4:3 and its placeholder is the muted palette
    /// tint with the image icon: flat, clearly a placeholder, no
    /// invented illustration.
    #[test]
    fn image_area_is_4_3_with_the_tinted_placeholder_tile() {
        assert_eq!(IMAGE_ASPECT, (4.0, 3.0));
        assert_eq!(image_height(288.0), 216.0);
        assert_eq!(image_height(240.0), 180.0);
        for theme in ThemeId::ALL {
            let ImagePlaceholder {
                fill,
                icon,
                icon_color,
                icon_size,
            } = placeholder(*theme);
            assert_eq!(
                fill,
                theme.colors().surface_container_highest,
                "{}: surface-container-highest",
                theme.name()
            );
            assert_eq!(icon, Icon::Image);
            assert_eq!(icon_color, theme.colors().text_muted());
            assert_eq!(icon_size, IconSize::Xl);
            assert_eq!(icon_size.px(), 32.0);
        }
    }

    /// The pairing floor: the card's own sizes are the component sizes
    /// the EditorialCard spec sets (24px over a 14px summary), while the
    /// display voice the headline borrows clears the system's 2.5x
    /// floor against body copy through the typography helper.
    #[test]
    fn display_voice_clears_the_pairing_floor_through_the_helper() {
        assert!((headline_to_summary_ratio() - 24.0 / 14.0).abs() < 1e-4);
        assert!(display_pairing_holds(), "the display specimen's own floor");
    }

    /// The card paints headless in every theme, linked and static,
    /// without touching the output deltas: the placeholder tile, the
    /// eyebrow row with its icon, the wrapping headline and the summary.
    /// The theme's faces and the icon families are installed first, as
    /// a host would.
    #[test]
    fn editorial_card_paints_headless() {
        use crate::states::UiKitFlags;
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            crate::typography::install(*theme, &ctx);
            crate::icon::install(&ctx);
            let flags = UiKitFlags::default();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                linked_card(*theme, &flags, ui);
                EditorialCard::new(
                    "Field notes",
                    "Premium isn't a price point. It's a discipline.",
                    "Why every Vynatix interface starts from a sixty-thirty-ten ratio.",
                )
                .ui(ui, *theme, &flags);
            })
            .drop_without_applying_deltas();
        }
    }

    /// The linked card takes keyboard focus as a single stop, so the
    /// focus ring (painted through the 04 helper inside `ui`) has a
    /// place to land.
    #[test]
    fn linked_editorial_card_is_one_focusable_stop() {
        use crate::states::UiKitFlags;
        let ctx = egui::Context::default();
        crate::typography::install(ThemeId::Light, &ctx);
        crate::icon::install(&ctx);
        let flags = UiKitFlags::default();
        let mut seen_id = None;
        for pass in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let response = linked_card(ThemeId::Light, &flags, ui);
                if pass == 0 {
                    seen_id = Some(response.id);
                } else {
                    assert!(response.has_focus(), "the linked card holds focus");
                }
            })
            .drop_without_applying_deltas();
            if pass == 0 {
                ctx.memory_mut(|memory| memory.request_focus(seen_id.unwrap()));
            }
        }
    }

    /// The shared linked card for the two headless passes above.
    fn linked_card(
        theme: ThemeId,
        flags: &crate::states::UiKitFlags,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        EditorialCard::new(
            "Case study",
            "Rebuilding the truck radio in sixteen weeks.",
            "Three engineers, one designer, one product partner.",
        )
        .action("Read the case study")
        .ui(ui, theme, flags)
    }
}
