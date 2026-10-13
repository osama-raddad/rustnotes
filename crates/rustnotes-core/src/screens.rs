//! Screens: the note list, the editor, a settings page, and the two-pane
//! (list–detail) arrangement used when the window is wide and tall enough.
//!
//! Every screen renders into a [`Scaffold`]; sizes come from [`Metrics`] and
//! colours from [`Palette`], so nothing here carries a literal colour or a
//! hand-rolled safe-area calculation. Screens never change navigation while
//! drawing: they queue a [`Cmd`] that `App::ui` applies afterwards.

use crate::layout::{
    self, add_button, back_button, bar_title, bottom_nav, icon_notes, icon_settings, BarLine,
    Metrics, Nav, NavItem, Palette, Scaffold, WindowClass,
};
use crate::{notes, util, App, Screen, Tab, DELETE_ARM_WINDOW, EDITOR_ID};
use egui::{
    pos2, vec2, Align, Align2, Id, Layout, Rect, ScrollArea, Sense, Shape, Stroke, Ui, UiBuilder,
};
use std::time::Instant;

/// Pull distance, in points, past which releasing triggers a refresh.
const PULL_THRESHOLD: f32 = 64.0;
/// Fraction of the card width a left swipe must pass to delete on release.
const SWIPE_DELETE_FRACTION: f32 = 0.4;
/// Movement, in points, before a touch commits to scrolling or swiping.
const AXIS_LOCK_SLOP: f32 = 8.0;

/// Navigation changes requested while drawing.
pub(crate) enum Cmd {
    NewNote,
    OpenNote(i64),
    Back,
    Tab(Tab),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    Undecided,
    Horizontal,
    Vertical,
}

/// A touch that started on a list row.
pub(crate) struct Swipe {
    id: i64,
    axis: Axis,
    dx: f32,
}

enum ListAction {
    Open(i64),
    Delete(usize),
}

const TABS: [NavItem; 2] = [
    NavItem {
        label: "Notes",
        icon: icon_notes,
    },
    NavItem {
        label: "Settings",
        icon: icon_settings,
    },
];

fn tab_bar(ui: &mut Ui, selected: Tab, pal: &Palette, m: &Metrics) -> Option<Tab> {
    let idx = if selected == Tab::Notes { 0 } else { 1 };
    bottom_nav(ui, &TABS, idx, pal, m).map(|i| if i == 0 { Tab::Notes } else { Tab::Settings })
}

/// Contents of the list's top bar. Returns whether "+" was tapped.
fn list_bar(ui: &mut Ui, count: usize, pal: &Palette, m: &Metrics) -> bool {
    let title = bar_title(ui, m.sp(3), "Notes", pal, m);
    let label = match count {
        0 => "no notes".to_string(),
        1 => "1 note".to_string(),
        n => format!("{n} notes"),
    };
    let y = ui.max_rect().center().y;
    ui.painter().text(
        pos2(title.right() + m.sp(2), y),
        Align2::LEFT_CENTER,
        label,
        m.font(m.text.caption),
        pal.text_weak,
    );
    add_button(ui, pal, m).clicked()
}

/// Contents of the editor's top bar. Returns whether back was tapped.
fn editor_bar(ui: &mut Ui, new_note: bool, show_back: bool, pal: &Palette, m: &Metrics) -> bool {
    let title = if new_note { "New note" } else { "Edit note" };
    if show_back {
        let back = back_button(ui, pal, m);
        bar_title(ui, 56.0, title, pal, m);
        back.clicked()
    } else {
        bar_title(ui, m.sp(3), title, pal, m);
        false
    }
}

impl App {
    /// Root of the UI for one frame. The host calls this from `eframe::App::ui`.
    pub fn ui(&mut self, root: &mut Ui) {
        let ctx = root.ctx().clone();
        self.time = ctx.input(|i| i.time);
        let pal = Palette::of(&ctx);
        let m = Metrics::DEFAULT;
        let class = WindowClass::of(&ctx);
        self.two_pane = class.two_pane_ok();
        let now = ctx.input(|i| i.time);

        match self.tab {
            Tab::Settings => {
                self.nav.settle();
                self.settings_screen(root, &pal, &m, class);
            }
            Tab::Notes if self.two_pane => {
                self.nav.settle();
                self.two_pane(root, &pal, &m);
            }
            Tab::Notes => {
                // The stack is borrowed while it draws; routes queue commands
                // instead of touching it.
                let mut nav = std::mem::replace(&mut self.nav, Nav::new(Screen::List));
                nav.show(root, now, |ui, route| self.route_ui(ui, *route, &pal, &m));
                self.nav = nav;
            }
        }

        for cmd in std::mem::take(&mut self.cmds) {
            match cmd {
                Cmd::NewNote => {
                    self.armed_delete = None;
                    self.open_editor(0);
                }
                Cmd::OpenNote(id) => {
                    self.armed_delete = None;
                    self.open_editor(id);
                }
                Cmd::Back => self.close_editor(),
                Cmd::Tab(t) => {
                    self.tab = t;
                }
            }
        }
    }

    /// One route of the single-pane notes flow.
    fn route_ui(&mut self, ui: &mut Ui, route: Screen, pal: &Palette, m: &Metrics) {
        match route {
            Screen::List => {
                let count = self.notes.len();
                let mut add = false;
                let mut tab = None;
                Scaffold::new(pal, m)
                    .limit_body_width()
                    .top_bar(|u| add = list_bar(u, count, pal, m))
                    .bottom_bar(|u| tab = tab_bar(u, Tab::Notes, pal, m))
                    .show(ui, |ui, _| self.list_body(ui, pal, m, None));
                if add {
                    self.cmds.push(Cmd::NewNote);
                }
                if let Some(t) = tab {
                    self.cmds.push(Cmd::Tab(t));
                }
            }
            Screen::Editor => {
                let new_note = self.edit_id == 0;
                let mut back = false;
                Scaffold::new(pal, m)
                    .background(pal.surface)
                    .limit_body_width()
                    .top_bar(|u| back = editor_bar(u, new_note, true, pal, m))
                    .show(ui, |ui, _| self.editor_body(ui, pal, m));
                if back {
                    self.cmds.push(Cmd::Back);
                }
            }
        }
    }

    /// List on the left, editor (or a placeholder) on the right.
    fn two_pane(&mut self, root: &mut Ui, pal: &Palette, m: &Metrics) {
        let ctx = root.ctx().clone();
        let full = ctx.viewport_rect();
        let inset_top = ctx.content_rect().top() - full.top();
        let count = self.notes.len();
        let detail_open = self.screen() == Screen::Editor;
        let selected = (detail_open && self.edit_id != 0).then_some(self.edit_id);
        let new_note = self.edit_id == 0;
        let mut add = false;
        let mut tab = None;

        Scaffold::new(pal, m)
            .bottom_bar(|u| tab = tab_bar(u, Tab::Notes, pal, m))
            .show(root, |ui, _| {
                // The scaffold clips its body; the bars extend into the
                // status-bar strip above it, so open the clip to the viewport.
                ui.set_clip_rect(full);
                let r = ui.max_rect();
                let list_w = (r.width() * 0.36).clamp(320.0, 420.0).min(r.width());
                let left = Rect::from_min_max(r.min, pos2(r.min.x + list_w, r.max.y));
                let right = Rect::from_min_max(pos2(left.right(), r.min.y), r.max);

                // Detail surface, edge to edge behind the right pane.
                ui.painter().rect_filled(
                    Rect::from_min_max(pos2(right.left(), full.top()), pos2(full.max.x, r.max.y)),
                    0.0,
                    pal.surface,
                );

                // List pane.
                let list_bar_rect =
                    Rect::from_min_size(left.min, vec2(left.width(), m.bar_height));
                layout::bar(ui, list_bar_rect, inset_top, 0.0, BarLine::Bottom, pal, |u| {
                    add = list_bar(u, count, pal, m)
                });
                let list_rect =
                    Rect::from_min_max(pos2(left.min.x, list_bar_rect.bottom()), left.max);
                let mut list_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(list_rect)
                        .layout(Layout::top_down(Align::Min)),
                );
                list_ui.set_clip_rect(list_rect);
                self.list_body(&mut list_ui, pal, m, selected);

                ui.painter().line_segment(
                    [pos2(left.right(), full.top()), pos2(left.right(), r.max.y)],
                    Stroke::new(1.0, pal.hairline),
                );

                // Detail pane.
                let detail_bar = Rect::from_min_size(right.min, vec2(right.width(), m.bar_height));
                layout::bar(ui, detail_bar, inset_top, 0.0, BarLine::Bottom, pal, |u| {
                    if detail_open {
                        editor_bar(u, new_note, false, pal, m);
                    }
                });
                let detail_rect =
                    Rect::from_min_max(pos2(right.left(), detail_bar.bottom()), right.max);
                if detail_open {
                    let mut detail_ui = ui.new_child(
                        UiBuilder::new()
                            .max_rect(detail_rect)
                            .layout(Layout::top_down(Align::Min)),
                    );
                    detail_ui.set_clip_rect(detail_rect);
                    self.editor_body(&mut detail_ui, pal, m);
                } else {
                    ui.painter().text(
                        detail_rect.center(),
                        Align2::CENTER_CENTER,
                        "Select a note or tap +",
                        m.font(m.text.body_large),
                        pal.text_faint,
                    );
                }
            });

        if add {
            self.cmds.push(Cmd::NewNote);
        }
        if let Some(t) = tab {
            self.cmds.push(Cmd::Tab(t));
        }
    }

    /// Read-only diagnostics: what the layout layer currently sees.
    fn settings_screen(&mut self, root: &mut Ui, pal: &Palette, m: &Metrics, class: WindowClass) {
        let mut tab = None;
        let ctx = root.ctx().clone();
        let vp = ctx.viewport_rect();
        let rows: [(&str, String); 7] = [
            (
                "Display scale",
                format!(
                    "{:.2} px per point",
                    ctx.native_pixels_per_point().unwrap_or(1.0)
                ),
            ),
            ("Text scale", format!("{:.2}x", ctx.zoom_factor())),
            ("Window", format!("{:.0} x {:.0} pt", vp.width(), vp.height())),
            (
                "Size class",
                format!("{:?} width, {:?} height", class.width, class.height),
            ),
            (
                "Layout",
                if class.two_pane_ok() { "two-pane" } else { "single-pane" }.to_string(),
            ),
            (
                "Appearance",
                format!("{:?}", ctx.theme()),
            ),
            ("Version", env!("CARGO_PKG_VERSION").to_string()),
        ];
        Scaffold::new(pal, m)
            .limit_body_width()
            .top_bar(|u| {
                bar_title(u, m.sp(3), "Settings", pal, m);
            })
            .bottom_bar(|u| tab = tab_bar(u, Tab::Settings, pal, m))
            .show(root, |ui, _| {
                ui.add_space(m.sp(2));
                for (label, value) in &rows {
                    let (row, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), m.touch_target + 8.0),
                        Sense::hover(),
                    );
                    let p = ui.painter();
                    p.text(
                        pos2(row.left() + m.sp(3), row.center().y),
                        Align2::LEFT_CENTER,
                        label,
                        m.font(m.text.body_large),
                        pal.text,
                    );
                    p.text(
                        pos2(row.right() - m.sp(3), row.center().y),
                        Align2::RIGHT_CENTER,
                        value,
                        m.font(m.text.body),
                        pal.text_weak,
                    );
                    p.line_segment(
                        [
                            pos2(row.left() + m.sp(3), row.bottom()),
                            pos2(row.right(), row.bottom()),
                        ],
                        Stroke::new(1.0, pal.hairline),
                    );
                }
            });
        if let Some(t) = tab {
            self.cmds.push(Cmd::Tab(t));
        }
    }

    fn delete_note(&mut self, id: i64) {
        self.db.delete(id);
        if self.screen() == Screen::Editor && self.edit_id == id {
            // Two-pane: the open note was deleted.
            let now = self.now();
            self.nav.pop(now, false);
            self.buffer.clear();
            self.edit_id = 0;
        }
        self.notes = self.db.list();
    }

    fn list_body(&mut self, ui: &mut Ui, pal: &Palette, m: &Metrics, selected: Option<i64>) {
        let ctx = ui.ctx().clone();
        let area = ui.max_rect();

        // Armed delete expires after 2 s even without input.
        if let Some((_, t)) = self.armed_delete {
            let elapsed = t.elapsed();
            if elapsed > DELETE_ARM_WINDOW {
                self.armed_delete = None;
            } else {
                ctx.request_repaint_after(DELETE_ARM_WINDOW - elapsed);
            }
        }

        let (down, origin, pos, dy, dragging) = ctx.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.press_origin(),
                i.pointer.latest_pos(),
                i.pointer.delta().y,
                i.pointer.is_decidedly_dragging(),
            )
        });
        let swipe_horizontal = matches!(&self.swipe, Some(s) if s.axis == Axis::Horizontal);

        // ---- pull to refresh ----------------------------------------------
        // egui scroll areas never overscroll, so watch the pointer: a drag that
        // starts with the list at the top and moves down stretches a gap above
        // it (with resistance); releasing past the threshold reloads.
        let in_area = origin.is_some_and(|o| area.contains(o));
        if down && in_area && !swipe_horizontal {
            if !self.pull_active {
                self.pull_active = true;
                self.pull_armed = self.list_offset <= 0.5;
                self.pull_raw = 0.0;
            }
            if self.pull_armed && dragging {
                self.pull_raw = (self.pull_raw + dy * 0.5).clamp(0.0, 120.0);
                if self.pull_raw == 0.0 && dy < 0.0 {
                    self.pull_armed = false; // dragging up: a normal scroll
                }
            }
        } else if self.pull_active {
            self.pull_active = false;
            if self.pull_raw >= PULL_THRESHOLD {
                log::info!("pull to refresh");
                self.notes = self.db.list();
            }
            self.pull_raw = 0.0;
        }
        let live = self.pull_active && self.pull_raw > 0.0;
        let pull = ctx.animate_value_with_time(
            Id::new("layout.pull"),
            self.pull_raw,
            if live { 0.0 } else { 0.15 },
        );
        if pull > 1.0 {
            let progress = (pull / PULL_THRESHOLD).min(1.0);
            let c = pos2(area.center().x, area.top() + pull / 2.0);
            let sweep = std::f32::consts::TAU * 0.75 * progress;
            let start = -std::f32::consts::FRAC_PI_2 + progress * 3.0;
            let pts: Vec<_> = (0..=24)
                .map(|k| {
                    let a = start + sweep * k as f32 / 24.0;
                    c + vec2(a.cos(), a.sin()) * 10.0
                })
                .collect();
            let color = if progress >= 1.0 { pal.accent } else { pal.text_faint };
            ui.painter().add(Shape::line(pts, Stroke::new(2.5, color)));
            ui.add_space(pull);
        }

        let mut scroll = ScrollArea::vertical()
            .id_salt("list")
            .auto_shrink(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if swipe_horizontal {
            scroll = scroll.scroll_source(egui::scroll_area::ScrollSource {
                drag: egui::scroll_area::DragScroll::Never,
                scroll_bar: false,
                mouse_wheel: false,
            });
        }
        if std::mem::take(&mut self.reset_list_scroll) || pull > 0.0 || self.pull_raw > 0.0 {
            scroll = scroll.vertical_scroll_offset(0.0);
        }

        let notes = &self.notes;
        let armed_delete = self.armed_delete;
        let mut swipe = self.swipe.take();
        let mut action: Option<ListAction> = None;
        let mut swipe_delete: Option<i64> = None;
        let out = scroll.show(ui, |ui| {
            let view = ui.available_rect_before_wrap();
            ui.add_space(m.sp(2));
            let w = ui.available_width();
            let card_m = m.sp(2);
            let card_pad = m.sp(3);
            let row_h = 78.0;
            let card_h = row_h - m.sp(1);
            let card_w = w - card_m * 2.0;
            let del_r = 16.0;
            let side_w = 64.0; // reserved for the delete button

            // Releasing a left swipe past the threshold deletes the row.
            if !down {
                if let Some(s) = swipe.take() {
                    if s.axis == Axis::Horizontal && -s.dx > SWIPE_DELETE_FRACTION * card_w {
                        swipe_delete = Some(s.id);
                    }
                }
            }

            for (idx, note) in notes.iter().enumerate() {
                let (row, _) = ui.allocate_exact_size(vec2(w, row_h), Sense::hover());
                let card = Rect::from_min_size(
                    pos2(row.left() + card_m, row.top()),
                    vec2(card_w, card_h),
                );
                // Delete is registered after the row so it wins the hit test.
                let row_resp = ui.interact(card, Id::new(("row", note.id)), Sense::click());
                let del_c = pos2(card.right() - 32.0, card.center().y);
                let del_hit = Rect::from_center_size(
                    del_c,
                    vec2(m.touch_target + 4.0, m.touch_target + 4.0),
                );
                let del_resp = ui.interact(del_hit, Id::new(("del", note.id)), Sense::click());
                if del_resp.clicked() {
                    action = Some(ListAction::Delete(idx));
                } else if row_resp.clicked() {
                    action = Some(ListAction::Open(note.id));
                }

                // ---- swipe to delete ---------------------------------------
                // Nothing is decided until the touch moves AXIS_LOCK_SLOP; then
                // the larger axis wins for the rest of the touch, so a scroll
                // never turns into a swipe or the reverse.
                let mut live_dx = None;
                if down && origin.is_some_and(|o| card.contains(o)) {
                    let s = swipe.get_or_insert(Swipe {
                        id: note.id,
                        axis: Axis::Undecided,
                        dx: 0.0,
                    });
                    if s.id == note.id {
                        if let (Some(o), Some(p)) = (origin, pos) {
                            let d = p - o;
                            if s.axis == Axis::Undecided && d.length() > AXIS_LOCK_SLOP {
                                s.axis = if d.x < 0.0 && d.x.abs() > d.y.abs() * 1.2 {
                                    Axis::Horizontal
                                } else {
                                    Axis::Vertical
                                };
                            }
                            if s.axis == Axis::Horizontal {
                                s.dx = d.x.clamp(-card_w * 0.95, 0.0);
                                live_dx = Some(s.dx);
                            }
                        }
                    }
                }
                let off = ui.ctx().animate_value_with_time(
                    Id::new(("layout.swipe", note.id)),
                    live_dx.unwrap_or(0.0),
                    if live_dx.is_some() { 0.0 } else { 0.18 },
                );

                if !ui.is_rect_visible(card) {
                    continue;
                }
                let p = ui.painter();
                if off < -0.5 {
                    p.rect_filled(card, m.corner, pal.danger);
                    if -off > 70.0 {
                        p.text(
                            pos2(card.right() - card_pad, card.center().y),
                            Align2::RIGHT_CENTER,
                            "Delete",
                            m.font(m.text.body_large),
                            pal.on_accent,
                        );
                    }
                }
                let card = card.translate(vec2(off, 0.0));
                let del_c = del_c + vec2(off, 0.0);

                let fill = if selected == Some(note.id) { pal.selected } else { pal.surface };
                p.rect_filled(card, m.corner, fill);

                let armed = matches!(armed_delete, Some((i, _)) if i == idx);
                p.circle_filled(del_c, del_r, if armed { pal.danger } else { pal.danger_bg });
                let k = del_r * 0.45;
                let cross = if armed { pal.on_accent } else { pal.danger };
                p.line_segment(
                    [del_c + vec2(-k, -k), del_c + vec2(k, k)],
                    Stroke::new(2.0, cross),
                );
                p.line_segment(
                    [del_c + vec2(-k, k), del_c + vec2(k, -k)],
                    Stroke::new(2.0, cross),
                );

                let text_w = card.width() - card_pad * 2.0 - side_w;
                let tx = card.left() + card_pad;
                let clip = Rect::from_min_size(pos2(tx, card.top()), vec2(text_w, card_h));
                let pc = p.with_clip_rect(clip);
                pc.text(
                    pos2(tx, card.top() + card_pad),
                    Align2::LEFT_TOP,
                    notes::title_of(&note.body),
                    m.font(m.text.body_large),
                    pal.text,
                );
                if let Some(sn) = notes::snippet_of(&note.body) {
                    pc.text(
                        pos2(tx, card.top() + card_pad + m.text.body_large * 1.45),
                        Align2::LEFT_TOP,
                        sn,
                        m.font(m.text.body),
                        pal.text_weak,
                    );
                }
                p.text(
                    pos2(tx + text_w, card.bottom() - card_pad),
                    Align2::RIGHT_BOTTOM,
                    util::format_note_date(note.updated),
                    m.font(m.text.caption),
                    pal.text_faint,
                );
            }
            ui.add_space(m.sp(3));

            if notes.is_empty() {
                let mid = view.top() + view.height() * 0.4;
                let p = ui.painter();
                let t1 = p.text(
                    pos2(view.center().x, mid),
                    Align2::CENTER_TOP,
                    "No notes yet",
                    m.font(m.text.headline),
                    pal.text_weak,
                );
                p.text(
                    pos2(view.center().x, t1.bottom() + m.sp(1)),
                    Align2::CENTER_TOP,
                    "Tap + to write one",
                    m.font(m.text.body_large),
                    pal.text_faint,
                );
            }
        });
        self.list_offset = out.state.offset.y;
        self.swipe = swipe;

        if let Some(id) = swipe_delete {
            self.delete_note(id);
            return;
        }
        match action {
            Some(ListAction::Open(id)) => self.cmds.push(Cmd::OpenNote(id)),
            Some(ListAction::Delete(idx)) => {
                // Two taps within 2 s on the cross confirms the delete.
                match self.armed_delete.take() {
                    Some((i, t)) if i == idx && t.elapsed() < DELETE_ARM_WINDOW => {
                        if let Some(id) = self.notes.get(idx).map(|n| n.id) {
                            self.delete_note(id);
                        }
                    }
                    _ => self.armed_delete = Some((idx, Instant::now())),
                }
            }
            None => {}
        }
    }

    fn editor_body(&mut self, ui: &mut Ui, pal: &Palette, m: &Metrics) {
        let id = Id::new(EDITOR_ID);
        if std::mem::take(&mut self.focus_editor) {
            // The widget id is shared by every note, so replace the stored
            // cursor/selection with "caret at the end of this note".
            let mut st = egui::text_edit::TextEditState::default();
            let end = egui::text::CCursor::new(self.buffer.chars().count());
            st.cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            st.store(ui.ctx(), id);
            ui.memory_mut(|mem| mem.request_focus(id));
        }
        let body_h = ui.available_height();
        let side = m.sp(3) as i8;
        ScrollArea::vertical()
            .id_salt("editor")
            .auto_shrink(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.add_space(m.sp(2));
                // The field fills the whole body so a tap anywhere below the
                // bar lands on it; TextEdit scrolls its cursor into view.
                let min = vec2(ui.available_width(), (body_h - m.sp(2)).max(0.0));
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut self.buffer)
                        .id(id)
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                            left: side,
                            right: side,
                            top: 0,
                            bottom: 0,
                        }))
                        .desired_width(f32::INFINITY)
                        .min_size(min)
                        .font(m.font(m.text.title))
                        .text_color(pal.text)
                        .hint_text("Start typing...")
                        .lock_focus(true),
                );
                // egui-winit only toggles the keyboard when focus changes, so
                // a keyboard the OS dismissed while the editor kept focus
                // would never come back. A tap in the editor re-asks for it.
                if resp.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));
                }
            });
    }
}
