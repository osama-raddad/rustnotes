//! FocusRing: the design system's keyboard-focus treatment as a
//! wrapper around any widget's response.
//!
//! Keyboard focus is the system's single most important accessibility
//! signal: a 2px `focus-ring` outline at a 2px offset over a 4px
//! canvas-coloured halo, never a colour change alone. The ring fades in
//! over ~120ms of egui input time ([`egui::Context::input`] time,
//! remembered per widget id); under [`UiKitFlags::reduced_motion`] the
//! fade clamps to 100ms, and without repaints the ring simply appears at
//! full strength (static, documented).
//!
//! Documented deviation: egui 0.36 draws no focus indicator on its own
//! widgets and exposes no focus styling in `Visuals` (its
//! `show_focused_widget` debug overlay aside), so there is no
//! theme-level install for the brand ring. Every kit widget wraps its
//! response here (enforced component by component from issue 05 on),
//! and a host that wants the ring on an egui built-in wraps it the
//! same way.

use egui::{CornerRadius, Rect, Response, Ui};

use crate::states::{self, UiKitFlags};
use crate::theme::ThemeId;
use crate::tokens::radii;

/// Paints the focus treatment around any widget's response: build one,
/// set [`radius`](Self::radius) when the element is not a default (md)
/// widget, then [`wrap`](Self::wrap) or [`paint`](Self::paint).
///
/// ```no_run
/// # use ui_kit::components::FocusRing;
/// # use ui_kit::states::UiKitFlags;
/// # use ui_kit::theme::ThemeId;
/// # let ctx = egui::Context::default();
/// # let theme = ThemeId::Light;
/// # let flags = UiKitFlags::default();
/// # let out = ctx.run_ui(egui::RawInput::default(), |ui| {
/// let response = FocusRing::new().wrap(
///     ui,
///     theme,
///     &flags,
///     |ui| ui.add(egui::Button::new("Start a project")),
/// );
/// # let _ = response;
/// # });
/// # out.drop_without_applying_deltas();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FocusRing {
    radius: CornerRadius,
}

impl Default for FocusRing {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusRing {
    /// A focus ring following the default widget radius (md).
    pub const fn new() -> Self {
        Self {
            radius: CornerRadius::same(radii::RADIUS_MD as u8),
        }
    }

    /// Follow the element's own corner radius instead (pills take
    /// `full`, cards `lg`).
    pub fn radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Run `add`, then paint the ring around its response when it holds
    /// keyboard focus. The inner response comes back unchanged.
    pub fn wrap(
        self,
        ui: &mut Ui,
        theme: ThemeId,
        flags: &UiKitFlags,
        add: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        let response = add(ui);
        self.paint(ui, &response, theme, flags);
        response
    }

    /// Paint the treatment around `response`, keyboard focus only.
    /// Returns true when the ring went down this frame.
    pub fn paint(self, ui: &Ui, response: &Response, theme: ThemeId, flags: &UiKitFlags) -> bool {
        self.paint_at(ui, response, response.rect, theme, flags)
    }

    /// [`paint`] around an explicit `rect`, reading the focus (and the
    /// fade stamp) off `response`: for the widgets whose ring belongs
    /// to one part of a larger hit area — the choice controls' 18px
    /// box and circle, the toggle's track, each inside a row whose
    /// label is part of the click target.
    pub fn paint_at(
        self,
        ui: &Ui,
        response: &Response,
        rect: Rect,
        theme: ThemeId,
        flags: &UiKitFlags,
    ) -> bool {
        let Some(started) = focus_since(ui, response) else {
            return false;
        };
        let fade = flags.fade_ms(states::FOCUS_FADE_MS) as f64 / 1000.0;
        let now = ui.ctx().input(|input| input.time);
        let fraction = ((now - started) / fade).clamp(0.0, 1.0) as f32;
        if fraction < 1.0 {
            // Play the fade out: repaint until it lands.
            let remaining = fade - (now - started);
            ui.ctx()
                .request_repaint_after_secs(remaining as f32 + 0.004);
        }
        let geometry =
            states::focus_ring_geometry(rect, self.radius, &theme.colors()).faded(fraction);
        let painter = ui.painter().clone().with_layer_id(response.layer_id);
        painter.add(geometry.halo);
        painter.add(geometry.ring);
        true
    }
}

/// The input time the widget was first seen focused, remembered per
/// widget id in egui's data; `None` while it holds none. A fresh gain
/// and a widget created already focused both stamp now, so the fade
/// always has a starting point.
fn focus_since(ui: &Ui, response: &Response) -> Option<f64> {
    let id = response.id;
    let ctx = ui.ctx();
    if !response.has_focus() {
        // Focus left: drop the stamp so a later regain re-animates.
        ctx.data_mut(|data| data.remove_temp::<f64>(id));
        return None;
    }
    let now = ctx.input(|input| input.time);
    Some(ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(id, || now)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring paints nothing on an unfocused widget, then every frame
    /// the widget holds the focus requested through egui's own Tab
    /// order machinery.
    #[test]
    fn ring_paints_only_while_focused() {
        let ctx = egui::Context::default();
        let flags = UiKitFlags::default();
        let mut painted = Vec::new();
        let mut seen_id = None;
        for pass in 0..3 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let response = ui.add(egui::Button::new("Start a project"));
                if pass == 0 {
                    seen_id = Some(response.id);
                }
                painted.push(FocusRing::new().paint(ui, &response, ThemeId::Light, &flags));
            })
            .drop_without_applying_deltas();
            if pass == 0 {
                ctx.memory_mut(|memory| memory.request_focus(seen_id.unwrap()));
            }
        }
        assert_eq!(painted, vec![false, true, true]);
    }

    /// `wrap` hands back the inner response untouched.
    #[test]
    fn wrap_returns_the_inner_response() {
        let ctx = egui::Context::default();
        let mut direct = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let wrapped = FocusRing::new().radius(4).wrap(
                ui,
                ThemeId::TealLight,
                &UiKitFlags::default(),
                |ui| {
                    let response = ui.add(egui::Button::new("Read the case study"));
                    direct = Some(response.id);
                    response
                },
            );
            assert_eq!(direct, Some(wrapped.id));
        })
        .drop_without_applying_deltas();
    }
}
