//! Snackbar: the design system's brief operational message, the
//! `.snackbar` rules the `components/Snackbar` preview carries and its
//! README.
//!
//! A snackbar is a brief message about something the person just did,
//! on an inverse surface near the bottom of the screen with at most
//! one action: `inverse-surface` at `radius-lg` under `shadow-l3`,
//! 48px tall at the least (the 36px dismiss circle between the row's
//! padding lifts that to 52), 14px `inverse-on-surface` text. The action
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
//! timer pauses while the pointer or the focus is inside — the
//! entrance runs on regardless, so a pointer resting where the
//! snackbar appears cannot hold it part-faded — and
//! [`UiKitFlags::reduced_motion`] touches the entrance only — the
//! fade clamps to 100ms and the slide-up goes — never the timing.
//! Placement: centred at the bottom, 24px above the edge, hugging its
//! text up to 600px wide and, on a phone (under the 600px tablet
//! breakpoint), spanning the window less a 16px margin on each side; the
//! host paints it over the page (an anchored [`egui::Area`] in the
//! foreground order, with egui's own fade-in off so the entrance is the
//! only fade), never moving focus to it, and the snackbar's action
//! stays available elsewhere in the page. The row inside follows the
//! preview across and down: `padding: 8 8 8 16` and `gap: space-3`,
//! border-box, so the 36px dismiss circle between the padding makes
//! the pane 52 tall at the least, and the text block carries its own
//! `padding-block: space-1`.
//!
//! Copy voice: one short sentence, outcome first; the action is a
//! verb — "Undo", "Retry now", "Saved.", "Link copied."

use egui::{Color32, Context, CornerRadius, Pos2, Rect, Sense};

use crate::components::focus_ring::FocusRing;
use crate::icon::IconSize;
use crate::states::{self, UiKitFlags};
use crate::theme::ThemeId;
use crate::tokens::{breakpoints, radii, spacing, type_scale};

/// How long an info or success snackbar stays: about 6 seconds.
pub const INFO_DURATION_MS: f32 = 6000.0;
/// How long one with an action stays: 10 seconds.
pub const ACTION_DURATION_MS: f32 = 10000.0;
/// The entrance fade: a step on the motion duration scale.
pub const ENTRANCE_MS: f32 = 200.0;
/// The entrance's slide-up, in px, dropped under reduced motion.
pub const ENTRANCE_SLIDE_PX: f32 = 8.0;
/// The widest a snackbar may be. It hugs its text up to this cap; under
/// the tablet breakpoint (a phone) it spans the window less a space-4
/// margin on each side instead.
pub const MAX_WIDTH: f32 = 600.0;
/// The clearance above the screen's bottom edge (or any bottom bar).
pub const BOTTOM_MARGIN: f32 = spacing::SPACE_6;
/// The least a snackbar stands tall: the preview's `min-height`. The
/// dismiss circle between the row's padding makes it 52 in practice.
pub const MIN_HEIGHT: f32 = 48.0;
/// The action button's height.
pub const ACTION_MIN_HEIGHT: f32 = 36.0;
/// The status tile: 32px square, `radius-md`.
pub const STATUS_TILE: f32 = 32.0;
/// The dismiss button: 36px.
pub const DISMISS_SIZE: f32 = 36.0;
/// The row's padding before its first item: the preview's
/// `padding-left`, space-4.
const ROW_PAD_START: f32 = spacing::SPACE_4;
/// The row's padding after its last item: `padding-right`, space-2.
const ROW_PAD_END: f32 = spacing::SPACE_2;
/// The row's padding above and below: space-2.
const ROW_PAD_Y: f32 = spacing::SPACE_2;
/// The gap between the row's items: `gap: space-3`.
const ROW_GAP: f32 = spacing::SPACE_3;
/// The text block's own padding above and below, inside the row's:
/// `.snackbar__text { padding-block: space-1 }`.
const TEXT_PAD_Y: f32 = spacing::SPACE_1;

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

/// The host's one visible snackbar with its two clocks.
#[derive(Clone, Debug)]
struct Visible {
    snackbar: Snackbar,
    /// The dismissal clock: the time the snackbar has been up, less the
    /// time it was paused (the pointer or the focus inside).
    elapsed_ms: f32,
    /// How long it has been on screen, which nothing holds: the entrance
    /// runs on it, so a pointer resting where the snackbar appears
    /// cannot freeze the fade part-way.
    age_ms: f32,
}

/// The snackbar host: the one-at-a-time rule, the timed
/// auto-dismiss, the pause while the pointer or the focus is inside,
/// and the entrance state. Pure logic ([`SnackbarHost::tick`]) drives
/// the machine, so the whole flow is testable without a context; the
/// host paints through [`SnackbarHost::paint`].
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
            age_ms: 0.0,
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
    /// pointer or the focus is inside the snackbar. The pause holds
    /// the dismissal clock alone; the entrance runs on regardless.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Advance the clocks by `dt_ms` and report the timeout when it
    /// fires: info and success leave at their duration, errors never
    /// time out. Never consults the flags — reduced motion touches
    /// the entrance only, never this timing. The snackbar's age, which
    /// the entrance runs on, always advances; a pause holds only the
    /// dismissal clock.
    pub fn tick(&mut self, dt_ms: f32) -> Option<SnackbarEvent> {
        let visible = self.visible.as_mut()?;
        visible.age_ms += dt_ms.max(0.0);
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
            Some(visible) => (visible.age_ms / entrance_ms(flags)).clamp(0.0, 1.0),
        }
    }

    /// Paint the snackbar on screen when one is showing: the anchored
    /// bottom-centred area, the entrance, the action and dismiss
    /// controls, and the frame's clock tick (the pause, while the pointer
    /// or the focus is inside, holds only the dismissal clock). Reports
    /// the event this frame.
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
            // The snackbar fades itself in, and reduced motion shortens
            // that fade. egui's own area fade-in would stack a second one
            // on top that no flag governs.
            .fade_in(false)
            .show(ctx, |ui| {
                let snackbar = self.current().unwrap().clone();
                let colors = theme.colors();

                // The row, left to right, as the preview sets it
                // (`padding: 8 8 8 16`, `gap: space-3`): the tile, the
                // text, the action's pill and the dismiss circle, a gap
                // between each. The wrap, the width and every placement
                // below read these terms and no others.
                let tile = look.tile.is_some();
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
                // The action's pill: its label between a space-3 padding
                // on each side.
                let action_width = action_galley
                    .as_ref()
                    .map_or(0.0, |galley| galley.size().x + 2.0 * spacing::SPACE_3);
                let before_text = ROW_PAD_START + if tile { STATUS_TILE + ROW_GAP } else { 0.0 };
                let after_text = ROW_GAP
                    + if action_galley.is_some() {
                        action_width + ROW_GAP
                    } else {
                        0.0
                    }
                    + DISMISS_SIZE
                    + ROW_PAD_END;
                // Up to the 600px cap, and never wider than the window
                // less a space-4 margin on each side.
                let content_width = ctx.content_rect().width();
                let max_width = MAX_WIDTH
                    .min(content_width - 2.0 * spacing::SPACE_4)
                    .max(0.0);
                // The text column takes what the row leaves and may shrink
                // to nothing, as the preview's `min-width: 0` lets it: a
                // floor would run the text into the pill on a narrow
                // phone. epaint breaks a word that is wider than the room.
                let wrap = (max_width - before_text - after_text).max(0.0);

                // The text: the message alone at 14px, or the bold title
                // over the 13px line. Both wrap into the room left.
                let wrapped = |text: &str, font_id: egui::FontId| {
                    let mut job = one_line(text, font_id);
                    job.wrap.max_width = wrap;
                    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                };
                let has_title = snackbar.title.is_some();
                let title_galley = snackbar.title.as_ref().map(|title| {
                    wrapped(
                        title,
                        egui::FontId {
                            size: type_scale::TEXT_BODY_SM,
                            family: crate::typography::body_semibold_family(theme),
                        },
                    )
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
                let line_galley = wrapped(
                    line_text,
                    egui::FontId {
                        size: line_size,
                        family: crate::typography::TypeStyle::Body.family(theme),
                    },
                );

                let mut text_width = line_galley.size().x;
                let mut text_height = line_galley.size().y;
                if let Some(galley) = &title_galley {
                    text_width = text_width.max(galley.size().x);
                    text_height += galley.size().y;
                }
                // The row is border-box in the preview: the dismiss circle
                // between the row's padding makes the pane 52 at the least
                // (the tile and the pill are no taller than it), and the
                // text block carries a padding of its own above and below.
                let height = MIN_HEIGHT
                    .max(DISMISS_SIZE + 2.0 * ROW_PAD_Y)
                    .max(text_height + 2.0 * (ROW_PAD_Y + TEXT_PAD_Y));
                // On a phone, under the tablet breakpoint, the pane spans
                // the window less its margins, the text taking the room the
                // controls leave; from the breakpoint up it hugs its text,
                // up to the cap.
                let width = if content_width < breakpoints::BREAKPOINT_TABLET {
                    max_width
                } else {
                    (before_text + text_width + after_text).min(max_width)
                };

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
                if let (true, Some(fill), Some(mark), Some(glyph)) =
                    (tile, look.tile, look.tile_mark, look.tile_icon)
                {
                    let tile_rect = Rect::from_min_size(
                        Pos2::new(
                            rect.left() + ROW_PAD_START,
                            rect.center().y - STATUS_TILE / 2.0,
                        ),
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
                }

                // The text block, centred on the pane's midline beside
                // the tile.
                let text_x = rect.left() + before_text;
                let mut text_y = rect.center().y - text_height / 2.0;
                if let Some(galley) = &title_galley {
                    ui.painter()
                        .galley(Pos2::new(text_x, text_y), galley.clone(), fade(look.text));
                    text_y += galley.size().y;
                }
                ui.painter()
                    .galley(Pos2::new(text_x, text_y), line_galley, fade(look.text));

                // The dismiss circle ends a padding short of the pane's
                // edge, so its focus ring and halo stay inside the pane.
                let dismiss_rect = Rect::from_min_size(
                    Pos2::new(
                        rect.right() - ROW_PAD_END - DISMISS_SIZE,
                        rect.center().y - DISMISS_SIZE / 2.0,
                    ),
                    egui::Vec2::splat(DISMISS_SIZE),
                );

                // The action: an inverse-primary text button, a gap short
                // of the dismiss circle.
                if let Some(galley) = &action_galley {
                    let action_rect = Rect::from_min_size(
                        Pos2::new(
                            dismiss_rect.left() - ROW_GAP - action_width,
                            rect.center().y - ACTION_MIN_HEIGHT / 2.0,
                        ),
                        egui::Vec2::new(action_width, ACTION_MIN_HEIGHT),
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
                if dismiss.clicked() {
                    event = event.or_else(|| self.dismiss());
                }

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

/// A one-line layout job in one format. The colour is left to the
/// painter (the placeholder): the paint passes the look's colour,
/// faded by the entrance, as the fallback, and epaint swaps it in for
/// the placeholder only. A colour set here would win over both.
fn one_line(text: &str, font_id: egui::FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat::simple(font_id, Color32::PLACEHOLDER),
    );
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
    use egui::Color32;

    use crate::icons::Icon;
    use crate::states::{self, UiKitFlags, REDUCED_FADE_MAX_MS};
    use crate::theme::ThemeId;
    use crate::tokens::{radii, spacing};
    use crate::typography::{assert_painted_inside, paint_pass, painted_text};

    use super::{
        duration_ms, entrance_ms, snackbar_look, Snackbar, SnackbarEvent, SnackbarHost,
        SnackbarStatus, ACTION_DURATION_MS, ACTION_MIN_HEIGHT, BOTTOM_MARGIN, DISMISS_SIZE,
        ENTRANCE_MS, INFO_DURATION_MS, MAX_WIDTH, MIN_HEIGHT, STATUS_TILE,
    };

    /// Assert two coordinates agree to a hundredth of a point.
    fn assert_close(actual: f32, expected: f32, what: &str) {
        assert!(
            (actual - expected).abs() < 0.01,
            "{what}: {actual} but expected {expected}"
        );
    }

    /// The colour a painted text shows in, resolved the way epaint's
    /// tessellator does it: the shape's override, else the galley's own
    /// colour, else the shape's fallback where the galley leaves the
    /// colour to the painter (`Color32::PLACEHOLDER`), then faded by the
    /// shape's opacity factor.
    fn shown_colour(shape: &egui::epaint::TextShape) -> Color32 {
        let own = shape.galley.job.sections[0].format.color;
        let colour = match shape.override_text_color {
            Some(over) => over,
            None if own == Color32::PLACEHOLDER => shape.fallback_color,
            None => own,
        };
        colour.gamma_multiply(shape.opacity_factor)
    }

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

    /// The pause holds the dismissal timer and nothing else: a pointer
    /// resting where the snackbar appears must not freeze its entrance
    /// part-faded. The entrance runs on the snackbar's age, which keeps
    /// counting through a pause, while the paused time stays off the
    /// dismissal clock.
    #[test]
    fn a_pause_holds_the_timer_but_not_the_entrance() {
        let flags = UiKitFlags::default();
        let mut host = SnackbarHost::new();
        host.show(Snackbar::info("Saved."));
        host.set_paused(true);
        assert_eq!(host.tick(100.0), None, "a paused host never times out");
        assert_eq!(
            host.entrance_fraction(&flags),
            0.5,
            "the entrance runs on: 100ms of 200"
        );
        host.tick(100.0);
        assert_eq!(
            host.entrance_fraction(&flags),
            1.0,
            "and completes under the pointer"
        );

        // The paused 200ms never ran off the dismissal clock: the full
        // six seconds are still due once the pause lifts.
        host.set_paused(false);
        assert_eq!(host.tick(INFO_DURATION_MS - 1.0), None);
        assert_eq!(host.tick(1.0), Some(SnackbarEvent::TimedOut));
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

    /// The rect the host laid out for `name` ("surface", "action" or
    /// "dismiss"), read back through the id the host gives it. Read it
    /// inside the pass that painted it: that is the pass whose layout it
    /// reports.
    fn rect_of(ui: &egui::Ui, name: &str) -> egui::Rect {
        ui.ctx()
            .read_response(egui::Id::new("vynatix-snackbar").with(name))
            .unwrap_or_else(|| panic!("the host laid out its {name}"))
            .rect
    }

    /// A context carrying the kit's faces, with `snackbar` showing on a
    /// fresh host and the one pass that settles it. egui keeps a new
    /// anchored area invisible on its first frame, while it learns the
    /// area's size, so that pass paints nothing and the next is the first
    /// visible one; reading the layout and the shapes inside that pass
    /// needs nothing more. `window`, when given, sets the window's size,
    /// which holds for the passes that follow.
    fn settled(
        theme: ThemeId,
        flags: &UiKitFlags,
        window: Option<egui::Vec2>,
        snackbar: Snackbar,
    ) -> (egui::Context, SnackbarHost) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: window.map(|size| egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            crate::icon::install(ui.ctx());
            crate::typography::install(theme, ui.ctx());
        });
        out.drop_without_applying_deltas();
        let mut host = SnackbarHost::new();
        host.show(snackbar);
        paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, flags);
        });
        (ctx, host)
    }

    /// The row sits as the preview sets it (`padding: 8 8 8 16`, `gap:
    /// space-3`): the dismiss circle ends a space-2 padding short of the
    /// pane's right edge, the action's pill a space-3 gap short of the
    /// circle, and the pill starts a space-3 gap past the message's end,
    /// its label one space-3 padding inside it and centred on the pane's
    /// midline. The pill's x once carried a height term that slid it
    /// about 14px from the circle, its width left out its own side
    /// padding so it overlapped the message, and the circle sat flush
    /// against the pane's edge. Down the row follows the preview too
    /// (border-box): the 36px circle between the space-2 padding makes
    /// the pane 52 tall, and the circle and the pill sit a space-2
    /// padding from its top and bottom; the pane was 48 tall, so they sat
    /// 6px from the edges.
    #[test]
    fn the_row_sits_as_the_preview_sets_it() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();
        let (ctx, mut host) = settled(
            theme,
            &flags,
            None,
            Snackbar::info("Timesheet submitted for week 41.").action("Undo"),
        );
        let ((surface, action, dismiss), shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            (
                rect_of(ui, "surface"),
                rect_of(ui, "action"),
                rect_of(ui, "dismiss"),
            )
        });

        let message = painted_text(&shapes, "Timesheet submitted for week 41.");
        let message_end = message.galley.rect.translate(message.pos.to_vec2()).right();
        assert_close(
            dismiss.right(),
            surface.right() - spacing::SPACE_2,
            "the circle ends a space-2 padding short of the pane's edge",
        );
        assert_close(
            action.right(),
            dismiss.left() - spacing::SPACE_3,
            "the pill ends a space-3 gap short of the circle",
        );
        assert_close(
            action.left(),
            message_end + spacing::SPACE_3,
            "the pill starts a space-3 gap past the message",
        );
        let label = assert_painted_inside(&shapes, "Undo", action);
        assert_close(
            label.x,
            action.left() + spacing::SPACE_3,
            "the label starts one space-3 padding inside the pill",
        );
        assert_close(
            surface.height(),
            DISMISS_SIZE + 2.0 * spacing::SPACE_2,
            "the pane is the dismiss circle between the row's padding",
        );
        for (what, control) in [("circle", dismiss), ("pill", action)] {
            assert_close(
                control.top() - surface.top(),
                spacing::SPACE_2,
                &format!("the {what} sits a space-2 padding below the pane's top"),
            );
            assert_close(
                surface.bottom() - control.bottom(),
                spacing::SPACE_2,
                &format!("the {what} sits a space-2 padding above the pane's bottom"),
            );
        }
    }

    /// The focus ring and halo of the dismiss circle and of the action's
    /// pill, four points out from the control, stay inside the pane. The
    /// circle once sat flush against the pane's edge, so its treatment
    /// was cut off there.
    #[test]
    fn the_controls_focus_treatment_stays_inside_the_pane() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();
        let (ctx, mut host) = settled(
            theme,
            &flags,
            None,
            Snackbar::info("Link copied.").action("Undo"),
        );
        let ((surface, action, dismiss), _) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            (
                rect_of(ui, "surface"),
                rect_of(ui, "action"),
                rect_of(ui, "dismiss"),
            )
        });
        for (what, control) in [("dismiss circle", dismiss), ("action pill", action)] {
            let treatment =
                states::focus_ring_geometry(control, egui::CornerRadius::same(0), &theme.colors())
                    .outer_edge();
            assert!(
                surface.contains_rect(treatment),
                "the {what}'s focus ring and halo ({treatment:?}) leave the pane ({surface:?})"
            );
        }
    }

    /// A press and release on the painted dismiss cross dismisses the
    /// snackbar: the release frame reports the dismissed event and the
    /// host has nothing left on screen, while the press alone does
    /// nothing. The pointer goes to the centre of the cross as the frame
    /// painted it, with the entrance over so nothing slides under it. The
    /// cross once did not act on a click at all: the circle was painted,
    /// hoverable and focusable, and the host never read its click.
    #[test]
    fn clicking_the_dismiss_cross_dismisses_the_snackbar() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();
        let (ctx, mut host) = settled(
            theme,
            &flags,
            None,
            Snackbar::info("Link copied.").action("Undo"),
        );
        host.tick(ENTRANCE_MS);
        let (_, shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
        });
        let cross = painted_text(&shapes, &Icon::Close.as_char().to_string());
        let at = cross.galley.rect.translate(cross.pos.to_vec2()).center();

        let mut events = Vec::new();
        for pressed in [true, false] {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::PointerMoved(at));
            input.events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            let mut event = None;
            ctx.run_ui(input, |ui| {
                event = host.paint(ui.ctx(), theme, &flags);
            })
            .drop_without_applying_deltas();
            events.push(event);
        }
        assert_eq!(
            events,
            vec![None, Some(SnackbarEvent::Dismissed)],
            "the release dismisses, the press alone does not"
        );
        assert_eq!(
            host.current(),
            None,
            "the snackbar is gone: nothing is left on screen"
        );
    }

    /// The text takes the look's colours and rides the entrance fade:
    /// the title and the line paint in `inverse-on-surface`, the action
    /// in `inverse-primary`, in every theme (the inverse surface flips
    /// between the light and the dark ones, so a fixed colour reads on
    /// one and not the other), and mid-entrance each colour is faded by
    /// the entrance fraction. The galleys once carried an explicit
    /// white, which epaint never swaps for the fallback colour the paint
    /// passes, so neither the look's colours nor the fade reached the
    /// text.
    #[test]
    fn text_takes_the_looks_colours_and_rides_the_fade() {
        for theme in ThemeId::ALL {
            let name = theme.name();
            let flags = UiKitFlags::default();
            let look = snackbar_look(SnackbarStatus::Info, *theme);
            let (ctx, mut host) = settled(
                *theme,
                &flags,
                None,
                Snackbar::info("Timesheet submitted for week 41.")
                    .title("Workspace archived")
                    .line("Read-only access for 30 days.")
                    .action("Undo"),
            );

            // Mid-entrance first, then with the entrance over.
            for entered in [false, true] {
                if entered {
                    host.tick(ENTRANCE_MS);
                }
                let (fraction, shapes) = paint_pass(&ctx, |ui| {
                    let fraction = host.entrance_fraction(&flags);
                    host.paint(ui.ctx(), *theme, &flags);
                    fraction
                });
                assert_eq!(
                    fraction == 1.0,
                    entered,
                    "{name}: entrance fraction {fraction}"
                );
                for (text, colour) in [
                    ("Workspace archived", look.text),
                    ("Read-only access for 30 days.", look.text),
                    ("Undo", look.action),
                ] {
                    assert_eq!(
                        shown_colour(&painted_text(&shapes, text)),
                        colour.gamma_multiply(fraction),
                        "{name}: {text:?} at entrance fraction {fraction}"
                    );
                }
            }
        }
    }

    /// The kit's entrance is the only fade on the text. Reduced motion
    /// clamps it to 100ms, so once those 100ms have run the first visible
    /// pass shows the text at full strength; without the flag the same
    /// pass is still part-way through the 200ms fade. Nothing stacks on
    /// either: egui fades a new area in by itself unless told not to,
    /// which dimmed the first frames whatever the flag said. egui's own
    /// animation time stays at its default here, so that fade would show.
    #[test]
    fn the_entrance_is_the_only_fade_on_the_text() {
        let theme = ThemeId::Light;
        let look = snackbar_look(SnackbarStatus::Info, theme);
        for reduced in [false, true] {
            let flags = UiKitFlags {
                reduced_motion: reduced,
                ..Default::default()
            };
            let (ctx, mut host) = settled(
                theme,
                &flags,
                None,
                Snackbar::info("Timesheet submitted for week 41.").action("Undo"),
            );
            // The whole of the reduced-motion fade, run before the first
            // visible frame.
            host.tick(REDUCED_FADE_MAX_MS);
            let (fraction, shapes) = paint_pass(&ctx, |ui| {
                let fraction = host.entrance_fraction(&flags);
                host.paint(ui.ctx(), theme, &flags);
                fraction
            });

            assert_eq!(
                fraction == 1.0,
                reduced,
                "reduced motion {reduced}: entrance fraction {fraction}"
            );
            for (text, colour) in [
                ("Timesheet submitted for week 41.", look.text),
                ("Undo", look.action),
            ] {
                assert_eq!(
                    shown_colour(&painted_text(&shapes, text)),
                    colour.gamma_multiply(fraction),
                    "reduced motion {reduced}: {text:?} at entrance fraction {fraction}"
                );
            }
        }
    }

    /// On a phone the snackbar is no wider than the window less a space-4
    /// margin on each side (the README: "full width less 16px margins"),
    /// not its 600px cap centred and clipped, and its text never runs
    /// into the pill. At 390pt a long message wraps inside the pane. At
    /// 320pt the gallery's "Sync failed" demo, with its tile and "Retry
    /// now", leaves the text column only about 70pt; the column shrinks
    /// to that (the preview's `min-width: 0`) instead of holding a 120pt
    /// floor that ran the text into the pill. The pane fits the content
    /// rect with its margins and the dismiss circle is whole inside the
    /// window.
    #[test]
    fn the_snackbar_fits_a_phone_with_its_margins() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();
        let message = "Timesheet submitted for week 41. Your manager will review it before Friday.";
        let title = "Sync failed";
        let line = "Upstream returned 503. Retrying automatically.";
        let cases = [
            (
                390.0,
                Snackbar::success(message).action("Retry now"),
                vec![message],
            ),
            (
                320.0,
                Snackbar::error(title)
                    .title(title)
                    .line(line)
                    .action("Retry now"),
                vec![title, line],
            ),
        ];
        for (window, snackbar, texts) in cases {
            let (ctx, mut host) = settled(theme, &flags, Some(egui::vec2(window, 844.0)), snackbar);
            let ((surface, action, dismiss), shapes) = paint_pass(&ctx, |ui| {
                host.paint(ui.ctx(), theme, &flags);
                (
                    rect_of(ui, "surface"),
                    rect_of(ui, "action"),
                    rect_of(ui, "dismiss"),
                )
            });

            let content = ctx.content_rect();
            assert_close(content.width(), window, "the window the pass ran in");
            assert!(
                surface.left() >= content.left() + spacing::SPACE_4 - 0.01
                    && surface.right() <= content.right() - spacing::SPACE_4 + 0.01,
                "{window}pt: the pane {surface:?} leaves the content rect {content:?} less its margins"
            );
            assert!(
                content.contains_rect(dismiss),
                "{window}pt: the dismiss circle {dismiss:?} is cut off by the window {content:?}"
            );
            for text in &texts {
                let shape = painted_text(&shapes, text);
                let text_end = shape.galley.rect.translate(shape.pos.to_vec2()).right();
                assert!(
                    text_end <= action.left() - spacing::SPACE_3 + 0.01,
                    "{window}pt: {text:?} ends at x {text_end}, inside the gap before the pill at {}",
                    action.left()
                );
            }
            let longest = painted_text(&shapes, texts[texts.len() - 1]);
            assert!(
                longest.galley.rows.len() > 1,
                "{window}pt: the long text wraps"
            );
        }
    }

    /// On a phone, under the tablet breakpoint, the snackbar spans the
    /// window less a space-4 margin on each side ("full width less 16px
    /// margins"), whatever its message; from the breakpoint up it hugs
    /// its text. At 390pt, and at 599pt just under the breakpoint, a short
    /// message still gets a pane exactly the content rect less the
    /// margins, its controls on the right; at 600pt the same message
    /// hugs its text. The pane used to hug at every width.
    #[test]
    fn on_a_phone_the_snackbar_spans_the_window_less_its_margins() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();
        for (window, spans) in [(390.0, true), (599.0, true), (600.0, false)] {
            let (ctx, mut host) = settled(
                theme,
                &flags,
                Some(egui::vec2(window, 844.0)),
                Snackbar::info("Link copied.").action("Undo"),
            );
            let ((surface, dismiss), _) = paint_pass(&ctx, |ui| {
                host.paint(ui.ctx(), theme, &flags);
                (rect_of(ui, "surface"), rect_of(ui, "dismiss"))
            });
            let content = ctx.content_rect();
            let spanning = content.width() - 2.0 * spacing::SPACE_4;
            if spans {
                assert_close(
                    surface.width(),
                    spanning,
                    &format!("{window}pt: the pane spans the window less its margins"),
                );
                assert_close(
                    surface.left(),
                    content.left() + spacing::SPACE_4,
                    &format!("{window}pt: the pane starts a space-4 margin in"),
                );
                assert_close(
                    dismiss.right(),
                    surface.right() - spacing::SPACE_2,
                    &format!("{window}pt: the controls stay on the pane's right"),
                );
            } else {
                assert!(
                    surface.width() < spanning,
                    "{window}pt: a short message hugs its text ({}) instead of spanning ({spanning})",
                    surface.width()
                );
            }
        }
    }

    /// The title sets the text width and wraps like the line. A title
    /// wider than its line pushes the pill out to a space-3 gap past its
    /// own end, and a title wider than the pane wraps before the pill.
    /// It was laid out without wrapping and never measured, so a wide
    /// title ran on under the pill and the circle.
    #[test]
    fn a_wide_title_sets_the_text_width_and_wraps_before_the_pill() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();
        let line = "Read-only access.";

        // Wider than its line: the pill clears the title.
        let title = "Workspace archived for the whole quarter";
        let (ctx, mut host) = settled(
            theme,
            &flags,
            None,
            Snackbar::info("Archived")
                .title(title)
                .line(line)
                .action("Undo"),
        );
        let (action, shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            rect_of(ui, "action")
        });
        let (title_shape, line_shape) = (painted_text(&shapes, title), painted_text(&shapes, line));
        let title_end = title_shape
            .galley
            .rect
            .translate(title_shape.pos.to_vec2())
            .right();
        let line_end = line_shape
            .galley
            .rect
            .translate(line_shape.pos.to_vec2())
            .right();
        assert!(title_end > line_end, "the title is the wider text");
        assert_close(
            action.left(),
            title_end + spacing::SPACE_3,
            "the pill starts a space-3 gap past the wider title",
        );

        // Wider than the pane: the title wraps and ends before the pill.
        let long = "Workspace archived for the whole quarter, with every file, member and setting in it, until the owner restores it";
        let (ctx, mut host) = settled(
            theme,
            &flags,
            None,
            Snackbar::info("Archived")
                .title(long)
                .line(line)
                .action("Undo"),
        );
        let (action, shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            rect_of(ui, "action")
        });
        let title_shape = painted_text(&shapes, long);
        assert!(title_shape.galley.rows.len() > 1, "the long title wraps");
        let title_end = title_shape
            .galley
            .rect
            .translate(title_shape.pos.to_vec2())
            .right();
        assert!(
            title_end <= action.left() - spacing::SPACE_3 + 0.01,
            "the title ends at x {title_end}, inside the gap before the pill at {}",
            action.left()
        );
    }

    /// The text block is centred on the pane's midline: a one-line
    /// message sits on it within half a point (top-aligned in a 48pt pane
    /// it sat about 7pt above), and a title over a wrapped line, whose
    /// pane hugs the text, starts the row's space-2 padding plus the text
    /// block's own space-1 below the pane's top.
    #[test]
    fn the_text_block_is_centred_on_the_pane() {
        let theme = ThemeId::Light;
        let flags = UiKitFlags::default();

        let (ctx, mut host) = settled(theme, &flags, None, Snackbar::info("Link copied."));
        let (surface, shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            rect_of(ui, "surface")
        });
        assert_painted_inside(&shapes, "Link copied.", surface);

        let title = "Workspace archived";
        let line = "Read-only access for 30 days, then permanent removal of the workspace, every file in it, and every member's access to either of them.";
        let (ctx, mut host) = settled(
            theme,
            &flags,
            None,
            Snackbar::info("Archived").title(title).line(line),
        );
        let (surface, shapes) = paint_pass(&ctx, |ui| {
            host.paint(ui.ctx(), theme, &flags);
            rect_of(ui, "surface")
        });
        let (title_shape, line_shape) = (painted_text(&shapes, title), painted_text(&shapes, line));
        assert!(
            line_shape.galley.rows.len() > 1,
            "the line wraps, so the pane hugs its text"
        );
        let text_height = title_shape.galley.size().y + line_shape.galley.size().y;
        let text_pad = spacing::SPACE_2 + spacing::SPACE_1;
        assert_close(
            surface.height(),
            text_height + 2.0 * text_pad,
            "the pane hugs its text, padded by the row and by the text block",
        );
        assert_close(
            title_shape.pos.y,
            surface.top() + text_pad,
            "the title starts a space-2 row padding and a space-1 text padding below the pane's top",
        );
        assert_close(
            line_shape.pos.y,
            title_shape.pos.y + title_shape.galley.size().y,
            "the line follows the title",
        );
    }
}
