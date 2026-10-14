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
//! Which focus earns the ring is the browsers' `:focus-visible` rule:
//! focus the keyboard is driving, not focus a pointer or a finger gave.
//! egui reports that a widget holds focus, never how it got there, and
//! the kit's widgets take focus on a click so that Tab and the arrow
//! keys carry on from the one tapped: ringing that focus would leave
//! every tapped chip or tab circled. So the ring keeps its own account
//! of the last input. A key press or typed text reads as keyboard; a
//! mouse press or a touch reads as pointer, and under pointer no ring
//! is drawn until the next key press. A key chord with Ctrl, Alt or Cmd
//! is a shortcut and does not count. Before any input the account
//! reads keyboard, so focus a host requests from code shows the ring.
//! A [`text_input`](FocusRing::text_input) ring ignores the account,
//! as browsers ring a focused text field however it got focus. The
//! first ring to paint in a pass reads that pass's input; every kit
//! widget paints its ring on every frame, so a screen of kit widgets
//! misses none of it (a pass in which no ring paints goes unread).
//!
//! Documented deviation: egui 0.36 draws no focus indicator on its own
//! widgets and exposes no focus styling in `Visuals` (its
//! `show_focused_widget` debug overlay aside), so there is no
//! theme-level install for the brand ring. Every kit widget wraps its
//! response here (enforced component by component from issue 05 on),
//! and a host that wants the ring on an egui built-in wraps it the
//! same way.

use egui::{Context, CornerRadius, Event, Id, Modifiers, Rect, Response, TouchPhase, Ui};

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
    text_input: bool,
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
            text_input: false,
        }
    }

    /// Follow the element's own corner radius instead (pills take
    /// `full`, cards `lg`).
    pub fn radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Ring the widget whenever it holds focus, however the focus
    /// arrived: for text inputs. The browsers' `:focus-visible` always
    /// matches a focused element that takes typed text, even after a
    /// mouse press, since the next thing the person does there is type;
    /// the ring then ignores the account of the last input (module
    /// docs).
    pub const fn text_input(mut self) -> Self {
        self.text_input = true;
        self
    }

    /// Run `add`, then paint the ring around its response when
    /// [`paint`](Self::paint) would. The inner response comes back
    /// unchanged.
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

    /// Paint the treatment around `response` while it holds focus the
    /// keyboard is driving (the module docs say which focus that is).
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
        // The input is read before the focus check, not after it: the
        // press that hides the ring lands frames before the focus it
        // hides (a click completes on release), so a read gated on
        // focus would only ever see the key presses.
        let keyboard = input_modality(ui.ctx()) == Modality::Keyboard;
        let shown = response.has_focus() && (keyboard || self.text_input);
        let Some(started) = shown_since(ui, response, shown) else {
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

/// The input time the ring was first shown on the widget, remembered
/// per widget id in egui's data; `None` while it is not shown. A fresh
/// gain, a widget created already focused and a pointer-focused widget
/// the keyboard then takes over all stamp now, so the fade always has
/// a starting point and a ring that appears late still fades in.
fn shown_since(ui: &Ui, response: &Response, shown: bool) -> Option<f64> {
    let id = response.id;
    let ctx = ui.ctx();
    if !shown {
        // The ring is down: drop the stamp so a later return
        // re-animates.
        ctx.data_mut(|data| data.remove_temp::<f64>(id));
        return None;
    }
    let now = ctx.input(|input| input.time);
    Some(ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(id, || now)))
}

/// What the person last used to drive the UI: the input half of the
/// `:focus-visible` rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modality {
    /// A key press or typed text: focus shows its ring.
    Keyboard,
    /// A mouse press or a touch: focus draws no ring.
    Pointer,
}

/// The account of the last input, with the pass whose input it last
/// read.
#[derive(Clone, Copy, Debug)]
struct LastInput {
    modality: Modality,
    read_pass: Option<u64>,
}

impl Default for LastInput {
    /// Before any input the account reads keyboard: focus a host
    /// requests from code, with no pointer involved, shows the ring.
    fn default() -> Self {
        Self {
            modality: Modality::Keyboard,
            read_pass: None,
        }
    }
}

/// What the person last used, read once per pass off the input events
/// and remembered per viewport in egui's data. A pass that carries no
/// deciding event (an idle frame, or the later pass of a multi-pass
/// frame: egui hands a frame's events to its first pass only) leaves
/// the account as it was.
///
/// Each context call below is its own statement, never inside another
/// one's closure: they all take the context lock.
fn input_modality(ctx: &Context) -> Modality {
    let id = Id::new("ui_kit::focus_ring::last_input").with(ctx.viewport_id());
    let pass = ctx.cumulative_pass_nr();
    let mut last = ctx
        .data(|data| data.get_temp::<LastInput>(id))
        .unwrap_or_default();
    if last.read_pass != Some(pass) {
        let seen = ctx.input(|input| last_modality(&input.events));
        last.modality = seen.unwrap_or(last.modality);
        last.read_pass = Some(pass);
        ctx.data_mut(|data| data.insert_temp(id, last));
    }
    last.modality
}

/// The modality the last deciding event of a pass points to, if the
/// pass has one: a press (mouse or touch) is pointer; a key press that
/// is not a shortcut, or typed text, is keyboard. Releases, moves,
/// scrolls and the like decide nothing.
fn last_modality(events: &[Event]) -> Option<Modality> {
    events.iter().rev().find_map(|event| match event {
        Event::PointerButton { pressed: true, .. }
        | Event::Touch {
            phase: TouchPhase::Start,
            ..
        } => Some(Modality::Pointer),
        Event::Key {
            pressed: true,
            modifiers,
            ..
        } if !is_shortcut(*modifiers) => Some(Modality::Keyboard),
        Event::Text(_) => Some(Modality::Keyboard),
        _ => None,
    })
}

/// A key held with Ctrl, Alt or Cmd is a shortcut (copy, save, an
/// app switcher) rather than someone working the keyboard; egui's own
/// Tab and arrow focus movement ignores such chords the same way.
/// Shift alone is not one: Shift+Tab walks backwards.
fn is_shortcut(modifiers: Modifiers) -> bool {
    modifiers.command || modifiers.ctrl || modifiers.alt
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, PointerButton, Pos2, TouchDeviceId, TouchId};

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

    /// What one frame of the two-button screen showed.
    #[derive(Debug, PartialEq)]
    struct Shown {
        /// Whether the ring went down around each button.
        ring: [bool; 2],
        /// Whether each button holds egui focus.
        focus: [bool; 2],
    }

    /// Two plain buttons wrapped the way the kit's widgets are: a
    /// completed click requests focus itself (egui gives a clicked
    /// button none), and the ring is asked for after that.
    struct Screen {
        ctx: egui::Context,
        text_input: bool,
        centres: [Pos2; 2],
    }

    impl Screen {
        /// The screen after one idle frame, which is what tells it where
        /// the buttons sit.
        fn new(text_input: bool) -> Self {
            let mut screen = Self {
                ctx: egui::Context::default(),
                text_input,
                centres: [Pos2::ZERO; 2],
            };
            screen.run(vec![], false);
            screen
        }

        /// One frame carrying `events`. With `two_passes` the first
        /// pass asks for a second, as a layout that wants one does,
        /// and the readings are the second pass's.
        fn run(&mut self, events: Vec<Event>, two_passes: bool) -> Shown {
            let flags = UiKitFlags::default();
            let mut input = egui::RawInput::default();
            input.events.extend(events);
            let mut shown = Shown {
                ring: [false; 2],
                focus: [false; 2],
            };
            self.ctx
                .run_ui(input, |ui| {
                    for (index, label) in ["First", "Second"].into_iter().enumerate() {
                        let response = ui.add(egui::Button::new(label));
                        if response.clicked() {
                            response.request_focus();
                        }
                        let mut ring = FocusRing::new();
                        if self.text_input {
                            ring = ring.text_input();
                        }
                        shown.ring[index] = ring.paint(ui, &response, ThemeId::Light, &flags);
                        shown.focus[index] = response.has_focus();
                        self.centres[index] = response.rect.center();
                    }
                    if two_passes && ui.ctx().current_pass_index() == 0 {
                        ui.ctx().request_discard("focus ring test: a second pass");
                    }
                })
                .drop_without_applying_deltas();
            shown
        }
    }

    /// The pointer moving to `pos` and pressing or releasing there.
    fn click_half(pos: Pos2, pressed: bool) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ]
    }

    fn key_press(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn touch(phase: TouchPhase) -> Event {
        Event::Touch {
            device_id: TouchDeviceId(0),
            id: TouchId(0),
            phase,
            pos: Pos2::ZERO,
            force: None,
        }
    }

    /// Focus a click gave is real focus with no ring, however many
    /// frames or passes go by; the first key press (Tab here) rings the
    /// focus it moves.
    #[test]
    fn pointer_focus_draws_no_ring_until_a_key_press() {
        let mut screen = Screen::new(false);
        let [first, _] = screen.centres;

        // The press, then the release that completes the click and
        // focuses the button. The release frame runs two passes, the
        // second with no events at all: the account has to outlive the
        // pass that read it.
        screen.run(click_half(first, true), false);
        let clicked = screen.run(click_half(first, false), true);
        assert_eq!(clicked.focus, [true, false], "the click focused it");
        assert_eq!(clicked.ring, [false, false], "pointer focus: no ring");
        let idle = screen.run(vec![], false);
        assert_eq!(idle.focus, [true, false]);
        assert_eq!(idle.ring, [false, false], "and none appears idling");

        // Tab hands the focus on; the key press is keyboard use, so
        // the new focus is ringed (the frame after, when egui has
        // settled it).
        screen.run(vec![key_press(Key::Tab, Modifiers::NONE)], false);
        let tabbed = screen.run(vec![], false);
        assert_eq!(tabbed.focus, [false, true], "Tab moved the focus on");
        assert_eq!(tabbed.ring, [false, true], "keyboard focus: ringed");
    }

    /// A shown ring comes down when a mouse button is pressed or a
    /// finger lands, with the focus kept, and typing brings it back.
    #[test]
    fn a_press_or_a_touch_takes_the_ring_down_and_typing_brings_it_back() {
        let mut screen = Screen::new(false);
        let [first, _] = screen.centres;

        // Keyboard focus to start with: Tab onto the first button.
        screen.run(vec![key_press(Key::Tab, Modifiers::NONE)], false);
        let tabbed = screen.run(vec![], false);
        assert_eq!(tabbed.focus, [true, false]);
        assert_eq!(tabbed.ring, [true, false]);

        // A mouse press on the focused button (and its release, the
        // click) keeps the focus and hides the ring.
        let pressed = screen.run(click_half(first, true), false);
        assert_eq!(pressed.focus, [true, false], "the press kept the focus");
        assert_eq!(pressed.ring, [false, false], "a mouse press: no ring");
        let released = screen.run(click_half(first, false), false);
        assert_eq!(released.focus, [true, false]);
        assert_eq!(released.ring, [false, false]);

        // Typed text is keyboard use.
        let typed = screen.run(vec![Event::Text("a".to_owned())], false);
        assert_eq!(typed.ring, [true, false], "typing brings the ring back");

        // A bare touch start, with no pointer press alongside it to
        // click or move the focus, takes it down again.
        let touched = screen.run(vec![touch(TouchPhase::Start)], false);
        assert_eq!(touched.focus, [true, false], "the touch kept the focus");
        assert_eq!(touched.ring, [false, false], "a touch: no ring");
    }

    /// A text input rings its focus whichever way it arrived.
    #[test]
    fn a_text_input_rings_pointer_focus_too() {
        let mut screen = Screen::new(true);
        let [first, _] = screen.centres;

        screen.run(click_half(first, true), false);
        let clicked = screen.run(click_half(first, false), false);
        assert_eq!(clicked.focus, [true, false], "the click focused it");
        assert_eq!(clicked.ring, [true, false], "a text input: ringed");
    }

    /// Only presses, key presses that are not shortcuts and typed text
    /// decide the modality; the last decider in a pass wins.
    #[test]
    fn the_last_deciding_event_sets_the_modality() {
        let pos = Pos2::ZERO;
        let press = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };

        assert_eq!(last_modality(&[]), None, "no events: no verdict");
        assert_eq!(
            last_modality(&[
                Event::PointerMoved(pos),
                press(false),
                touch(TouchPhase::Move),
                touch(TouchPhase::End),
                Event::Key {
                    key: Key::Tab,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                },
            ]),
            None,
            "moves, releases and key releases decide nothing"
        );

        assert_eq!(last_modality(&[press(true)]), Some(Modality::Pointer));
        assert_eq!(
            last_modality(&[touch(TouchPhase::Start)]),
            Some(Modality::Pointer)
        );
        assert_eq!(
            last_modality(&[key_press(Key::Tab, Modifiers::NONE)]),
            Some(Modality::Keyboard)
        );
        assert_eq!(
            last_modality(&[key_press(Key::Tab, Modifiers::SHIFT)]),
            Some(Modality::Keyboard),
            "Shift+Tab is the keyboard"
        );
        assert_eq!(
            last_modality(&[Event::Text("a".to_owned())]),
            Some(Modality::Keyboard)
        );

        for chord in [Modifiers::CTRL, Modifiers::COMMAND, Modifiers::ALT] {
            assert_eq!(
                last_modality(&[key_press(Key::S, chord)]),
                None,
                "{chord:?}+S is a shortcut, not keyboard use"
            );
        }

        assert_eq!(
            last_modality(&[key_press(Key::Tab, Modifiers::NONE), press(true)]),
            Some(Modality::Pointer),
            "a press after the key wins"
        );
        assert_eq!(
            last_modality(&[press(true), key_press(Key::Tab, Modifiers::NONE)]),
            Some(Modality::Keyboard),
            "a key after the press wins"
        );
    }
}
