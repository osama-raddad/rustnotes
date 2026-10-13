use egui::{vec2, Ui, UiBuilder};

/// Stack-based navigation with a shared-axis slide between routes.
///
/// A transition always has a *front* and a *back* screen. Pushing: the new
/// route is the front and slides in from the right while the old one drifts
/// left (parallax). Popping: the leaving route is the front and slides out
/// to the right, revealing the one underneath. `q` runs 0..1 over the move.
///
/// The stack changes immediately; the transition only controls what is drawn.
/// An interactive edge swipe drives `q` by hand ([`Nav::begin_drag`]).
pub struct Nav<R> {
    stack: Vec<R>,
    tr: Option<Transition<R>>,
}

struct Transition<R> {
    front: R,
    back: R,
    push: bool,
    q: f32,
    anim: Option<Anim>,
    dragging: bool,
}

struct Anim {
    from: f32,
    to: f32,
    t0: f64,
}

/// Slide duration in seconds.
const DURATION: f64 = 0.25;
/// How far the back screen drifts, as a fraction of the width.
const PARALLAX: f32 = 0.3;

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

impl<R: Clone> Nav<R> {
    pub fn new(root: R) -> Self {
        Nav {
            stack: vec![root],
            tr: None,
        }
    }

    pub fn top(&self) -> &R {
        self.stack.last().expect("nav stack is never empty")
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }

    pub fn is_animating(&self) -> bool {
        self.tr.is_some()
    }

    /// Push `route`. `animate` is false when the layout has no screen-to-screen
    /// motion (two-pane) or the user asked for reduced motion.
    pub fn push(&mut self, route: R, now: f64, animate: bool) {
        let back = self.top().clone();
        self.stack.push(route.clone());
        self.tr = animate.then(|| Transition {
            front: route,
            back,
            push: true,
            q: 0.0,
            anim: Some(Anim {
                from: 0.0,
                to: 1.0,
                t0: now,
            }),
            dragging: false,
        });
    }

    /// Pop the top route. Returns false at the root.
    pub fn pop(&mut self, now: f64, animate: bool) -> bool {
        if !self.can_pop() {
            return false;
        }
        let leaving = self.stack.pop().expect("checked");
        let revealed = self.top().clone();
        self.tr = animate.then(|| Transition {
            front: leaving,
            back: revealed,
            push: false,
            q: 0.0,
            anim: Some(Anim {
                from: 0.0,
                to: 1.0,
                t0: now,
            }),
            dragging: false,
        });
        true
    }

    /// Drop any running slide (the layout has no screen-to-screen motion right
    /// now). An interactive drag in progress is left alone.
    pub fn settle(&mut self) {
        if !self.is_dragging() {
            self.tr = None;
        }
    }

    /// Start an interactive pop (edge swipe). False if there is nothing to pop
    /// or a transition is already running.
    pub fn begin_drag(&mut self) -> bool {
        if !self.can_pop() || self.tr.is_some() {
            return false;
        }
        let n = self.stack.len();
        self.tr = Some(Transition {
            front: self.stack[n - 1].clone(),
            back: self.stack[n - 2].clone(),
            push: false,
            q: 0.0,
            anim: None,
            dragging: true,
        });
        true
    }

    pub fn is_dragging(&self) -> bool {
        self.tr.as_ref().is_some_and(|t| t.dragging)
    }

    /// Follow the finger: `q` is the fraction of the width dragged.
    pub fn drag_to(&mut self, q: f32) {
        if let Some(t) = self.tr.as_mut().filter(|t| t.dragging) {
            t.q = q.clamp(0.0, 1.0);
        }
    }

    /// Release. `commit` pops the route and finishes the slide; otherwise the
    /// front screen springs back and the stack is untouched.
    pub fn end_drag(&mut self, commit: bool, now: f64) {
        let Some(t) = self.tr.as_mut().filter(|t| t.dragging) else {
            return;
        };
        t.dragging = false;
        let from = t.q;
        if commit {
            t.anim = Some(Anim {
                from,
                to: 1.0,
                t0: now,
            });
            self.stack.pop();
        } else {
            t.anim = Some(Anim {
                from,
                to: 0.0,
                t0: now,
            });
        }
    }

    /// Draw the current route, or both routes mid-transition.
    ///
    /// `draw` renders one route into a UI whose rect covers the viewport,
    /// possibly offset horizontally. The idle route and the incoming route
    /// share the id salt `"nav_top"`, so their scroll state survives.
    pub fn show(&mut self, root: &mut Ui, now: f64, mut draw: impl FnMut(&mut Ui, &R)) {
        let ctx = root.ctx().clone();
        // Advance the animation.
        if let Some(t) = self.tr.as_mut() {
            if let Some(a) = &t.anim {
                let p = (((now - a.t0) / DURATION) as f32).clamp(0.0, 1.0);
                t.q = a.from + (a.to - a.from) * ease_out_cubic(p);
                if p >= 1.0 {
                    self.tr = None;
                }
            }
        }
        let Some(t) = self.tr.as_ref() else {
            let rect = root.max_rect();
            let mut ui = root.new_child(UiBuilder::new().id_salt("nav_top").max_rect(rect));
            let route = self.top().clone();
            draw(&mut ui, &route);
            return;
        };
        ctx.request_repaint();

        let rect = root.max_rect();
        let w = rect.width();
        let (front_dx, back_dx) = if t.push {
            ((1.0 - t.q) * w, -PARALLAX * w * t.q)
        } else {
            (t.q * w, -PARALLAX * w * (1.0 - t.q))
        };
        // The route that ends up on top keeps the stable salt.
        let (front_salt, back_salt) = if t.push || t.dragging {
            ("nav_top", "nav_prev")
        } else {
            ("nav_prev", "nav_top")
        };
        let (front, back) = (t.front.clone(), t.back.clone());

        let mut back_ui = root.new_child(
            UiBuilder::new()
                .id_salt(back_salt)
                .max_rect(rect.translate(vec2(back_dx, 0.0))),
        );
        draw(&mut back_ui, &back);
        let mut front_ui = root.new_child(
            UiBuilder::new()
                .id_salt(front_salt)
                .max_rect(rect.translate(vec2(front_dx, 0.0))),
        );
        draw(&mut front_ui, &front);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_track_the_stack() {
        let mut n = Nav::new("list");
        assert!(!n.can_pop());
        n.push("editor", 0.0, true);
        assert_eq!(*n.top(), "editor");
        assert!(n.is_animating());
        assert!(n.pop(1.0, false));
        assert_eq!(*n.top(), "list");
        assert!(!n.is_animating());
        assert!(!n.pop(1.0, true), "cannot pop the root");
    }

    #[test]
    fn cancelled_drag_leaves_the_stack_alone() {
        let mut n = Nav::new("list");
        n.push("editor", 0.0, false);
        assert!(n.begin_drag());
        n.drag_to(0.2);
        n.end_drag(false, 0.0);
        assert_eq!(*n.top(), "editor");
    }

    #[test]
    fn committed_drag_pops() {
        let mut n = Nav::new("list");
        n.push("editor", 0.0, false);
        assert!(n.begin_drag());
        n.drag_to(0.6);
        n.end_drag(true, 0.0);
        assert_eq!(*n.top(), "list");
    }

    #[test]
    fn cannot_drag_at_the_root() {
        let mut n = Nav::new("list");
        assert!(!n.begin_drag());
    }
}
