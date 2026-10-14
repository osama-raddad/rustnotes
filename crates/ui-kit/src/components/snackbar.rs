//! Snackbar: the design system's brief operational message, the
//! `.snackbar` rules the `components/Snackbar` preview carries and its
//! README.
//!
//! A snackbar is a brief message about something the person just did,
//! on an inverse surface near the bottom of the screen with at most
//! one action: `inverse-surface` at `radius-lg` under `shadow-l3`,
//! 48px tall at the least, 14px `inverse-on-surface` text. The action
//! is a text button in `inverse-primary` at weight 600; the dismiss a
//! 36px `close` icon button. The status variants carry a 32px tile
//! before the text: `check_circle` in `success` on
//! `success-container`, `error` in `error` on `error-container` — the
//! status marks fall below 3:1 on the inverse surface itself, so the
//! tile stays. The optional title sets weight 600 and the line under
//! it drops to 13px.
//!
//! [`SnackbarHost`] is the state machine: one visible snackbar at a
//! time — a new one replaces the current, it never queues — with a
//! timed auto-dismiss the spec pins (about 6s for info and success,
//! 10s with an action; errors stay until dismissed or resolved). The
//! timer pauses while the pointer or the focus is inside, and
//! [`UiKitFlags::reduced_motion`] touches the entrance only — the
//! fade clamps to 100ms and the slide-up goes — never the timing.
//! Placement: centred at the bottom, 24px above the edge, up to 600px
//! wide; the host paints it over the page (an anchored [`egui::Area`]
//! in the foreground order), never moving focus to it, and the
//! snackbar's action stays available elsewhere in the page.
//!
//! Copy voice: one short sentence, outcome first; the action is a
//! verb — "Undo", "Retry now", "Saved.", "Link copied."

use egui::{Color32, Context, CornerRadius, Pos2, Rect, Sense};

use crate::components::focus_ring::FocusRing;
use crate::icon::IconSize;
use crate::states::{self, UiKitFlags};
use crate::theme::ThemeId;
use crate::tokens::{radii, spacing, type_scale};

/// How long an info or success snackbar stays: about 6 seconds.
pub const INFO_DURATION_MS: f32 = 6000.0;
/// How long one with an action stays: 10 seconds.
pub const ACTION_DURATION_MS: f32 = 10000.0;
/// The entrance fade: a step on the motion duration scale.
pub const ENTRANCE_MS: f32 = 200.0;
/// The entrance's slide-up, in px, dropped under reduced motion.
pub const ENTRANCE_SLIDE_PX: f32 = 8.0;
/// The widest a snackbar may be.
pub const MAX_WIDTH: f32 = 600.0;
/// The clearance above the screen's bottom edge (or any bottom bar).
pub const BOTTOM_MARGIN: f32 = spacing::SPACE_6;
/// The least a snackbar stands tall.
pub const MIN_HEIGHT: f32 = 48.0;
/// The action button's height.
pub const ACTION_MIN_HEIGHT: f32 = 36.0;
/// The status tile: 32px square, `radius-md`.
pub const STATUS_TILE: f32 = 32.0;
/// The dismiss button: 36px.
pub const DISMISS_SIZE: f32 = 36.0;

/// One snackbar's status: the plain inverse surface, or one of the
/// two tile variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SnackbarStatus {
    /// The plain `.snackbar`: no tile, `role="status"`.
    #[default]
    Info,
    /// `snackbar--success`: the `check_circle` tile, `role="status"`.
    Success,
    /// `snackbar--error`: the `error` tile, `role="alert"`.
    Error,
}

impl SnackbarStatus {
    /// Every status, preview order.
    pub const ALL: &[SnackbarStatus] = &[
        SnackbarStatus::Info,
        SnackbarStatus::Success,
        SnackbarStatus::Error,
    ];
}

/// How long a snackbar of `status` with `has_action` stays on screen,
/// per the spec: about 6s, 10s with an action; errors stay until
/// dismissed or resolved (`None`).
pub fn duration_ms(status: SnackbarStatus, has_action: bool) -> Option<f32> {
    match status {
        SnackbarStatus::Info | SnackbarStatus::Success => Some(if has_action {
            ACTION_DURATION_MS
        } else {
            INFO_DURATION_MS
        }),
        SnackbarStatus::Error => None,
    }
}

/// The entrance duration resolved under the flags: the fade clamps to
/// 100ms under reduced motion; the timing of the message itself never
/// changes (see [`SnackbarHost::tick`], which takes no flags).
pub fn entrance_ms(flags: &UiKitFlags) -> f32 {
    flags.fade_ms(ENTRANCE_MS)
}

/// One snackbar: the message the host wants shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Snackbar {
    /// The status variant, which picks the tile.
    pub status: SnackbarStatus,
    /// The one short sentence, outcome first. The plain message when
    /// no title rides.
    pub message: String,
    /// The optional bold title line.
    pub title: Option<String>,
    /// The optional 13px line under the title.
    pub line: Option<String>,
    /// The one action's verb label, at most one.
    pub action: Option<String>,
}

impl Snackbar {
    /// A plain info snackbar carrying one sentence.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            status: SnackbarStatus::Info,
            message: message.into(),
            title: None,
            line: None,
            action: None,
        }
    }

    /// A success snackbar carrying one sentence.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            status: SnackbarStatus::Success,
            message: message.into(),
            title: None,
            line: None,
            action: None,
        }
    }

    /// An error snackbar: it stays until dismissed or resolved.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: SnackbarStatus::Error,
            message: message.into(),
            title: None,
            line: None,
            action: None,
        }
    }

    /// Set the bold title line.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the 13px line under the title.
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.line = Some(line.into());
        self
    }

    /// Set the one action's verb label.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// The typed events a snackbar host reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnackbarEvent {
    /// The action button fired; the host runs it and the snackbar is
    /// gone.
    Action,
    /// The dismiss button fired.
    Dismissed,
    /// The timer ran out.
    TimedOut,
}

/// The host's one visible snackbar with its timer.
#[derive(Clone, Debug)]
struct Visible {
    snackbar: Snackbar,
    elapsed_ms: f32,
}

/// The snackbar host: the one-at-a-time rule, the timed
/// auto-dismiss, the pause while the pointer or the focus is inside,
/// and the entrance state. Pure logic ([`SnackbarHost::tick`]) drives
/// the machine, so the whole flow is testable without a context; the
/// host paints through [`SnackbarHost::show`].
#[derive(Clone, Debug, Default)]
pub struct SnackbarHost {
    visible: Option<Visible>,
    paused: bool,
}

impl SnackbarHost {
    /// An idle host: nothing showing.
    pub const fn new() -> Self {
        Self {
            visible: None,
            paused: false,
        }
    }

    /// Show `snackbar`, replacing whatever is on screen: one at a
    /// time, the spec's rule — a new snackbar replaces the current
    /// one, it never queues. The replacement's timer starts from zero.
    pub fn show(&mut self, snackbar: Snackbar) {
        self.visible = Some(Visible {
            snackbar,
            elapsed_ms: 0.0,
        });
        self.paused = false;
    }

    /// The snackbar on screen, if any.
    pub fn current(&self) -> Option<&Snackbar> {
        self.visible.as_ref().map(|visible| &visible.snackbar)
    }

    /// Whether a snackbar is on screen.
    pub fn is_showing(&self) -> bool {
        self.visible.is_some()
    }

    /// Hold or release the timer: the spec pauses it while the
    /// pointer or the focus is inside the snackbar.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Advance the clock by `dt_ms` and report the timeout when it
    /// fires: info and success leave at their duration, errors never
    /// time out. Never consults the flags — reduced motion touches
    /// the entrance only, never this timing.
    pub fn tick(&mut self, dt_ms: f32) -> Option<SnackbarEvent> {
        let visible = self.visible.as_mut()?;
        if self.paused {
            return None;
        }
        visible.elapsed_ms += dt_ms.max(0.0);
        let duration = duration_ms(visible.snackbar.status, visible.snackbar.action.is_some())?;
        if visible.elapsed_ms >= duration {
            self.visible = None;
            Some(SnackbarEvent::TimedOut)
        } else {
            None
        }
    }

    /// Dismiss now (the dismiss button): clears the slot and reports
    /// the typed event.
    pub fn dismiss(&mut self) -> Option<SnackbarEvent> {
        self.visible.take().map(|_| SnackbarEvent::Dismissed)
    }

    /// Fire the action (the action button): clears the slot and
    /// reports the typed event. A snackbar without one reports
    /// nothing, and nothing changes.
    pub fn trigger_action(&mut self) -> Option<SnackbarEvent> {
        if self
            .visible
            .as_ref()
            .is_some_and(|visible| visible.snackbar.action.is_some())
        {
            self.visible = None;
            Some(SnackbarEvent::Action)
        } else {
            None
        }
    }

    /// The entrance fraction (0 to 1) for the snackbar on screen,
    /// resolved under the flags' reduced-motion clamps.
    pub fn entrance_fraction(&self, flags: &UiKitFlags) -> f32 {
        match &self.visible {
            None => 0.0,
            Some(visible) => (visible.elapsed_ms / entrance_ms(flags)).clamp(0.0, 1.0),
        }
    }

    /// Paint the snackbar on screen when one is showing: the anchored
    /// bottom-centred area, the entrance, the action and dismiss
    /// controls, and the frame's clock tick (which pauses while the
    /// pointer or the focus is inside). Reports the event this frame.
    pub fn paint(
        &mut self,
        ctx: &Context,
        theme: ThemeId,
        flags: &UiKitFlags,
    ) -> Option<SnackbarEvent> {
        if !self.is_showing() {
            return None;
        }
        let look = snackbar_look(self.current().unwrap().status, theme);
        let fraction = self.entrance_fraction(flags);
        let fade = |color: Color32| color.gamma_multiply(fraction);
        let slide = if flags.spatial_ms(ENTRANCE_MS) > 0.0 {
            (1.0 - fraction) * ENTRANCE_SLIDE_PX
        } else {
            0.0
        };

        let id = egui::Id::new("vynatix-snackbar");
        let mut event = None;
        let mut paused_now = false;
        egui::Area::new(id)
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -BOTTOM_MARGIN - slide),
            )
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                let snackbar = self.current().unwrap().clone();
                let colors = theme.colors();

                // The text: the message alone at 14px, or the bold
                // title over the 13px line.
                let has_title = snackbar.title.is_some();
                let title_galley = snackbar.title.as_ref().map(|title| {
                    let job = one_line(
                        title,
                        egui::FontId {
                            size: type_scale::TEXT_BODY_SM,
                            family: crate::typography::body_semibold_family(theme),
                        },
                    );
                    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                });
                let line_text = if has_title {
                    snackbar
                        .line
                        .as_deref()
                        .unwrap_or(snackbar.message.as_str())
                } else {
                    snackbar.message.as_str()
                };
                let line_size = if has_title || snackbar.line.is_some() {
                    type_scale::TEXT_LABEL
                } else {
                    type_scale::TEXT_BODY_SM
                };

                // The accessories, measured so the text wraps into the
                // width left inside the 600px cap.
                let action_galley = snackbar.action.as_ref().map(|label| {
                    let job = one_line(
                        label,
                        egui::FontId {
                            size: type_scale::TEXT_BODY_SM,
                            family: crate::typography::body_semibold_family(theme),
                        },
                    );
                    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                });
                let tile = look.tile.is_some();
                let mut reserved = ACTION_MIN_HEIGHT.max(DISMISS_SIZE) + spacing::SPACE_3;
                if tile {
                    reserved += STATUS_TILE + spacing::SPACE_3;
                }
                let wrap = (MAX_WIDTH
                    - 2.0 * spacing::SPACE_3
                    - spacing::SPACE_2
                    - reserved
                    - action_galley
                        .as_ref()
                        .map_or(0.0, |galley| galley.size().x + 2.0 * spacing::SPACE_3))
                .max(120.0);
                let line_galley = ui.ctx().fonts_mut(|fonts| {
                    let mut job = one_line(
                        line_text,
                        egui::FontId {
                            size: line_size,
                            family: crate::typography::TypeStyle::Body.family(theme),
                        },
                    );
                    job.wrap.max_width = wrap;
                    fonts.layout_job(job)
                });

                let mut text_height = line_galley.size().y;
                if let Some(galley) = &title_galley {
                    text_height += galley.size().y;
                }
                let height = MIN_HEIGHT
                    .max(text_height + 2.0 * spacing::SPACE_2)
                    .max(if tile {
                        STATUS_TILE + 2.0 * spacing::SPACE_2
                    } else {
                        0.0
                    });
                let width = (spacing::SPACE_4
                    + spacing::SPACE_2
                    + reserved
                    + action_galley.as_ref().map_or(0.0, |galley| galley.size().x)
                    + line_galley.size().x
                    + spacing::SPACE_2)
                    .min(MAX_WIDTH)
                    .max(line_galley.size().x + spacing::SPACE_4 + spacing::SPACE_2);

                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::new(width, height), Sense::hover());
                let surface = ui.interact(rect, id.with("surface"), Sense::click());

                // The pane: shadow-l3 over the inverse surface, both
                // riding the entrance fade.
                let radius = CornerRadius::same(radii::RADIUS_LG as u8);
                let mut shadow = crate::theme::egui_shadow(theme.shadows().l3);
                shadow.color = fade(shadow.color);
                ui.painter().add(shadow.as_shape(rect, radius));
                ui.painter()
                    .rect_filled(rect, radius, fade(colors.inverse_surface));

                // The status tile before the text.
                let mut cursor = rect.left() + spacing::SPACE_4;
                if let (true, Some(fill), Some(mark), Some(glyph)) =
                    (tile, look.tile, look.tile_mark, look.tile_icon)
                {
                    let tile_rect = Rect::from_min_size(
                        Pos2::new(cursor, rect.center().y - STATUS_TILE / 2.0),
                        egui::Vec2::splat(STATUS_TILE),
                    );
                    ui.painter().rect_filled(
                        tile_rect,
                        CornerRadius::same(radii::RADIUS_MD as u8),
                        fade(fill),
                    );
                    ui.painter().text(
                        tile_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        glyph.as_char().to_string(),
                        egui::FontId {
                            size: IconSize::Md.px(),
                            family: crate::icon::family_for_theme(theme, false),
                        },
                        fade(mark),
                    );
                    cursor += STATUS_TILE + spacing::SPACE_3;
                }

                // The text block, top-aligned beside the tile.
                let mut text_y = rect.top() + spacing::SPACE_2;
                if let Some(galley) = &title_galley {
                    ui.painter()
                        .galley(Pos2::new(cursor, text_y), galley.clone(), fade(look.text));
                    text_y += galley.size().y;
                }
                ui.painter()
                    .galley(Pos2::new(cursor, text_y), line_galley, fade(look.text));

                // The action: an inverse-primary text button, ending a
                // space-2 gap short of the dismiss circle.
                if let Some(galley) = &action_galley {
                    let action_rect = Rect::from_min_size(
                        Pos2::new(
                            rect.right()
                                - DISMISS_SIZE
                                - spacing::SPACE_2
                                - (galley.size().x + 2.0 * spacing::SPACE_3),
                            rect.center().y - ACTION_MIN_HEIGHT / 2.0,
                        ),
                        egui::Vec2::new(
                            galley.size().x + 2.0 * spacing::SPACE_3,
                            ACTION_MIN_HEIGHT,
                        ),
                    );
                    let action = ui.interact(action_rect, id.with("action"), Sense::click());
                    let state = crate::states::WidgetState::of(&action);
                    let fill = if state.pressed {
                        states::pressed_overlay(look.action)
                    } else if state.hovered {
                        states::hover_overlay(look.action)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let painter = ui.painter().clone().with_layer_id(action.layer_id);
                    if fill != Color32::TRANSPARENT {
                        painter.rect_filled(
                            action_rect,
                            CornerRadius::same(radii::RADIUS_MD as u8),
                            fade(fill),
                        );
                    }
                    painter.galley(
                        crate::typography::left_center(
                            action_rect.left() + spacing::SPACE_3,
                            action_rect.center().y,
                            galley.size(),
                        ),
                        galley.clone(),
                        fade(look.action),
                    );
                    FocusRing::new()
                        .radius(radii::RADIUS_MD as u8)
                        .paint(ui, &action, theme, flags);
                    action.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            snackbar.action.as_deref().unwrap_or_default(),
                        )
                    });
                    paused_now |= action.hovered() || action.has_focus();
                    if action.clicked() {
                        event = self.trigger_action();
                    }
                }

                // The dismiss: a 36px circle, the close icon.
                let dismiss_rect = Rect::from_min_size(
                    Pos2::new(
                        rect.right() - DISMISS_SIZE,
                        rect.center().y - DISMISS_SIZE / 2.0,
                    ),
                    egui::Vec2::splat(DISMISS_SIZE),
                );
                let dismiss = ui.interact(dismiss_rect, id.with("dismiss"), Sense::click());
                let state = crate::states::WidgetState::of(&dismiss);
                let fill = if state.pressed {
                    states::pressed_overlay(look.text)
                } else if state.hovered {
                    states::hover_overlay(look.text)
                } else {
                    Color32::TRANSPARENT
                };
                let painter = ui.painter().clone().with_layer_id(dismiss.layer_id);
                if fill != Color32::TRANSPARENT {
                    painter.rect_filled(
                        dismiss_rect,
                        CornerRadius::same((DISMISS_SIZE / 2.0) as u8),
                        fade(fill),
                    );
                }
                painter.text(
                    dismiss_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    crate::icons::Icon::Close.as_char().to_string(),
                    egui::FontId {
                        size: IconSize::Md.px(),
                        family: crate::icon::family_for_theme(theme, false),
                    },
                    fade(look.text),
                );
                FocusRing::new()
                    .radius(radii::RADIUS_FULL as u8)
                    .paint(ui, &dismiss, theme, flags);
                dismiss.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Dismiss")
                });
                paused_now |= dismiss.hovered() || dismiss.has_focus();

                // The surface hover pauses the timer too.
                paused_now |= surface.hovered();
            });

        // The pointer or the focus was inside: hold the timer.
        self.set_paused(paused_now);

        // The frame's clock: tick after painting, so a timeout leaves
        // the screen at the frame after the one that crossed it.
        let dt_ms = ctx.input(|input| input.unstable_dt.max(0.0) * 1000.0);
        event = event.or(self.tick(dt_ms));

        // Repaint while the entrance plays and while a timed
        // snackbar waits out its clock; errors sit until dismissed.
        if fraction < 1.0 {
            ctx.request_repaint_after_secs(0.016);
        }
        if let Some(visible) = &self.visible {
            if let Some(duration) =
                duration_ms(visible.snackbar.status, visible.snackbar.action.is_some())
            {
                let remaining = ((duration - visible.elapsed_ms) / 1000.0).max(0.0);
                ctx.request_repaint_after_secs(remaining + 0.004);
            }
        }
        event
    }
}

/// A one-line layout job in one format.
fn one_line(text: &str, font_id: egui::FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(text, 0.0, egui::TextFormat::simple(font_id, Color32::WHITE));
    job
}

/// The inverse pair the snackbar paints in: pure data off the tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnackbarLook {
    /// `inverse-surface`.
    pub surface: Color32,
    /// `inverse-on-surface`.
    pub text: Color32,
    /// `inverse-primary`: the action label.
    pub action: Color32,
    /// The status tile's fill, when the status carries one.
    pub tile: Option<Color32>,
    /// The status tile's icon colour.
    pub tile_mark: Option<Color32>,
    /// The status tile's icon.
    pub tile_icon: Option<crate::icons::Icon>,
}

/// Resolve the snackbar's paints, per theme and status.
pub fn snackbar_look(status: SnackbarStatus, theme: ThemeId) -> SnackbarLook {
    let colors = theme.colors();
    let (tile, tile_mark, tile_icon) = match status {
        SnackbarStatus::Info => (None, None, None),
        SnackbarStatus::Success => (
            Some(colors.success_container),
            Some(colors.success),
            Some(crate::icons::Icon::CheckCircle),
        ),
        SnackbarStatus::Error => (
            Some(colors.error_container),
            Some(colors.error),
            Some(crate::icons::Icon::Error),
        ),
    };
    SnackbarLook {
        surface: colors.inverse_surface,
        text: colors.inverse_on_surface,
        action: colors.inverse_primary,
        tile,
        tile_mark,
        tile_icon,
    }
}

#[cfg(test)]
mod tests {
    use crate::icons::Icon;
    use crate::states::UiKitFlags;
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing};
    use crate::typography::{assert_painted_inside, paint_pass};

    use super::{
        duration_ms, entrance_ms, snackbar_look, Snackbar, SnackbarEvent, SnackbarHost,
        SnackbarStatus, ACTION_DURATION_MS, ACTION_MIN_HEIGHT, BOTTOM_MARGIN, DISMISS_SIZE,
        ENTRANCE_MS, INFO_DURATION_MS, MAX_WIDTH, MIN_HEIGHT, STATUS_TILE,
    };

    /// The spec's timing table: info and success leave after about 6
    /// seconds, 10 with an action; errors stay until dismissed or
    /// resolved.
    #[test]
    fn durations_follow_the_spec_table() {
        assert_eq!(duration_ms(SnackbarStatus::Info, false), Some(6000.0));
        assert_eq!(duration_ms(SnackbarStatus::Success, false), Some(6000.0));
        assert_eq!(duration_ms(SnackbarStatus::Info, true), Some(10000.0));
        assert_eq!(duration_ms(SnackbarStatus::Success, true), Some(10000.0));
        assert_eq!(duration_ms(SnackbarStatus::Error, false), None);
        assert_eq!(duration_ms(SnackbarStatus::Error, true), None);
        assert_eq!(INFO_DURATION_MS, 6000.0);
        assert_eq!(ACTION_DURATION_MS, 10000.0);
    }

    /// The machine the issue names: enqueue → show → timeout → gone.
    /// The timer fires exactly at the duration and clears the slot.
    #[test]
    fn the_timer_fires_and_clears() {
        let mut host = SnackbarHost::new();
        assert!(!host.is_showing(), "starts idle");

        host.show(Snackbar::info("Saved."));
        assert!(host.is_showing(), "enqueue shows");
        assert_eq!(host.current().map(|s| s.message.as_str()), Some("Saved."));

        assert_eq!(host.tick(INFO_DURATION_MS - 1.0), None, "one ms early");
        assert!(host.is_showing(), "still up at 5999ms");
        assert_eq!(host.tick(1.0), Some(SnackbarEvent::TimedOut));
        assert!(!host.is_showing(), "gone at 6000ms");
        assert_eq!(host.tick(1000.0), None, "an idle host times nothing");

        // An action holds it longer.
        host.show(Snackbar::info("Timesheet submitted for week 41.").action("View"));
        assert_eq!(host.tick(ACTION_DURATION_MS - 1.0), None);
        assert!(host.is_showing());
        assert_eq!(host.tick(1.0), Some(SnackbarEvent::TimedOut));
    }

    /// Errors stay until dismissed or resolved: no timeout ever fires.
    #[test]
    fn errors_stay_until_dismissed() {
        let mut host = SnackbarHost::new();
        host.show(Snackbar::error("Sync failed").line("Upstream returned 503."));
        for _ in 0..10 {
            assert_eq!(host.tick(10_000.0), None, "a minute passes");
            assert!(host.is_showing(), "the error stays");
        }
        assert_eq!(host.dismiss(), Some(SnackbarEvent::Dismissed));
        assert!(!host.is_showing());
        assert_eq!(host.dismiss(), None, "an empty host dismisses nothing");
    }

    /// One at a time: a second enqueue replaces the current snackbar,
    /// it never queues, and the replacement's timer starts over.
    #[test]
    fn a_second_enqueue_replaces_never_queues() {
        let mut host = SnackbarHost::new();
        host.show(Snackbar::info("Saved."));
        host.show(Snackbar::error("Sync failed"));
        assert_eq!(
            host.current().map(|s| s.status),
            Some(SnackbarStatus::Error),
            "the newest is the one on screen"
        );

        // The replacement's timer starts from zero: 6s of ticking
        // after the replace does not retire an 10s action snackbar.
        host.show(Snackbar::success("Workspace archived").action("Undo"));
        assert_eq!(host.tick(INFO_DURATION_MS), None, "the timer restarted");
        assert!(host.is_showing());
        assert_eq!(
            host.tick(ACTION_DURATION_MS - INFO_DURATION_MS),
            Some(SnackbarEvent::TimedOut)
        );
    }

    /// The timer pauses while the pointer or the focus is inside, and
    /// releases when they leave.
    #[test]
    fn the_pause_holds_the_timer() {
        let mut host = SnackbarHost::new();
        host.show(Snackbar::info("Link copied."));
        host.set_paused(true);
        assert_eq!(host.tick(60_000.0), None, "a paused timer stands still");
        assert!(host.is_showing());
        host.set_paused(false);
        assert_eq!(host.tick(INFO_DURATION_MS), Some(SnackbarEvent::TimedOut));
    }

    /// The action and the dismiss both report their typed event and
    /// clear the slot.
    #[test]
    fn action_and_dismiss_events_clear_the_host() {
        let mut host = SnackbarHost::new();
        host.show(Snackbar::success("Workspace archived").action("Undo"));
        assert_eq!(host.trigger_action(), Some(SnackbarEvent::Action));
        assert!(!host.is_showing(), "the action dismisses");

        host.show(Snackbar::info("Link copied."));
        assert_eq!(host.trigger_action(), None, "no action to fire");
        assert!(host.is_showing(), "nothing happened");
        assert_eq!(host.dismiss(), Some(SnackbarEvent::Dismissed));
        assert!(!host.is_showing());
    }

    /// Reduced motion trims the entrance, never the timing: the
    /// entrance clamps to 100ms and the slide goes, while the
    /// auto-dismiss duration stays exactly the spec's numbers under
    /// both flag sets.
    #[test]
    fn reduced_motion_trims_the_entrance_never_the_timing() {
        let full = UiKitFlags::default();
        let reduced = UiKitFlags {
            reduced_motion: true,
            ..Default::default()
        };

        assert_eq!(entrance_ms(&full), ENTRANCE_MS);
        assert_eq!(entrance_ms(&reduced), 100.0, "the fade clamps to 100ms");

        // The entrance completes sooner under reduction while the
        // message holds for the full six seconds either way.
        let mut host = SnackbarHost::new();
        host.show(Snackbar::info("Saved."));
        assert_eq!(host.entrance_fraction(&reduced), 0.0, "starts hidden");
        host.tick(50.0);
        assert_eq!(host.entrance_fraction(&reduced), 0.5, "halfway at 50ms");
        host.tick(50.0);
        assert_eq!(host.entrance_fraction(&reduced), 1.0, "done at 100ms");
        assert!(host.is_showing(), "the message itself is still up");
        // 100ms of the entrance already ran off the clock.
        assert_eq!(host.tick(INFO_DURATION_MS - 100.0 - 1.0), None);
        assert_eq!(host.tick(1.0), Some(SnackbarEvent::TimedOut));

        let mut host = SnackbarHost::new();
        host.show(Snackbar::info("Saved."));
        host.tick(100.0);
        assert!(
            host.entrance_fraction(&full) < 1.0,
            "the full entrance takes 200ms"
        );
    }

    /// The placement and shape constants: centred at the bottom, 24px
    /// above the edge, up to 600px wide, 48px at the least, the
    /// 36px dismiss and the 32px tile, `radius-lg` under `shadow-l3`.
    #[test]
    fn placement_constants_match_the_spec() {
        assert_eq!(MAX_WIDTH, 600.0);
        assert_eq!(BOTTOM_MARGIN, 24.0, "space-6");
        assert_eq!(BOTTOM_MARGIN, spacing::SPACE_6);
        assert_eq!(MIN_HEIGHT, 48.0);
        assert_eq!(ACTION_MIN_HEIGHT, 36.0);
        assert_eq!(DISMISS_SIZE, 36.0);
        assert_eq!(STATUS_TILE, 32.0);
        assert_eq!(radii::RADIUS_LG, 12.0, "radius-lg");
    }

    /// The looks per theme: the inverse pair everywhere, the action in
    /// `inverse-primary`, and the status tiles only on success and
    /// error — the marks fall below 3:1 on the inverse surface itself,
    /// so the tile stays.
    #[test]
    fn looks_resolve_the_inverse_pair_per_theme() {
        for theme in ThemeId::ALL {
            let colors = theme.colors();
            let name = theme.name();

            let info = snackbar_look(SnackbarStatus::Info, *theme);
            assert_eq!(info.surface, colors.inverse_surface, "{name}: the surface");
            assert_eq!(info.text, colors.inverse_on_surface, "{name}: the text");
            assert_eq!(info.action, colors.inverse_primary, "{name}: the action");
            assert_eq!(info.tile, None, "{name}: no tile on info");
            assert_eq!(info.tile_mark, None);

            let success = snackbar_look(SnackbarStatus::Success, *theme);
            assert_eq!(success.tile, Some(colors.success_container), "{name}: tile");
            assert_eq!(success.tile_mark, Some(colors.success), "{name}: mark");
            assert_eq!(success.tile_icon, Some(Icon::CheckCircle), "{name}: glyph");

            let error = snackbar_look(SnackbarStatus::Error, *theme);
            assert_eq!(error.tile, Some(colors.error_container), "{name}: tile");
            assert_eq!(error.tile_mark, Some(colors.error), "{name}: mark");
            assert_eq!(error.tile_icon, Some(Icon::Error), "{name}: glyph");
        }
    }

    /// The constructors and builders carry their status and their
    /// optional parts, the action at most one verb label.
    #[test]
    fn constructors_carry_status_and_parts() {
        let info = Snackbar::info("Link copied.");
        assert_eq!(info.status, SnackbarStatus::Info);
        assert_eq!(info.message, "Link copied.");
        assert_eq!(info.title, None);
        assert_eq!(info.line, None);
        assert_eq!(info.action, None);

        let archived = Snackbar::success("Workspace archived")
            .title("Workspace archived")
            .line("Read-only access for 30 days, then permanent removal.")
            .action("Undo");
        assert_eq!(archived.status, SnackbarStatus::Success);
        assert_eq!(archived.action.as_deref(), Some("Undo"));
        assert_eq!(
            archived.line.as_deref(),
            Some("Read-only access for 30 days, then permanent removal.")
        );

        let sync = Snackbar::error("Sync failed").action("Retry now");
        assert_eq!(sync.status, SnackbarStatus::Error);
        assert_eq!(sync.action.as_deref(), Some("Retry now"));
    }

    /// The paint is quiet when idle and draws the anchored snackbar
    /// when one is showing, in every theme and every status, without
    /// touching the output deltas.
    #[test]
    fn paint_is_quiet_idle_and_draws_when_showing() {
        for theme in ThemeId::ALL {
            let ctx = egui::Context::default();
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                crate::icon::install(ui.ctx());
                crate::typography::install(*theme, ui.ctx());
            });
            out.drop_without_applying_deltas();
            let flags = UiKitFlags::default();

            let mut host = SnackbarHost::new();

            ctx.run_ui(egui::RawInput::default(), |ui| {
                assert_eq!(
                    host.paint(ui.ctx(), *theme, &flags),
                    None,
                    "idle paints nothing"
                );
            })
            .drop_without_applying_deltas();

            for status in SnackbarStatus::ALL {
                let snackbar = Snackbar {
                    status: *status,
                    ..Snackbar::info("Timesheet submitted for week 41.")
                        .title("Workspace archived")
                        .line("Read-only access for 30 days.")
                        .action("Undo")
                };
                host.show(snackbar);
                ctx.run_ui(egui::RawInput::default(), |ui| {
                    let event = host.paint(ui.ctx(), *theme, &flags);
                    assert_eq!(event, None, "no interaction this frame");
                })
                .drop_without_applying_deltas();
                assert!(host.is_showing(), "{} {:?}", theme.name(), status);
            }
        }
    }

    /// The action sits where the layout puts it: its rect ends a space-2
    /// gap short of the dismiss circle, and its label starts one space-3
    /// padding inside the rect, centred on the snackbar's midline. The
    /// rect's x once carried a height term (the action's minimum height
    /// less the label's), which slid the button about 14px further from
    /// the dismiss circle than the layout intends. The rects are read
    /// back through the ids the host gives its surface and its action.
    #[test]
    fn the_action_ends_a_gap_short_of_the_dismiss_circle() {
        let ctx = egui::Context::default();
        let theme = ThemeId::Light;
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let flags = UiKitFlags::default();
        let mut host = SnackbarHost::new();
        host.show(Snackbar::info("Timesheet submitted for week 41.").action("Undo"));

        // The anchored area is invisible on its first frame, while egui
        // learns its size: one pass settles it, the next is the one read.
        paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
        });
        let ((surface, action), shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            let id = egui::Id::new("vynatix-snackbar");
            let rect_of = |name: &str| {
                ui.ctx()
                    .read_response(id.with(name))
                    .unwrap_or_else(|| panic!("the host laid out its {name}"))
                    .rect
            };
            (rect_of("surface"), rect_of("action"))
        });

        let pos = assert_painted_inside(&shapes, "Undo", action);
        let label_edge = action.left() + spacing::SPACE_3;
        assert!(
            (pos.x - label_edge).abs() < 0.01,
            "the label starts at x {} but one padding inside the action is {label_edge}",
            pos.x
        );
        let action_edge = surface.right() - DISMISS_SIZE - spacing::SPACE_2;
        assert!(
            (action.right() - action_edge).abs() < 0.01,
            "the action ends at x {} but a gap short of the dismiss circle is {action_edge}",
            action.right()
        );
    }
}
