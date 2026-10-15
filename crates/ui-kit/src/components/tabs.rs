//! Tabs: the design system's peer-panel switcher, the `.tabs` rules the
//! `components/Tabs` preview carries and its README.
//!
//! Tabs switch between peer panels of related content in one context;
//! the accent underline marks the active one. Labels are the body face
//! at 14px, weight 500, in `text-muted`; the active tab turns
//! `md-sys-color-primary` at weight 600 — the state never rests on
//! colour — and carries the 2px `indicator` underline over the row's
//! 1px `md-sys-color-outline-variant` rule, the surface's one accent
//! moment. Tabs sit 28px apart with 14px by 4px padding, and when they
//! outgrow the width the row scrolls sideways rather than wrapping.
//! Keyboard focus draws the shared 2px ring at `radius-xs` (the
//! preview's own focus outline), and the arrow keys move the selection
//! while any tab of the row holds it, wrapping.
//!
//! The ring reaches [`FOCUS_RING_REACH`] past a tab, so the row paints
//! it under a clip widened by that much at each side (the first tab
//! stands at the strip's left edge, the last at its right once scrolled
//! to the end); the host needs to leave that room beside the row, as the
//! gallery's 16px gutter does.
//!
//! Documented deviation: the web system keeps only the selected tab in
//! the tab order (`tabindex="-1"` on the rest). egui's Tab traversal
//! walks every interactive widget, so the kit cannot take three of four
//! tabs out of it; instead the arrow keys act everywhere the row holds
//! focus and the selection follows, which is the behaviour the spec
//! gives the arrows. And the rest label's weight 500: only Monochrome
//! ships a medium instance of the body face, so Forest and Teal rest at
//! the regular 400 ([`crate::typography::body_medium_family`] resolves
//! the nearest one), the active weight 600 being the face every theme
//! installs.
//!
//! The gallery is the component's real demo: its own panel switcher is
//! a [`Tabs`] row (see the `ui-kit-gallery` crate).

use egui::scroll_area::{DragScroll, ScrollSource};
use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Ui, Vec2};

use crate::components::focus_ring::FocusRing;
use crate::states::{UiKitFlags, FOCUS_RING_REACH};
use crate::theme::ThemeId;
use crate::tokens::{radii, type_scale};

/// The gap between tabs: 28px, the preview's own `margin-right`.
pub const TAB_GAP: f32 = 28.0;
/// One tab's vertical padding: 14px.
pub const TAB_PAD_Y: f32 = 14.0;
/// One tab's horizontal padding: 4px.
pub const TAB_PAD_X: f32 = 4.0;
/// The label size: `text-body-sm`.
pub const TAB_TEXT_SIZE: f32 = type_scale::TEXT_BODY_SM;
/// The rest label's weight.
pub const TAB_REST_WEIGHT: u16 = 500;
/// The active label's weight.
pub const TAB_ACTIVE_WEIGHT: u16 = 600;
/// The active tab's underline: 2px of `indicator` over the row's rule.
pub const INDICATOR_HEIGHT: f32 = 2.0;
/// The row's own bottom rule: 1px `outline-variant`.
pub const RULE_HEIGHT: f32 = 1.0;

/// One tab's resolved paint: pure data off the tokens, testable
/// without a context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabLook {
    /// The label colour: `text-muted` at rest, `primary` active.
    pub label: Color32,
    /// The label weight: 500 at rest, 600 active.
    pub weight: u16,
    /// Whether the 2px `indicator` underline rides under the tab.
    pub indicator: bool,
}

/// Resolve one tab's paint, per theme: rest is `text-muted` at the
/// body face's medium stand-in, active is `primary` at 600 with the
/// `indicator` underline on. The weight maps to the theme's faces
/// through [`crate::typography::body_medium_family`] and
/// [`crate::typography::body_semibold_family`].
pub fn tab_look(active: bool, theme: ThemeId) -> TabLook {
    let colors = theme.colors();
    if active {
        TabLook {
            label: colors.primary,
            weight: TAB_ACTIVE_WEIGHT,
            indicator: true,
        }
    } else {
        TabLook {
            label: colors.text_muted(),
            weight: TAB_REST_WEIGHT,
            indicator: false,
        }
    }
}

/// Clamp a host's active index into `len` labels: an index past the
/// end lands on the last label, and an empty row reports 0.
pub fn clamp_index(active: usize, len: usize) -> usize {
    active.min(len.saturating_sub(1))
}

/// The active tab's 2px underline: the bottom strip of the tab's own
/// rect, which the row lays flush with its bottom so the underline
/// sits over the row's 1px rule (the preview's `-1px` bottom margin).
pub fn indicator_rect(tab_rect: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(tab_rect.left(), tab_rect.bottom() - INDICATOR_HEIGHT),
        Vec2::new(tab_rect.width(), INDICATOR_HEIGHT),
    )
}

/// The row's 1px `outline-variant` bottom rule.
pub fn rule_rect(row_rect: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(row_rect.left(), row_rect.bottom() - RULE_HEIGHT),
        Vec2::new(row_rect.width(), RULE_HEIGHT),
    )
}

/// The Tabs widget: one row of peer-panel labels, clicking or arrowing
/// moves the selection, the host owns the index as typed state.
///
/// ```
/// use ui_kit::components::tabs::Tabs;
///
/// let tabs = ["Overview", "Engagements", "Team", "Press"];
/// // let response = Tabs::new(&tabs).selected(0).ui(ui, theme, flags);
/// // if let Some(moved_to) = response.changed {
/// //     active = moved_to;
/// // }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Tabs<'a> {
    labels: &'a [&'a str],
    selected: usize,
    /// When a drag scrolls the strip; egui's own default unless the host
    /// says otherwise.
    drag_to_scroll: DragScroll,
}

impl<'a> Tabs<'a> {
    /// A row of `labels` with nothing selected but the first.
    pub const fn new(labels: &'a [&'a str]) -> Self {
        Self {
            labels,
            selected: 0,
            drag_to_scroll: DragScroll::OnTouch,
        }
    }

    /// Set the active index; out-of-range values clamp at draw time.
    pub const fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    /// Set when a drag scrolls the strip once it outgrows the width,
    /// the strip's counterpart of [`egui::Window::drag_to_scroll`]. The
    /// default is egui's own, [`DragScroll::OnTouch`]: a drag scrolls
    /// only once egui has seen a touch, so on a mouse-only desktop a
    /// drag never scrolls; once the screen has been touched, egui arms
    /// it for every pointer. A touch-first host passes
    /// [`DragScroll::Always`], as for its own scroll areas: egui binds a
    /// press to its drag target against the previous frame, so under
    /// `OnTouch` the first swipe after launch finds nothing to drag and
    /// lands on nothing. The kit carries no platform switch of its own;
    /// the host knows its screen.
    pub const fn drag_to_scroll(mut self, drag_to_scroll: DragScroll) -> Self {
        self.drag_to_scroll = drag_to_scroll;
        self
    }

    /// Show the row in `ui`: the labels, the active one's underline,
    /// the row's rule, the shared focus ring on the focused tab, and
    /// the arrow-key navigation. Hands back the clamped selection and
    /// the index it moved to this frame, if it moved.
    pub fn ui(self, ui: &mut Ui, theme: ThemeId, flags: &UiKitFlags) -> TabsResponse {
        let colors = theme.colors();
        let len = self.labels.len();
        if len == 0 {
            // An empty row allocates nothing and reports a hollow
            // response: the host's index reads back as 0.
            let (_, response) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
            return TabsResponse {
                selected: 0,
                changed: None,
                response,
            };
        }
        let selected = clamp_index(self.selected, len);

        // The row's ids live under one base so the focus check and the
        // host can read any tab's response back.
        let ctx = ui.ctx().clone();
        let id = ui.next_auto_id().with("vynatix-tabs");

        // The arrows move the selection while any tab of this row
        // holds the focus, wrapping (egui binds focus at the pass
        // boundary, so last frame's responses carry it).
        let focused_here = (0..len).any(|index| {
            ctx.read_response(id.with(index))
                .is_some_and(|response| response.has_focus())
        });
        let (right, left) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowRight),
                input.key_pressed(egui::Key::ArrowLeft),
            )
        });
        let mut changed = if focused_here && (right || left) {
            let delta = if right { 1 } else { -1 };
            Some(((selected as i32 + delta).rem_euclid(len as i32)) as usize)
        } else {
            None
        };
        if let Some(to) = changed {
            // The selection follows the arrows and so does the focus,
            // explicitly: egui's own arrow traversal moves the system
            // focus before the widgets run, and the request wins.
            ctx.memory_mut(|memory| memory.request_focus(id.with(to)));
        }
        // The visuals move this frame, ahead of the host's state copy.
        let showing = changed.unwrap_or(selected);

        // The row: full-width viewport so a row that outgrows the
        // width scrolls sideways rather than wrapping, the tabs held
        // 28px apart at the label's line box plus their 14px paddings.
        // The viewport carries its own id salt: an unsalted scroll area
        // takes its id from its `Ui` alone, so an unsalted neighbour in
        // the same `Ui` (a host's page area right below the strip)
        // would share the strip's scroll state, and the strip, which
        // scrolls sideways only, would clamp that page's vertical
        // offset back to zero every frame.
        let saved_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::new(TAB_GAP, 0.0);
        let line_height = TAB_TEXT_SIZE * crate::tokens::leading::LEADING_NORMAL;
        let tab_height = 2.0 * TAB_PAD_Y + line_height;
        let mut active_rect: Option<Rect> = None;
        let mut row: Option<Rect> = None;
        // The clip the host gave the row, before the scroll area narrows
        // it to the strip.
        let host_clip = ui.clip_rect();
        let scroll = egui::ScrollArea::horizontal()
            .id_salt(id.with("strip"))
            .scroll_source(ScrollSource {
                drag: self.drag_to_scroll,
                ..ScrollSource::default()
            })
            .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                // The row runs left to right inside the scroll
                // viewport; the scroll area's own content layout
                // starts vertical.
                ui.horizontal(|ui| {
                    for (index, label) in self.labels.iter().enumerate() {
                        let active = index == showing;
                        let look = tab_look(active, theme);
                        let family = if active {
                            crate::typography::body_semibold_family(theme)
                        } else {
                            crate::typography::body_medium_family(theme)
                        };
                        let galley = ui.ctx().fonts_mut(|fonts| {
                            fonts.layout_delayed_color(
                                (*label).to_owned(),
                                egui::FontId {
                                    size: TAB_TEXT_SIZE,
                                    family,
                                },
                                f32::INFINITY,
                            )
                        });
                        let width = galley.size().x + 2.0 * TAB_PAD_X;
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(width, tab_height), Sense::hover());
                        row = Some(row.map_or(rect, |so_far| so_far.union(rect)));
                        let response = ui.interact(rect, id.with(index), Sense::click());
                        if response.clicked() {
                            // A click moves the selection and lands the
                            // focus, the native tab behaviour egui only
                            // gives Tab traversal.
                            changed = Some(index);
                            ctx.memory_mut(|memory| memory.request_focus(id.with(index)));
                        }

                        let painter = ui.painter().clone().with_layer_id(response.layer_id);
                        painter.galley(
                            crate::typography::left_center(
                                rect.left() + TAB_PAD_X,
                                rect.center().y,
                                galley.size(),
                            ),
                            galley,
                            look.label,
                        );
                        if active {
                            active_rect = Some(rect);
                        }
                        // The scroll area clips on its scroll axis, x, so
                        // it cuts the ring at the first tab and, once
                        // scrolled to the end, at the last. Paint the
                        // ring under a clip widened along x by the ring's
                        // reach. The other axis is not clipped by the
                        // scroll area, and the widened clip stays inside
                        // the host's, so the ring cannot paint beyond
                        // what the host allows.
                        let strip_clip = ui.clip_rect();
                        ui.set_clip_rect(
                            strip_clip
                                .expand2(Vec2::new(FOCUS_RING_REACH, 0.0))
                                .intersect(host_clip),
                        );
                        FocusRing::new()
                            .radius(radii::RADIUS_XS as u8)
                            .paint(ui, &response, theme, flags);
                        ui.set_clip_rect(strip_clip);
                    }
                });
            });

        // The row's 1px rule across the tabs, with the active tab's
        // 2px indicator over it (painted after so the accent mark
        // covers the hairline, the preview's `-1px` overlap).
        let row_rect = row.unwrap_or(scroll.inner_rect);
        // The rule runs the strip's whole visible width, edge to edge,
        // not just under the tabs' union: a rule that stops short of the
        // content's edge reads as a stray margin.
        let strip = Rect::from_x_y_ranges(scroll.inner_rect.x_range(), row_rect.y_range());
        ui.painter().rect_filled(
            rule_rect(strip),
            CornerRadius::same(0),
            colors.outline_variant,
        );
        if let Some(tab_rect) = active_rect {
            ui.painter().rect_filled(
                indicator_rect(tab_rect),
                CornerRadius::same(0),
                colors.indicator,
            );
        }

        ui.spacing_mut().item_spacing = saved_spacing;

        // The row's own response: the tabs' union, hover-sensed, so
        // hosts can read the row's geometry back.
        let response = ui.interact(row_rect, id.with("row"), Sense::hover());

        TabsResponse {
            selected: changed.unwrap_or(selected),
            changed,
            response,
        }
    }
}

/// The tabs' response: the row's egui response plus the typed reads.
#[derive(Clone, Debug)]
pub struct TabsResponse {
    /// The clamped active index after this frame.
    pub selected: usize,
    /// The index the selection moved to this frame, by click or arrow.
    pub changed: Option<usize>,
    /// The row's response.
    pub response: Response,
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, RawInput, Rect, Vec2};

    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing, type_scale};
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        clamp_index, indicator_rect, rule_rect, tab_look, Tabs, INDICATOR_HEIGHT, RULE_HEIGHT,
        TAB_ACTIVE_WEIGHT, TAB_GAP, TAB_PAD_X, TAB_PAD_Y, TAB_REST_WEIGHT, TAB_TEXT_SIZE,
    };

    /// Install the faces the tabs paint with.
    fn install_faces(ctx: &egui::Context, theme: ThemeId) {
        let out = ctx.run_ui(RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
    }

    /// The preview's own numbers: labels at `text-body-sm` (14px), tabs
    /// 28px apart with 14px by 4px padding, the 2px indicator over the
    /// row's 1px `outline-variant` rule.
    #[test]
    fn layout_constants_match_the_preview() {
        assert_eq!(TAB_TEXT_SIZE, type_scale::TEXT_BODY_SM);
        assert_eq!(TAB_TEXT_SIZE, 14.0);
        assert_eq!(TAB_GAP, 28.0);
        assert_eq!(TAB_PAD_Y, 14.0);
        assert_eq!(TAB_PAD_X, 4.0);
        assert_eq!(INDICATOR_HEIGHT, 2.0);
        assert_eq!(RULE_HEIGHT, 1.0);
    }

    /// Rest tabs are `text-muted` at weight 500 in the body face's
    /// medium stand-in; the active tab is `primary` at 600 with the
    /// indicator on. The state never rests on colour: weight and
    /// underline carry it too.
    #[test]
    fn looks_follow_the_active_state_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let rest = tab_look(false, *theme);
            assert_eq!(rest.label, colors.text_muted(), "{name}: muted at rest");
            assert_eq!(rest.weight, TAB_REST_WEIGHT, "{name}: weight 500");
            assert!(!rest.indicator, "{name}: no underline at rest");

            let active = tab_look(true, *theme);
            assert_eq!(active.label, colors.primary, "{name}: primary active");
            assert_eq!(active.weight, TAB_ACTIVE_WEIGHT, "{name}: weight 600");
            assert!(active.indicator, "{name}: the indicator underline");
        }
        // The weights resolve through the typography helpers: 600 is
        // the semibold every theme installs, 500 the medium stand-in.
        for theme in ThemeId::ALL {
            assert_eq!(
                crate::typography::body_semibold_family_name(*theme),
                match theme.name() {
                    "Forest Light" | "Forest Dark" => "Aptos SemiBold",
                    _ => "Geist SemiBold",
                },
                "{}: weight 600",
                theme.name()
            );
        }
        assert_eq!(
            crate::typography::body_medium_family_name(ThemeId::MonoLight),
            "Geist Medium",
            "Monochrome ships the medium instance"
        );
        assert_eq!(
            crate::typography::body_medium_family_name(ThemeId::Light),
            "Aptos",
            "Forest rests at the regular 400 (documented deviation)"
        );
        assert_eq!(
            crate::typography::body_medium_family_name(ThemeId::TealDark),
            "Geist",
            "Teal rests at the regular 400 (documented deviation)"
        );
    }

    /// The indicator is the tab's bottom 2px, so a row that lays its
    /// tabs flush with the row bottom puts the underline over the 1px
    /// rule exactly as the preview's `-1px` bottom margin does.
    #[test]
    fn indicator_sits_over_the_row_rule() {
        let tab = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(100.0, 49.0));
        assert_eq!(
            indicator_rect(tab),
            Rect::from_min_size(Pos2::new(10.0, 67.0), Vec2::new(100.0, 2.0)),
            "the tab's own bottom strip"
        );
        let row = Rect::from_min_size(Pos2::new(0.0, 20.0), Vec2::new(400.0, 49.0));
        assert_eq!(
            rule_rect(row),
            Rect::from_min_size(Pos2::new(0.0, 68.0), Vec2::new(400.0, 1.0)),
            "the row's 1px bottom rule"
        );
        // The indicator's own bottom edge sits 1px past the rule, so
        // the accent mark covers the hairline instead of floating over
        // it.
        assert_eq!(
            indicator_rect(tab).bottom(),
            rule_rect(row).bottom(),
            "both meet the row's bottom edge"
        );
    }

    /// The index bounds: an out-of-range selection clamps to the last
    /// label, and an empty row reports 0.
    #[test]
    fn active_index_clamps_to_the_labels() {
        assert_eq!(clamp_index(0, 4), 0);
        assert_eq!(clamp_index(3, 4), 3);
        assert_eq!(clamp_index(7, 4), 3, "past the end lands on the last");
        assert_eq!(clamp_index(usize::MAX, 3), 2);
        assert_eq!(clamp_index(0, 0), 0, "an empty row reports 0");
        assert_eq!(clamp_index(9, 0), 0);
    }

    /// The widget round trip, headless: a click moves the typed
    /// selection; the arrow keys move it again, wrapping, and land the
    /// focus on the newly active tab.
    #[test]
    fn tabs_click_and_arrow_round_trip() {
        let ctx = egui::Context::default();
        install_faces(&ctx, ThemeId::Light);
        let flags = UiKitFlags::default();
        let theme = ThemeId::Light;
        let labels = ["Overview", "Engagements", "Team"];
        let mut active = 0;

        // Pass 0: record each tab's rect through egui's response reads
        // (the ids the widget gives its tabs).
        let mut tab_rects = Vec::new();
        ctx.run_ui(RawInput::default(), |ui| {
            let base = ui.next_auto_id();
            Tabs::new(&labels).selected(active).ui(ui, theme, &flags);
            for index in 0..labels.len() {
                let id = base.with("vynatix-tabs").with(index);
                if let Some(response) = ui.ctx().read_response(id) {
                    tab_rects.push(response.rect);
                }
            }
        })
        .drop_without_applying_deltas();
        assert_eq!(tab_rects.len(), 3, "one response per tab");
        let second_center = tab_rects[1].center();

        // Passes: press and release on the second tab; the selection
        // moves, typed.
        for pressed in [true, false] {
            let mut input = RawInput::default();
            input.events.push(egui::Event::PointerMoved(second_center));
            input.events.push(egui::Event::PointerButton {
                pos: second_center,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            ctx.run_ui(input, |ui| {
                let response = Tabs::new(&labels).selected(active).ui(ui, theme, &flags);
                if let Some(moved_to) = response.changed {
                    active = moved_to;
                }
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(active, 1, "the click moves the selection");

        // ArrowRight moves to the third, ArrowRight again wraps to the
        // first, and the focus lands on the newly active tab.
        let key = |key: egui::Key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        let mut focused = None;
        for expected in [2, 0] {
            let mut input = RawInput::default();
            input.events.push(key(egui::Key::ArrowRight));
            ctx.run_ui(input, |ui| {
                let base = ui.next_auto_id();
                let response = Tabs::new(&labels).selected(active).ui(ui, theme, &flags);
                assert_eq!(
                    response.changed,
                    Some(expected),
                    "the arrow moves the selection, wrapping"
                );
                if let Some(moved_to) = response.changed {
                    active = moved_to;
                }
                focused = (0..labels.len()).find(|index| {
                    ui.ctx()
                        .read_response(base.with("vynatix-tabs").with(index))
                        .is_some_and(|r| r.has_focus())
                });
            })
            .drop_without_applying_deltas();
            assert_eq!(focused, Some(active), "the focus follows the selection");
        }
        assert_eq!(active, 0, "wrapped past the end back to the first");

        // ArrowLeft from the first wraps to the last.
        let mut input = RawInput::default();
        input.events.push(key(egui::Key::ArrowLeft));
        ctx.run_ui(input, |ui| {
            let response = Tabs::new(&labels).selected(active).ui(ui, theme, &flags);
            assert_eq!(response.changed, Some(2), "left wraps to the last");
        })
        .drop_without_applying_deltas();

        // Out-of-range selections clamp at draw time.
        ctx.run_ui(RawInput::default(), |ui| {
            let response = Tabs::new(&labels).selected(9).ui(ui, theme, &flags);
            assert_eq!(response.selected, 2, "clamped to the last label");
        })
        .drop_without_applying_deltas();
    }

    /// The strip's sideways scroll area keeps a state of its own. An
    /// unsalted `ScrollArea` takes its id from its `Ui` alone, so the
    /// strip and an unsalted page area below it in the same `Ui` once
    /// shared one `scroll_area::State`: the strip, which scrolls
    /// sideways only, clamped the shared vertical offset back to zero
    /// every frame, and the page never moved. A page area scrolled to
    /// 300pt stays there pass after pass.
    #[test]
    fn the_strip_leaves_a_neighbouring_scroll_area_alone() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();
        let labels = ["Brand", "Foundations", "Components", "Feedback"];
        let mut offsets = Vec::new();
        for pass in 0..4 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360.0, 640.0))),
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                Tabs::new(&labels).ui(ui, theme, &flags);
                // The page: unsalted, tall content, scrolled once.
                let mut page = egui::ScrollArea::vertical().max_height(400.0);
                if pass == 0 {
                    page = page.vertical_scroll_offset(300.0);
                }
                let out = page.show(ui, |ui| {
                    ui.allocate_space(Vec2::new(200.0, 2000.0));
                });
                offsets.push(out.state.offset.y);
            })
            .drop_without_applying_deltas();
        }
        assert!(
            offsets.iter().all(|y| (y - 300.0).abs() < 0.5),
            "the page must hold its 300pt offset every pass: {offsets:?}"
        );
    }

    /// The row lays out in the theme's faces at the spec's spacing and
    /// paints without touching the output deltas, in every theme.
    #[test]
    fn tabs_paint_headless_in_every_theme() {
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            install_faces(&ctx, *theme);
            let flags = UiKitFlags::default();
            ctx.run_ui(RawInput::default(), |ui| {
                let response = Tabs::new(&["Foundations", "Components", "Feedback"])
                    .selected(1)
                    .ui(ui, *theme, &flags);
                let height = TAB_PAD_Y * 2.0 + TAB_TEXT_SIZE * 1.5;
                assert!(
                    (response.response.rect.height() - height).abs() < 0.5,
                    "{}: the row is the label's line box plus the 14px paddings",
                    theme.name()
                );
                assert!(
                    response.response.rect.width() > TAB_GAP,
                    "the tabs sit apart"
                );
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(spacing::SPACE_3, 12.0);
        assert_eq!(radii::RADIUS_XS, 2.0, "the focus outline's radius");
    }

    /// Each label starts one tab padding in from its own tab's left edge
    /// and stays inside the tab. The paint once also subtracted half the
    /// label's width from that left anchor, so the first tab "Brand" read
    /// "and" with its text running out of the row's left edge, and every
    /// later label ran back over the 28px gap before its tab.
    #[test]
    fn tab_labels_start_a_padding_in_from_their_tab() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        install_faces(&ctx, theme);
        let flags = UiKitFlags::default();
        let labels = ["Brand", "Components", "Motion"];

        // Each tab's rect through the ids the widget gives its tabs.
        let (tab_rects, shapes) = paint_pass(&ctx, |ui| {
            let base = ui.next_auto_id();
            Tabs::new(&labels).selected(0).ui(ui, theme, &flags);
            (0..labels.len())
                .filter_map(|index| {
                    ui.ctx()
                        .read_response(base.with("vynatix-tabs").with(index))
                })
                .map(|response| response.rect)
                .collect::<Vec<_>>()
        });
        assert_eq!(tab_rects.len(), labels.len(), "one response per tab");

        for (label, tab) in labels.iter().zip(&tab_rects) {
            let pos = assert_painted_inside(&shapes, label, *tab);
            assert!(
                (pos.x - (tab.left() + TAB_PAD_X)).abs() < 0.01,
                "{label}: starts at x {} but a tab padding in is {}",
                pos.x,
                tab.left() + TAB_PAD_X
            );
        }
    }
}
