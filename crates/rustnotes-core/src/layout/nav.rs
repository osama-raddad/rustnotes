use egui::{vec2, Sense, Ui, UiBuilder};

/// Stack-based navigation with a shared-axis slide between routes.
///
/// A transition always has a *front* and a *back* screen. Pushing: the new
/// route is the front and slides in from the right while the old one drifts
/// left (parallax). Popping: the leaving route is the front and slides out
/// to the right, revealing the one underneath. `q` runs 0..1 over the move.
///
/// The stack changes immediately; the transition only controls what is drawn.
/// An interactive edge swipe drives `q` by hand ([`Nav::begin_drag`]).
///
/// Every route instance carries a stable identity for its whole life, so it
/// keeps one id salt — and therefore one egui state namespace — in every
/// position it is drawn in: idle top, incoming front, drifting back,
/// mid-drag. Widget state (a list's scroll offset) survives being navigated
/// over and back.
pub struct Nav<R> {
    stack: Vec<R>,
    /// Identity per route, parallel to `stack`.
    ids: Vec<u64>,
    next_id: u64,
    tr: Option<Transition<R>>,
}

struct Transition<R> {
    front: R,
    front_id: u64,
    back: R,
    back_id: u64,
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
            ids: vec![0],
            next_id: 1,
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

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Starting `q` for a transition that replaces a running one. `q` measures
    /// progress toward each direction's own end state, so a push at `q` draws
    /// exactly what a pop at `1 - q` draws: reversing the slide resumes from
    /// the mirrored progress instead of snapping to a rail. Continuing in the
    /// same direction cannot reuse the picture (the pair of routes changes),
    /// so it restarts from the beginning.
    fn flip_seed(&self, push: bool) -> f32 {
        match self.tr.as_ref() {
            Some(t) if t.push != push => 1.0 - t.q,
            _ => 0.0,
        }
    }

    /// Push `route`. `animate` is false when the layout has no screen-to-screen
    /// motion (two-pane) or the user asked for reduced motion.
    pub fn push(&mut self, route: R, now: f64, animate: bool) {
        let back = self.top().clone();
        let back_id = *self.ids.last().expect("nav stack is never empty");
        let id = self.alloc_id();
        let from = self.flip_seed(true);
        self.stack.push(route.clone());
        self.ids.push(id);
        self.tr = animate.then(|| Transition {
            front: route,
            front_id: id,
            back,
            back_id,
            push: true,
            q: from,
            anim: Some(Anim {
                from,
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
        let from = self.flip_seed(false);
        let leaving = self.stack.pop().expect("checked");
        let leaving_id = self.ids.pop().expect("checked");
        let revealed = self.top().clone();
        let revealed_id = *self.ids.last().expect("nav stack is never empty");
        self.tr = animate.then(|| Transition {
            front: leaving,
            front_id: leaving_id,
            back: revealed,
            back_id: revealed_id,
            push: false,
            q: from,
            anim: Some(Anim {
                from,
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
            front_id: self.ids[n - 1],
            back: self.stack[n - 2].clone(),
            back_id: self.ids[n - 2],
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
    /// Non-finite input reads as "no drag yet" rather than poisoning every
    /// rect derived from `q`.
    pub fn drag_to(&mut self, q: f32) {
        if let Some(t) = self.tr.as_mut().filter(|t| t.dragging) {
            t.q = if q.is_finite() {
                q.clamp(0.0, 1.0)
            } else {
                0.0
            };
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
            self.ids.pop();
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
    /// possibly offset horizontally. Each route is salted by its own identity,
    /// so a route's widget state survives appearing as the idle top, the
    /// incoming front, or the drifting back of a slide.
    ///
    /// While a transition runs, the front route owns all pointer input; the
    /// route behind it is painted but cannot be hit-tested.
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
            let salt = *self.ids.last().expect("nav stack is never empty");
            let mut ui = root.new_child(UiBuilder::new().id_salt(salt).max_rect(rect));
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
        let (front, back) = (t.front.clone(), t.back.clone());

        let mut back_ui = root.new_child(
            UiBuilder::new()
                .id_salt(t.back_id)
                .max_rect(rect.translate(vec2(back_dx, 0.0))),
        );
        draw(&mut back_ui, &back);
        let mut front_ui = root.new_child(
            UiBuilder::new()
                .id_salt(t.front_id)
                .max_rect(rect.translate(vec2(front_dx, 0.0))),
        );
        // An invisible full-viewport sensor: registered after everything the
        // back route drew but before the front route's own widgets, it wins
        // the hit-test against the back route everywhere while the front
        // route's widgets still win on top of it. Without it, a tap mid-slide
        // could reach a widget the user cannot see.
        let _ = front_ui.interact(
            rect,
            front_ui.id().with("input_block"),
            Sense::click_and_drag(),
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

    // --- headless frames ---------------------------------------------------
    //
    // `Nav::show` is driven through `Context::run_ui` with synthetic input,
    // the same seam the host uses, capturing the UI each route was handed.

    use egui::{Pos2, RawInput, Rect};

    fn raw(t: f64) -> RawInput {
        RawInput {
            time: Some(t),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 800.0))),
            ..Default::default()
        }
    }

    /// What one route was handed by `show` this frame.
    struct Drawn {
        route: &'static str,
        id: egui::Id,
        rect: Rect,
    }

    /// Run one headless frame of `show`, recording each route's UI.
    fn frame(ctx: &egui::Context, nav: &mut Nav<&'static str>, t: f64, seen: &mut Vec<Drawn>) {
        seen.clear();
        ctx.run_ui(raw(t), |root| {
            nav.show(root, t, |ui, route| {
                seen.push(Drawn {
                    route,
                    id: ui.id(),
                    rect: ui.max_rect(),
                });
            });
        })
        .drop_without_applying_deltas();
    }

    fn drawn<'a>(seen: &'a [Drawn], route: &str) -> &'a Drawn {
        seen.iter().find(|d| d.route == route).expect("route drawn")
    }

    /// A route keeps one id salt — one egui state namespace — whether it is
    /// the idle top, the incoming front, or the back of a slide.
    #[test]
    fn routes_keep_one_namespace_across_push_and_pop() {
        let ctx = egui::Context::default();
        let mut nav = Nav::new("list");
        let mut seen = vec![];

        frame(&ctx, &mut nav, 0.0, &mut seen);
        assert_eq!(seen.len(), 1);
        let list_id = drawn(&seen, "list").id;

        nav.push("editor", 0.0, true);
        frame(&ctx, &mut nav, 0.0, &mut seen);
        assert_eq!(seen.len(), 2, "both routes drawn mid-push");
        assert_eq!(drawn(&seen, "list").id, list_id);
        let editor_id = drawn(&seen, "editor").id;
        assert_ne!(editor_id, list_id);

        // Past the duration the push completes and the editor idles on the
        // namespace it arrived with.
        frame(&ctx, &mut nav, DURATION + 0.01, &mut seen);
        assert!(!nav.is_animating());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].route, "editor");
        assert_eq!(seen[0].id, editor_id);

        nav.pop(DURATION + 0.02, true);
        frame(&ctx, &mut nav, DURATION + 0.02, &mut seen);
        assert_eq!(seen.len(), 2, "both routes drawn mid-pop");
        assert_eq!(drawn(&seen, "list").id, list_id);
        assert_eq!(drawn(&seen, "editor").id, editor_id);

        frame(&ctx, &mut nav, 2.0 * DURATION + 0.03, &mut seen);
        assert!(!nav.is_animating());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].route, "list");
        assert_eq!(seen[0].id, list_id);
    }

    /// The slide brings the front in from the right (push) and returns it to
    /// the right (pop), settling exactly on the viewport.
    #[test]
    fn transition_geometry_advances_and_settles() {
        let ctx = egui::Context::default();
        let mut nav = Nav::new("list");
        let mut seen = vec![];

        nav.push("editor", 0.0, true);
        frame(&ctx, &mut nav, 0.0, &mut seen);
        assert_eq!(
            drawn(&seen, "editor").rect.min.x,
            400.0,
            "push starts off-screen"
        );
        assert_eq!(drawn(&seen, "list").rect.min.x, 0.0, "back starts at rest");

        // Halfway, the eased push has covered 1 - (1/2)³ = 87.5% of the width.
        frame(&ctx, &mut nav, DURATION * 0.5, &mut seen);
        assert!((drawn(&seen, "editor").rect.min.x - 50.0).abs() < 0.01);
        assert!(
            (drawn(&seen, "list").rect.min.x + 105.0).abs() < 0.01,
            "parallax drift"
        );

        frame(&ctx, &mut nav, DURATION + 0.01, &mut seen);
        assert!(!nav.is_animating());
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].rect,
            Rect::from_min_size(Pos2::ZERO, vec2(400.0, 800.0))
        );
    }

    /// Popping in the middle of a push reverses from the picture on screen:
    /// the drawn rects are continuous and nothing snaps to a rail.
    #[test]
    fn reversing_a_push_mid_slide_is_continuous() {
        let ctx = egui::Context::default();
        let mut nav = Nav::new("list");
        let mut seen = vec![];

        nav.push("editor", 0.0, true);
        frame(&ctx, &mut nav, DURATION * 0.5, &mut seen);
        let editor_before = drawn(&seen, "editor").rect.min.x;
        let list_before = drawn(&seen, "list").rect.min.x;
        assert!(editor_before > 0.0 && editor_before < 400.0, "mid-slide");

        nav.pop(DURATION * 0.5, true);
        frame(&ctx, &mut nav, DURATION * 0.5, &mut seen);
        let editor_after = drawn(&seen, "editor").rect.min.x;
        let list_after = drawn(&seen, "list").rect.min.x;
        assert!(
            (editor_before - editor_after).abs() < 1e-3,
            "front must not jump: {editor_before} -> {editor_after}"
        );
        assert!(
            (list_before - list_after).abs() < 1e-3,
            "back must not jump: {list_before} -> {list_after}"
        );
        assert!(
            editor_after > 0.0 && editor_after < 400.0,
            "no snap to a rail"
        );

        // The reversal still finishes, landing back on the list.
        frame(&ctx, &mut nav, 1.5 * DURATION + 0.01, &mut seen);
        assert!(!nav.is_animating());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].route, "list");
    }

    /// A push onto a running push cannot reuse the picture (the pair of
    /// routes changed), so the new front restarts from off-screen.
    #[test]
    fn push_during_push_restarts_for_the_new_front() {
        let ctx = egui::Context::default();
        let mut nav = Nav::new("list");
        let mut seen = vec![];

        nav.push("editor", 0.0, true);
        frame(&ctx, &mut nav, DURATION * 0.5, &mut seen);
        nav.push("detail", DURATION * 0.5, true);
        frame(&ctx, &mut nav, DURATION * 0.5, &mut seen);
        assert_eq!(
            drawn(&seen, "detail").rect.min.x,
            400.0,
            "the new front starts off-screen"
        );
        assert_eq!(
            drawn(&seen, "editor").rect.min.x,
            0.0,
            "old front rests as back"
        );

        frame(&ctx, &mut nav, 1.5 * DURATION + 0.01, &mut seen);
        assert!(!nav.is_animating());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].route, "detail");
    }

    /// An interactive drag keeps the same namespaces as the routes it shows.
    #[test]
    fn drag_keeps_route_namespaces() {
        let ctx = egui::Context::default();
        let mut nav = Nav::new("list");
        let mut seen = vec![];

        frame(&ctx, &mut nav, 0.0, &mut seen);
        let list_id = drawn(&seen, "list").id;

        nav.push("editor", 0.0, false);
        frame(&ctx, &mut nav, 0.1, &mut seen);
        let editor_id = drawn(&seen, "editor").id;

        assert!(nav.begin_drag());
        nav.drag_to(0.4);
        frame(&ctx, &mut nav, 0.2, &mut seen);
        assert_eq!(drawn(&seen, "editor").id, editor_id);
        assert_eq!(drawn(&seen, "list").id, list_id);
        assert!(
            (drawn(&seen, "editor").rect.min.x - 160.0).abs() < 0.01,
            "40% out"
        );
    }

    /// `drag_to` clamps into 0..1 and reads non-finite input as "no drag",
    /// so nothing it feeds can end up with NaN vertices.
    #[test]
    fn drag_to_clamps_and_rejects_non_finite() {
        let ctx = egui::Context::default();
        let mut nav = Nav::new("list");
        let mut seen = vec![];

        nav.push("editor", 0.0, false);
        assert!(nav.begin_drag());

        nav.drag_to(f32::NAN);
        frame(&ctx, &mut nav, 1.0, &mut seen);
        assert_eq!(
            drawn(&seen, "editor").rect.min.x,
            0.0,
            "NaN reads as no drag"
        );
        assert!(drawn(&seen, "list").rect.min.x.is_finite());

        nav.drag_to(-0.5);
        frame(&ctx, &mut nav, 1.1, &mut seen);
        assert_eq!(drawn(&seen, "editor").rect.min.x, 0.0, "clamped at 0");
        assert!((drawn(&seen, "list").rect.min.x + 120.0).abs() < 0.01);

        nav.drag_to(1.5);
        frame(&ctx, &mut nav, 1.2, &mut seen);
        assert_eq!(drawn(&seen, "editor").rect.min.x, 400.0, "clamped at 1");
        assert_eq!(drawn(&seen, "list").rect.min.x, 0.0);
    }
}
