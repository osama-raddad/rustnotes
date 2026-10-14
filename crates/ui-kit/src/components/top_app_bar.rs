//! TopAppBar: the page's chrome bar, the `.nav` rules the
//! `components/TopAppBar` preview carries and its README.
//!
//! The bar carries the logo, the primary navigation, search and one
//! call to action across the top of a page, on Liquid Glass: the
//! regular thickness over the surface tint (`glass-tint-surface` with
//! `--glass-blur-regular`), a 1px `md-sys-color-outline-variant`
//! border, `radius-lg` and 12px by 24px padding. In the dark themes
//! the tint is the deep surface, so the logo and links take their
//! dark-theme colours. Under [`UiKitFlags::reduced_transparency`] the
//! pane turns solid `surface-container`, the web system's swap.
//!
//! The logo is the wordmark at 24px tall in `primary`. Links are the
//! body face at 14px weight 500 in `text-body` without underline,
//! turning `primary` on hover; the current page's link carries the
//! same treatment Tabs give their active tab: `primary` with the 2px
//! `indicator` underline, 8px under the text. Icon buttons are the
//! 44px circles of [`crate::components::IconButton`], and the one call
//! to action is the small tertiary button ([`crate::components::Button`],
//! the surface's single accent moment — never two on the bar).
//!
//! Sticky behaviour: the web bar sticks to the top and relies on the
//! stylesheet's `scroll-padding-top: 96px` so focused elements never
//! hide behind it. The egui host paints the bar in a top panel
//! (`egui::Panel::top`) — it stays put while the central panel
//! scrolls — and [`SCROLL_RESERVE`] carries the 96px reserve's number
//! for hosts that scroll programmatically.
//!
//! Documented deviation: the web bar is a floating rounded pane inside
//! the page gutters; the kit paints the same rounded pane full-bleed
//! inside its panel, the glass tint compositing over whatever scrolls
//! beneath the panel's transparent frame (see the gallery).

use egui::{Color32, CornerRadius, Margin, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::components::focus_ring::FocusRing;
use crate::components::liquid_glass::{glass_recipe, GlassRecipe, GlassThickness, GlassTint};
use crate::states::UiKitFlags;
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale};

/// The bar's vertical padding: 12px (`space-3`).
pub const BAR_PAD_Y: f32 = spacing::SPACE_3;
/// The bar's horizontal padding: 24px (`space-6`).
pub const BAR_PAD_X: f32 = spacing::SPACE_6;
/// The gap between the bar's three groups: 24px, the `.nav` gap.
pub const GROUP_GAP: f32 = spacing::SPACE_6;
/// The gap between nav links: 28px, the `.nav ul` gap.
pub const LINK_GAP: f32 = 28.0;
/// The gap inside the actions group: 8px (`space-2`).
pub const ACTIONS_GAP: f32 = spacing::SPACE_2;
/// The wordmark's height in the bar: 24px, just over the 96px-wide
/// floor at the wordmark's aspect.
pub const LOGO_HEIGHT: f32 = spacing::SPACE_6;
/// The tallest bar item: the 44px icon buttons, which set the bar's
/// height.
pub const CONTENT_HEIGHT: f32 = 44.0;
/// The link text size: `text-body-sm`.
pub const LINK_TEXT_SIZE: f32 = type_scale::TEXT_BODY_SM;
/// The link weight: 500, like the rest tab.
pub const LINK_WEIGHT: u16 = 500;
/// The current page's underline: 8px under the text, per the
/// preview's `text-underline-offset`.
pub const LINK_UNDERLINE_OFFSET: f32 = 8.0;
/// The current page's underline thickness: the 2px `indicator` Tabs
/// use.
pub const LINK_UNDERLINE_HEIGHT: f32 = 2.0;
/// The sticky-header scroll reserve the stylesheet sets as
/// `scroll-padding-top` on the root.
pub const SCROLL_RESERVE: f32 = spacing::SPACE_24;

/// The bar's height for a content row `content` tall: the 12px paddings
/// both sides over the content.
pub fn bar_height(content_height: f32) -> f32 {
    BAR_PAD_Y * 2.0 + content_height
}

/// One nav link's resolved paint: pure data off the tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkLook {
    /// The label colour: `text-body` at rest, `primary` on hover and
    /// on the current page.
    pub label: Color32,
    /// Whether the 2px `indicator` underline rides under the link.
    pub underline: bool,
}

/// Resolve one link's paint, per theme: rest is `text-body`, hover
/// turns `primary`, and the current page's link is `primary` with the
/// 2px `indicator` underline (current outranks hover).
pub fn link_look(current: bool, hovered: bool, theme: ThemeId) -> LinkLook {
    let colors = theme.colors();
    if current {
        LinkLook {
            label: colors.primary,
            underline: true,
        }
    } else if hovered {
        LinkLook {
            label: colors.primary,
            underline: false,
        }
    } else {
        LinkLook {
            label: colors.text_body(),
            underline: false,
        }
    }
}

/// The current page's underline: 2px tall, as wide as the label, its
/// top edge 8px under the label's box (the preview's
/// `text-underline-offset: 8px`, measured off the text since egui
/// exposes no baseline).
pub fn link_underline_rect(label_rect: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(
            label_rect.left(),
            label_rect.bottom() + LINK_UNDERLINE_OFFSET,
        ),
        Vec2::new(label_rect.width(), LINK_UNDERLINE_HEIGHT),
    )
}

/// The bar's glass chrome: the regular thickness over the surface
/// tint, per the README's "Liquid Glass, Regular thickness".
pub fn bar_recipe(theme: ThemeId, flags: &UiKitFlags) -> GlassRecipe {
    glass_recipe(GlassThickness::Regular, GlassTint::Surface, theme, flags)
}

/// The bar's 1px `outline-variant` border.
pub fn bar_border(theme: ThemeId) -> Stroke {
    Stroke::new(1.0, theme.colors().outline_variant)
}

/// The TopAppBar chrome widget: the glass pane, its border and the
/// 12/24 padding, with the host's content (logo, nav, actions) laid
/// out inside.
///
/// ```
/// use ui_kit::components::top_app_bar::TopAppBar;
///
/// // TopAppBar::new().ui(ui, theme, flags, |ui| {
/// //     /* brand, nav links, search, the one CTA */
/// // });
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TopAppBar;

impl Default for TopAppBar {
    fn default() -> Self {
        Self::new()
    }
}

impl TopAppBar {
    /// An empty bar; everything inside arrives with the closure.
    pub const fn new() -> Self {
        Self
    }

    /// Show the bar in `ui` with `add` as its content row. The bar is
    /// chrome, not a control: the pane itself senses nothing, the
    /// links and buttons inside do their own sensing. The pane paints
    /// the regular surface glass with its 1px border and specular
    /// edge, holds the content at the 12/24 padding, and stands at
    /// least the 68px the 44px icon buttons make.
    pub fn ui<R>(
        self,
        ui: &mut Ui,
        theme: ThemeId,
        flags: &UiKitFlags,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> Response {
        let recipe = bar_recipe(theme, flags);
        let radius = CornerRadius::same(radii::RADIUS_LG as u8);
        let frame = egui::Frame::default()
            .inner_margin(Margin {
                left: BAR_PAD_X as i8,
                right: BAR_PAD_X as i8,
                top: BAR_PAD_Y as i8,
                bottom: BAR_PAD_Y as i8,
            })
            .corner_radius(radius)
            .fill(recipe.fill)
            .shadow(crate::theme::egui_shadow(recipe.shadow));
        let inner = frame.show(ui, |ui| {
            ui.set_min_height(CONTENT_HEIGHT);
            add(ui)
        });
        let rect = inner.response.rect;
        let painter = ui.painter().clone().with_layer_id(inner.response.layer_id);
        painter.rect_stroke(rect, radius, bar_border(theme), egui::StrokeKind::Inside);
        if let Some(color) = recipe.specular {
            let [from, to] = crate::components::liquid_glass::specular_line(rect, radii::RADIUS_LG);
            painter.line_segment([from, to], Stroke::new(1.0, color));
        }
        inner.response
    }
}

/// One nav link inside the bar: a 14px weight-500 label, primary on
/// hover, the 2px `indicator` underline when current, the shared focus
/// ring at `radius-xs` like the tab labels. Clicking reports through
/// the response.
pub struct NavLink<'a> {
    label: &'a str,
    current: bool,
}

impl<'a> NavLink<'a> {
    /// A link labelled `label`, current when it names the page shown.
    pub const fn new(label: &'a str, current: bool) -> Self {
        Self { label, current }
    }

    /// Show the link and hand back its response.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> Response {
        let colors = theme.colors();
        let font_id = egui::FontId {
            size: LINK_TEXT_SIZE,
            family: crate::typography::body_medium_family(theme),
        };
        let galley = ui.ctx().fonts_mut(|fonts| {
            fonts.layout_delayed_color(self.label.to_owned(), font_id, f32::INFINITY)
        });
        let size = Vec2::new(
            galley.size().x,
            LINK_TEXT_SIZE * crate::tokens::leading::LEADING_NORMAL,
        );
        let id = ui.next_auto_id().with("vynatix-nav-link");
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(rect, id, Sense::click());

        // The link's paint resolves off the live response: hover and
        // current arrive the frame they land.
        let look = link_look(self.current, response.hovered(), theme);
        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        painter.galley(
            Pos2::new(rect.left(), rect.center().y) - galley.size() / 2.0,
            galley,
            look.label,
        );
        if look.underline {
            painter.rect_filled(
                link_underline_rect(rect),
                CornerRadius::same(0),
                colors.indicator,
            );
        }
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, self.label));
        FocusRing::new()
            .radius(radii::RADIUS_XS as u8)
            .paint(ui, &response, theme, flags);
        response
    }
}

#[cfg(test)]
mod tests {
    use egui::{Color32, Pos2, RawInput, Rect, Stroke, Vec2};

    use crate::components::liquid_glass::{glass_recipe, GlassThickness, GlassTint};
    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing};

    use super::{
        bar_border, bar_height, bar_recipe, link_look, link_underline_rect, NavLink, TopAppBar,
        ACTIONS_GAP, BAR_PAD_X, BAR_PAD_Y, CONTENT_HEIGHT, GROUP_GAP, LINK_GAP, LINK_TEXT_SIZE,
        LINK_UNDERLINE_HEIGHT, LINK_UNDERLINE_OFFSET, LINK_WEIGHT, LOGO_HEIGHT, SCROLL_RESERVE,
    };

    /// The preview's own numbers: 12px by 24px padding, 24px between
    /// the bar's groups, 28px between links, 8px inside the actions,
    /// a 24px wordmark over 44px icon buttons, and the 68px bar that
    /// makes. The stylesheet's 96px scroll reserve rides along.
    #[test]
    fn bar_layout_constants_match_the_preview() {
        assert_eq!(BAR_PAD_Y, 12.0, "space-3");
        assert_eq!(BAR_PAD_X, 24.0, "space-6");
        assert_eq!(GROUP_GAP, 24.0, "the .nav gap");
        assert_eq!(LINK_GAP, 28.0, "the ul gap");
        assert_eq!(ACTIONS_GAP, 8.0, "space-2");
        assert_eq!(LOGO_HEIGHT, 24.0, "the wordmark's height");
        assert_eq!(CONTENT_HEIGHT, 44.0, "the icon buttons set the height");
        assert_eq!(bar_height(CONTENT_HEIGHT), 68.0, "12 + 44 + 12");
        assert_eq!(bar_height(52.0), 76.0, "taller content grows the bar");
        assert_eq!(SCROLL_RESERVE, 96.0, "scroll-padding-top");
        assert_eq!(SCROLL_RESERVE, spacing::SPACE_24);
    }

    /// The link looks per theme: rest `text-body` with no underline,
    /// hover `primary` with none, the current page `primary` with the
    /// 2px `indicator` underline, and current outranks hover.
    #[test]
    fn link_looks_follow_current_and_hover_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let rest = link_look(false, false, *theme);
            assert_eq!(rest.label, colors.text_body(), "{name}: text-body at rest");
            assert!(!rest.underline, "{name}: no underline at rest");

            let hover = link_look(false, true, *theme);
            assert_eq!(hover.label, colors.primary, "{name}: primary on hover");
            assert!(!hover.underline, "{name}: hover adds no underline");

            let current = link_look(true, false, *theme);
            assert_eq!(current.label, colors.primary, "{name}: primary current");
            assert!(current.underline, "{name}: the indicator underline");

            let current_hover = link_look(true, true, *theme);
            assert_eq!(current_hover, current, "{name}: current outranks hover");
        }
    }

    /// The current page's underline: 2px tall, as wide as the label,
    /// 8px under the label's box.
    #[test]
    fn link_underline_sits_eight_under_the_label() {
        let label = Rect::from_min_size(Pos2::new(10.0, 40.0), Vec2::new(50.0, 21.0));
        assert_eq!(
            link_underline_rect(label),
            Rect::from_min_size(Pos2::new(10.0, 69.0), Vec2::new(50.0, 2.0)),
            "8px under the label box, 2px tall"
        );
        assert_eq!(LINK_UNDERLINE_OFFSET, 8.0);
        assert_eq!(LINK_UNDERLINE_HEIGHT, 2.0);
        assert_eq!(LINK_TEXT_SIZE, 14.0);
        assert_eq!(LINK_WEIGHT, 500);
    }

    /// The bar's chrome: the regular thickness over the surface tint,
    /// exactly the README's glass recipe, plus the 1px
    /// `outline-variant` border at `radius-lg`; under reduced
    /// transparency the pane swaps to solid `surface-container`.
    #[test]
    fn bar_chrome_is_regular_surface_glass() {
        let flags = UiKitFlags::default();
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let recipe = bar_recipe(*theme, &flags);
            let expected =
                glass_recipe(GlassThickness::Regular, GlassTint::Surface, *theme, &flags);
            assert_eq!(recipe.fill, expected.fill, "{name}: the README's glass");
            assert_eq!(recipe.specular, expected.specular, "{name}: the edge");
            assert_eq!(recipe.shadow, expected.shadow, "{name}: the L2 elevation");
            assert_eq!(
                recipe.thickness,
                GlassThickness::Regular,
                "{name}: the regular thickness"
            );
            assert_eq!(recipe.fill, colors.glass_tint_surface, "{name}: the tint");

            let border = bar_border(*theme);
            assert_eq!(border.width, 1.0, "{name}: 1px");
            assert_eq!(
                border.color, colors.outline_variant,
                "{name}: outline-variant"
            );
        }
        assert_eq!(radii::RADIUS_LG, 12.0, "radius-lg");

        let solid_flags = UiKitFlags {
            reduced_transparency: true,
            ..Default::default()
        };
        let solid = bar_recipe(ThemeId::Dark, &solid_flags);
        assert_eq!(
            solid.fill,
            ThemeId::Dark.colors().surface_container,
            "the reduced-transparency swap"
        );
        assert_eq!(solid.specular, None, "no specular edge when solid");
    }

    /// The bar lays its content out at the spec's padding and reaches
    /// the 68px height with 44px content, painting headless without
    /// touching the output deltas, in every theme.
    #[test]
    fn bar_paints_headless_with_content() {
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            let out = ctx.run_ui(RawInput::default(), |ui| {
                crate::icon::install(ui.ctx());
                crate::typography::install(*theme, ui.ctx());
            });
            out.drop_without_applying_deltas();
            let flags = UiKitFlags::default();
            let mut heights = Vec::new();
            ctx.run_ui(RawInput::default(), |ui| {
                let response = TopAppBar::new().ui(ui, *theme, &flags, |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 44.0),
                        egui::Sense::hover(),
                    );
                    let _ = rect;
                });
                heights.push(response.rect.height());
            })
            .drop_without_applying_deltas();
            assert_eq!(heights, vec![68.0], "{}: 12 + 44 + 12", theme.name());
        }
    }

    /// The nav link paints headless and reports its rect through the
    /// row: a click lands on the response.
    #[test]
    fn nav_link_paints_and_reports_clicks() {
        let ctx = egui::Context::default();
        let out = ctx.run_ui(RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(ThemeId::Light, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();
        let mut center = None;
        ctx.run_ui(RawInput::default(), |ui| {
            let response = NavLink::new("Work", true).ui(ui, ThemeId::Light, &flags);
            center = Some(response.rect.center());
            assert!(response.rect.height() >= 21.0, "a text-sized target");
        })
        .drop_without_applying_deltas();

        for pressed in [true, false] {
            let mut input = RawInput::default();
            input
                .events
                .push(egui::Event::PointerMoved(center.unwrap()));
            input.events.push(egui::Event::PointerButton {
                pos: center.unwrap(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            let mut clicked = Vec::new();
            ctx.run_ui(input, |ui| {
                let response = NavLink::new("Work", true).ui(ui, ThemeId::Light, &flags);
                clicked.push(response.clicked());
            })
            .drop_without_applying_deltas();
            if !pressed {
                assert_eq!(clicked, vec![true], "the release fires the link");
            }
        }
    }

    /// The stroke the border carries: kept here so the widget and the
    /// tests agree on the shape of it.
    #[test]
    fn border_stroke_shape() {
        let border = bar_border(ThemeId::Light);
        assert_eq!(border, Stroke::new(1.0, Color32::from_rgb(200, 202, 190)));
    }
}
