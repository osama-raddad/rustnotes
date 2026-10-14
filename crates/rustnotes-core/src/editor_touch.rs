//! Touch intent in the note editor: a swipe scrolls, a tap places the caret,
//! a press held still starts a selection.
//!
//! egui's `TextEdit` lets a touch drag select text once the field has focus
//! (`text_edit/builder.rs`: `!has_touch_screen() || has_focus(id)`), and the
//! note editor keeps focus, so every finger drag used to become a selection
//! and the editor's `ScrollArea` never saw a swipe. Native editors decide by
//! intent; this filter does the same, on the raw input before egui sees it.
//! Per touch on the editor:
//!
//! - **Pending**: the finger is down, its simulated press and moves are
//!   withheld. Its `Touch` events always pass, so egui still knows it runs
//!   on a touch screen.
//! - **Scroll**: the finger leaves the slop before the long-press window
//!   ends. Its moves become wheel deltas with a hover pointer parked where
//!   the touch started; egui hands the wheel to the scroll area under a
//!   pointer that drags nothing, so they land on the editor even when the
//!   finger wanders onto the top bar. The deltas go out as a trackpad
//!   scroll (wheel `Start`, `Move`s, `End`), which egui applies in the frame
//!   they arrive; plain wheel steps of 8 pt or more would be smoothed over
//!   later frames, and the content would trail the finger. The press is
//!   never delivered.
//! - **Tap**: the finger lifts inside the slop and the window. Press and
//!   release are replayed together at the start point: a click, which puts
//!   the caret there.
//! - **Select**: the window ends with the finger inside the slop. The press
//!   is delivered then, at the start point, and the rest of the touch
//!   passes untouched, so egui's drag selection works as it did.
//!
//! Only the finger egui-winit mirrors as the pointer (the first one down) is
//! filtered. Other fingers pass untouched, and so does a mouse, which sends
//! no `Touch` events at all.

use egui::{Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect, TouchId, TouchPhase};
use std::time::Duration;

/// Movement, in points, before a touch in the editor becomes a swipe.
const SCROLL_SLOP: f32 = 8.0;
/// How long, in seconds, a touch must stay inside the slop to start a
/// selection instead of a tap or a swipe.
pub(crate) const LONG_PRESS: f64 = 0.4;

/// The editor's touch-intent filter. `App::filter_input` runs it every frame,
/// after the edge swipe has taken its touches.
#[derive(Default)]
pub(crate) struct EditorTouch {
    gesture: Gesture,
    /// A swipe ended last frame and its wheel `End` is still owed. It goes
    /// out first thing next frame: egui drops a frame's wheel deltas when
    /// that frame carries the `End`, and the swipe's last move may share the
    /// release frame.
    owe_wheel_end: bool,
}

/// What the pointer-mirrored finger is doing.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
enum Gesture {
    /// No finger down.
    #[default]
    Idle,
    /// The finger is egui's: it started off the editor (or while another
    /// filter held a finger), or it became a selection.
    Untouched(TouchId),
    /// Down on the editor, intent undecided: press and moves withheld.
    Pending {
        id: TouchId,
        start: Pos2,
        /// The finger's latest y, for the first wheel delta.
        last_y: f32,
        /// egui's clock when the long-press window ends.
        deadline: f64,
    },
    /// A swipe: each move scrolls the editor by the finger's travel.
    Scrolling {
        id: TouchId,
        start: Pos2,
        last_y: f32,
    },
    /// The editor went away under the finger (popped, or covered by a
    /// slide): nothing of the touch reaches a widget any more.
    Dropped(TouchId),
}

impl Gesture {
    fn touch(self) -> Option<TouchId> {
        match self {
            Gesture::Idle => None,
            Gesture::Untouched(id) | Gesture::Dropped(id) => Some(id),
            Gesture::Pending { id, .. } | Gesture::Scrolling { id, .. } => Some(id),
        }
    }
}

impl EditorTouch {
    /// Rewrite one frame's events.
    ///
    /// `editor` is where the editor's scroll area was drawn last frame while
    /// the editor may own touches (it is on top and nothing slides it), and
    /// `None` otherwise. `claim` is false while another filter holds a finger
    /// (then a new finger is not the one egui-winit mirrors). `now` is egui's
    /// clock, in seconds.
    pub(crate) fn filter(
        &mut self,
        events: Vec<Event>,
        now: f64,
        editor: Option<Rect>,
        claim: bool,
    ) -> Vec<Event> {
        let mut out = Vec::with_capacity(events.len() + 4);
        if std::mem::take(&mut self.owe_wheel_end) {
            out.push(wheel(TouchPhase::End, 0.0));
        }
        if editor.is_none() {
            match self.gesture {
                Gesture::Pending { id, .. } => self.gesture = Gesture::Dropped(id),
                Gesture::Scrolling { id, .. } => {
                    out.push(wheel(TouchPhase::End, 0.0));
                    self.gesture = Gesture::Dropped(id);
                }
                _ => {}
            }
        }
        // The window ended while the finger stayed inside the slop. This
        // frame may carry no event for the finger at all (the repaint the
        // editor requested brought it), so check before the events.
        if let Gesture::Pending {
            id,
            start,
            deadline,
            ..
        } = self.gesture
        {
            if now >= deadline {
                out.push(Event::PointerMoved(start));
                out.push(button(start, true));
                self.gesture = Gesture::Untouched(id);
            }
        }
        let claimable = editor.filter(|_| claim);
        // egui-winit follows a finger's `Touch End` with its release. Once
        // this filter has answered the lift itself, that release must not
        // reach egui (after a replayed click it would read as a double click).
        let mut swallow_release = false;
        for ev in events {
            match ev {
                Event::Touch { id, phase, pos, .. } => {
                    out.push(ev);
                    if self.on_touch(id, phase, pos, now, claimable, &mut out) {
                        swallow_release = true;
                    }
                }
                Event::PointerMoved(_) | Event::PointerButton { .. } => match self.gesture {
                    Gesture::Idle | Gesture::Untouched(_) => {
                        let release = matches!(ev, Event::PointerButton { pressed: false, .. });
                        if !(release && std::mem::take(&mut swallow_release)) {
                            out.push(ev);
                        }
                    }
                    // The finger's intent decides what egui sees of it.
                    Gesture::Pending { .. } | Gesture::Scrolling { .. } | Gesture::Dropped(_) => {}
                },
                _ => out.push(ev),
            }
        }
        out
    }

    /// Advance the gesture on one touch event, pushing what egui sees in
    /// place of the finger's own pointer events. `claimable` is the editor's
    /// rect when a new finger may be claimed. Returns true when this answered
    /// the finger's lift, so the release egui-winit sends next must not pass.
    fn on_touch(
        &mut self,
        id: TouchId,
        phase: TouchPhase,
        pos: Pos2,
        now: f64,
        claimable: Option<Rect>,
        out: &mut Vec<Event>,
    ) -> bool {
        match (phase, self.gesture) {
            // The first finger down is the one egui-winit mirrors as the
            // pointer; any finger after it is left alone below.
            (TouchPhase::Start, Gesture::Idle) => {
                self.gesture = if claimable.is_some_and(|r| r.contains(pos)) {
                    Gesture::Pending {
                        id,
                        start: pos,
                        last_y: pos.y,
                        deadline: now + LONG_PRESS,
                    }
                } else {
                    Gesture::Untouched(id)
                };
                false
            }
            (
                TouchPhase::Move,
                Gesture::Pending {
                    id: finger,
                    start,
                    last_y,
                    ..
                },
            ) if finger == id => {
                if pos.distance(start) > SCROLL_SLOP {
                    // Out of the slop inside the window: a swipe.
                    out.push(Event::PointerMoved(start));
                    out.push(wheel(TouchPhase::Start, 0.0));
                    out.push(wheel(TouchPhase::Move, pos.y - last_y));
                    self.gesture = Gesture::Scrolling {
                        id,
                        start,
                        last_y: pos.y,
                    };
                } else if let Gesture::Pending { last_y, .. } = &mut self.gesture {
                    *last_y = pos.y;
                }
                false
            }
            (
                TouchPhase::Move,
                Gesture::Scrolling {
                    id: finger,
                    start,
                    last_y,
                },
            ) if finger == id => {
                // The content follows the finger: a finger moving up gives a
                // negative dy, which grows the scroll offset. The hover stays
                // parked on the start, inside the editor's scroll area.
                out.push(Event::PointerMoved(start));
                out.push(wheel(TouchPhase::Move, pos.y - last_y));
                self.gesture = Gesture::Scrolling {
                    id,
                    start,
                    last_y: pos.y,
                };
                false
            }
            (TouchPhase::End | TouchPhase::Cancel, gesture) if gesture.touch() == Some(id) => {
                self.gesture = Gesture::Idle;
                let lifted = phase == TouchPhase::End;
                match gesture {
                    Gesture::Pending { start, .. } if lifted => {
                        // A tap: the whole click at once, where the finger
                        // went down.
                        out.push(Event::PointerMoved(start));
                        out.push(button(start, true));
                        out.push(button(start, false));
                        true
                    }
                    Gesture::Scrolling { .. } => {
                        self.owe_wheel_end = true;
                        lifted
                    }
                    Gesture::Dropped(_) => lifted,
                    // Untouched: egui saw the press, so it gets the release.
                    // Pending but cancelled: nothing was delivered to undo.
                    _ => false,
                }
            }
            // Another finger, or a phase this gesture has no use for.
            _ => false,
        }
    }

    /// While a touch waits out the long-press window: the time left. The
    /// window must end on a frame even when the finger never moves.
    pub(crate) fn wake_after(&self, now: f64) -> Option<Duration> {
        match self.gesture {
            Gesture::Pending { deadline, .. } => {
                Some(Duration::try_from_secs_f64(deadline - now).unwrap_or(Duration::ZERO))
            }
            _ => None,
        }
    }
}

/// A vertical wheel event in points; `dy` > 0 moves the content down.
fn wheel(phase: TouchPhase, dy: f32) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: egui::vec2(0.0, dy),
        phase,
        modifiers: Modifiers::NONE,
    }
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};
    use TouchPhase::{Cancel, End, Move, Start};

    /// The editor's scroll area: below a 56 pt top bar on a 400 x 800 screen.
    const EDITOR: Rect = Rect::from_min_max(pos2(0.0, 56.0), pos2(400.0, 800.0));

    fn touch(id: u64, phase: TouchPhase, pos: Pos2) -> Event {
        Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: TouchId(id),
            phase,
            pos,
            force: None,
        }
    }

    /// egui-winit's events for the first finger down (`on_touch`): the touch
    /// plus the pointer events it mirrors.
    fn primary(phase: TouchPhase, pos: Pos2) -> Vec<Event> {
        let t = touch(0, phase, pos);
        match phase {
            Start => vec![t, Event::PointerMoved(pos), button(pos, true)],
            Move => vec![t, Event::PointerMoved(pos)],
            End => vec![t, button(pos, false), Event::PointerGone],
            Cancel => vec![t, Event::PointerGone],
        }
    }

    /// The editor on top and claimable.
    fn run(f: &mut EditorTouch, events: Vec<Event>, now: f64) -> Vec<Event> {
        f.filter(events, now, Some(EDITOR), true)
    }

    fn buttons(events: &[Event]) -> Vec<(Pos2, bool)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::PointerButton { pos, pressed, .. } => Some((*pos, *pressed)),
                _ => None,
            })
            .collect()
    }

    fn wheels(events: &[Event]) -> Vec<(TouchPhase, f32)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::MouseWheel { delta, phase, .. } => Some((*phase, delta.y)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_tap_is_one_click_where_the_finger_went_down() {
        let mut f = EditorTouch::default();
        let p = pos2(100.0, 300.0);
        let down = run(&mut f, primary(Start, p), 1.0);
        assert_eq!(down, vec![touch(0, Start, p)], "press and move wait");
        // A wobble inside the slop is still a tap.
        let q = p + vec2(3.0, 4.0);
        assert_eq!(run(&mut f, primary(Move, q), 1.03), vec![touch(0, Move, q)]);
        let up = run(&mut f, primary(End, q), 1.1);
        assert_eq!(
            buttons(&up),
            vec![(p, true), (p, false)],
            "one click at the start, egui-winit's own release dropped"
        );
        assert_eq!(up.last(), Some(&Event::PointerGone), "the pointer leaves");
        assert_eq!(f.wake_after(1.1), None);
    }

    #[test]
    fn a_swipe_scrolls_by_the_finger_travel() {
        let mut f = EditorTouch::default();
        let p = pos2(200.0, 600.0);
        run(&mut f, primary(Start, p), 1.0);

        let first = run(&mut f, primary(Move, pos2(200.0, 590.0)), 1.016);
        assert_eq!(
            wheels(&first),
            vec![(Start, 0.0), (Move, -10.0)],
            "past the slop: a trackpad-style scroll; finger up, dy < 0"
        );
        let hovers: Vec<_> = first
            .iter()
            .filter_map(|e| match e {
                Event::PointerMoved(q) => Some(*q),
                _ => None,
            })
            .collect();
        assert_eq!(hovers, vec![p], "the hover is parked on the start");
        assert!(buttons(&first).is_empty());

        // Onto the top bar: the wheel still goes to the editor.
        let off = run(&mut f, primary(Move, pos2(200.0, 20.0)), 1.032);
        assert_eq!(wheels(&off), vec![(Move, -570.0)]);
        assert!(off.contains(&Event::PointerMoved(p)));
        let back = run(&mut f, primary(Move, pos2(200.0, 50.0)), 1.048);
        assert_eq!(wheels(&back), vec![(Move, 30.0)], "finger down, dy > 0");

        let up = run(&mut f, primary(End, pos2(200.0, 50.0)), 1.064);
        assert!(
            buttons(&up).is_empty(),
            "no press was delivered: no release"
        );
        assert!(
            wheels(&up).is_empty(),
            "the End waits: it would drop deltas"
        );
        assert_eq!(up.last(), Some(&Event::PointerGone));
        assert_eq!(wheels(&run(&mut f, vec![], 1.08)), vec![(End, 0.0)]);
        assert!(wheels(&run(&mut f, vec![], 1.096)).is_empty(), "owed once");
    }

    #[test]
    fn a_press_held_still_is_delivered_when_the_window_ends() {
        let mut f = EditorTouch::default();
        let p = pos2(100.0, 300.0);
        run(&mut f, primary(Start, p), 1.0);
        let wait = f.wake_after(1.1).expect("a pending touch needs a frame");
        assert!((wait.as_secs_f64() - (LONG_PRESS - 0.1)).abs() < 1e-6);

        assert!(buttons(&run(&mut f, vec![], 1.0 + LONG_PRESS - 0.01)).is_empty());
        let late = run(&mut f, vec![], 1.0 + LONG_PRESS);
        assert_eq!(
            buttons(&late),
            vec![(p, true)],
            "the withheld press, at the start"
        );
        assert_eq!(f.wake_after(1.5), None, "decided");

        // The finger is egui's from here: its drag selects, as before.
        let drag = primary(Move, p + vec2(0.0, 40.0));
        assert_eq!(run(&mut f, drag.clone(), 1.5), drag);
        let lift = primary(End, p + vec2(0.0, 40.0));
        assert_eq!(run(&mut f, lift.clone(), 1.6), lift);
    }

    #[test]
    fn touches_off_the_editor_and_other_fingers_pass_untouched() {
        let mut f = EditorTouch::default();
        // From the top bar into the editor: egui's from start to end.
        let path = [
            (Start, pos2(200.0, 20.0)),
            (Move, pos2(200.0, 300.0)),
            (End, pos2(200.0, 300.0)),
        ];
        for (k, (phase, pos)) in path.into_iter().enumerate() {
            let sent = primary(phase, pos);
            assert_eq!(run(&mut f, sent.clone(), 1.0 + 0.1 * k as f64), sent);
        }

        // A second finger on the editor is not the pointer: it passes too.
        let sent = primary(Start, pos2(200.0, 20.0));
        assert_eq!(run(&mut f, sent.clone(), 2.0), sent);
        for (phase, pos) in [
            (Start, pos2(200.0, 400.0)),
            (Move, pos2(200.0, 300.0)),
            (End, pos2(200.0, 300.0)),
        ] {
            let sent = vec![touch(1, phase, pos)];
            assert_eq!(run(&mut f, sent.clone(), 2.1), sent);
        }
        let sent = primary(End, pos2(200.0, 20.0));
        assert_eq!(run(&mut f, sent.clone(), 2.2), sent);

        // No editor to claim for (the list is on top), or a claim refused
        // (the edge swipe holds a finger): untouched.
        let p = pos2(200.0, 400.0);
        for (editor, claim) in [(None, true), (Some(EDITOR), false)] {
            let mut f = EditorTouch::default();
            for phase in [Start, End] {
                let sent = primary(phase, p);
                assert_eq!(f.filter(sent.clone(), 3.0, editor, claim), sent);
            }
        }
    }

    #[test]
    fn a_mouse_is_never_filtered() {
        let mut f = EditorTouch::default();
        let p = pos2(200.0, 400.0);
        let q = p + vec2(0.0, 50.0);
        let mouse = vec![
            Event::PointerMoved(p),
            button(p, true),
            Event::PointerMoved(q),
            button(q, false),
            wheel(Move, -3.0),
        ];
        assert_eq!(run(&mut f, mouse.clone(), 1.0), mouse);
        assert_eq!(f.wake_after(1.0), None);
    }

    #[test]
    fn a_touch_outliving_its_editor_reaches_nothing() {
        let p = pos2(200.0, 400.0);
        // The back key pops the editor under a pending finger.
        let mut f = EditorTouch::default();
        run(&mut f, primary(Start, p), 1.0);
        let moved = f.filter(primary(Move, p + vec2(0.0, 50.0)), 1.1, None, true);
        assert_eq!(moved, vec![touch(0, Move, p + vec2(0.0, 50.0))]);
        let up = f.filter(primary(End, p + vec2(0.0, 50.0)), 1.2, None, true);
        assert!(buttons(&up).is_empty(), "no click on whatever is there now");
        assert_eq!(up.last(), Some(&Event::PointerGone));

        // Mid-swipe, the scroll ends at once and moves no further.
        let mut f = EditorTouch::default();
        run(&mut f, primary(Start, p), 2.0);
        run(&mut f, primary(Move, p - vec2(0.0, 30.0)), 2.016);
        let gone = f.filter(primary(Move, p - vec2(0.0, 60.0)), 2.032, None, true);
        assert_eq!(wheels(&gone), vec![(End, 0.0)]);
    }

    #[test]
    fn a_cancelled_touch_delivers_nothing() {
        let mut f = EditorTouch::default();
        let p = pos2(200.0, 400.0);
        run(&mut f, primary(Start, p), 1.0);
        let cancel = primary(Cancel, p);
        assert_eq!(run(&mut f, cancel.clone(), 1.1), cancel, "no click");
        assert_eq!(f.wake_after(1.1), None);
        let after = run(&mut f, vec![], 1.0 + LONG_PRESS);
        assert!(buttons(&after).is_empty(), "and no late press");
    }
}
