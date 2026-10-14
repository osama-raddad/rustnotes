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
use crate::{debounce, notes, App, DebounceAction, Screen, Tab, DELETE_ARM_WINDOW, EDITOR_ID};
use egui::{
    pos2,
    text::{LayoutJob, TextWrapping},
    vec2, Align, Align2, Id, Layout, Rect, ScrollArea, Sense, Shape, Stroke, Ui, UiBuilder,
};
use std::time::Instant;

/// Inputs the settings diagnostics depend on, as a cache key (NIT-2): the
/// rows are rebuilt only when one of these changes, not every frame.
#[derive(PartialEq)]
pub(crate) struct DiagKey {
    px_bits: Option<u32>,
    zoom_bits: u32,
    w_bits: u32,
    h_bits: u32,
    class: WindowClass,
    theme: egui::Theme,
}

/// Pull distance, in points, past which releasing triggers a refresh.
const PULL_THRESHOLD: f32 = 64.0;
/// Fraction of the card width a left swipe must pass to delete on release.
const SWIPE_DELETE_FRACTION: f32 = 0.4;
/// Movement, in points, before a touch commits to scrolling or swiping.
const AXIS_LOCK_SLOP: f32 = 8.0;
/// What an armed row says in place of its date. The first tap on the cross
/// only arms the delete; without this the button looks broken, because
/// nothing else tells the user a second tap is wanted.
const DELETE_HINT: &str = "Tap again to delete";

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
    /// Which note's delete cross was tapped (MED-1: keyed by id, not row
    /// index, so a reorder between the two taps can't retarget the delete).
    Delete(i64),
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
        // NIT-3: two clocks — egui's `time` (this one) drives nav animation
        // because it steps with painted frames; `Instant` drives the
        // delete-arm window and the autosave debounce, which must elapse
        // (and fire) with no input at all. Don't mix them.
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

        // ---- autosave (HIGH-2) ------------------------------------------
        // The TextEdit wrote into `buffer` while the editor drew; a diff
        // against the previous frame's snapshot records the edit time.
        if self.buffer != self.last_seen_buffer {
            self.last_seen_buffer.clone_from(&self.buffer);
            self.last_edit = Some(Instant::now());
        }
        match debounce(self.last_edit, self.dirty(), Instant::now()) {
            DebounceAction::Save => self.autosave(),
            // Repaint after the remaining quiet period so the save fires
            // even with no further input.
            DebounceAction::Wait(left) => ctx.request_repaint_after(left),
            DebounceAction::None => {}
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
                let list_bar_rect = Rect::from_min_size(left.min, vec2(left.width(), m.bar_height));
                layout::bar(
                    ui,
                    list_bar_rect,
                    inset_top,
                    0.0,
                    BarLine::Bottom,
                    pal,
                    |u| add = list_bar(u, count, pal, m),
                );
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
        // NIT-2: rebuild the rows only when an input value changed, not
        // every frame.
        let key = DiagKey {
            px_bits: ctx.native_pixels_per_point().map(f32::to_bits),
            zoom_bits: ctx.zoom_factor().to_bits(),
            w_bits: vp.width().to_bits(),
            h_bits: vp.height().to_bits(),
            class,
            theme: ctx.theme(),
        };
        if !matches!(&self.diag, Some((k, _)) if *k == key) {
            let rows: Vec<(&str, String)> = vec![
                (
                    "Display scale",
                    format!(
                        "{:.2} px per point",
                        ctx.native_pixels_per_point().unwrap_or(1.0)
                    ),
                ),
                ("Text scale", format!("{:.2}x", ctx.zoom_factor())),
                (
                    "Window",
                    format!("{:.0} x {:.0} pt", vp.width(), vp.height()),
                ),
                (
                    "Size class",
                    format!("{:?} width, {:?} height", class.width, class.height),
                ),
                (
                    "Layout",
                    if class.two_pane_ok() {
                        "two-pane"
                    } else {
                        "single-pane"
                    }
                    .to_string(),
                ),
                ("Appearance", format!("{:?}", ctx.theme())),
                ("Version", env!("CARGO_PKG_VERSION").to_string()),
            ];
            self.diag = Some((key, rows));
        }
        // HIGH-1b: the live storage state is one cheap row, always fresh.
        let storage = match &self.db_error {
            Some(e) => format!("error: {e}"),
            None => "ok".to_owned(),
        };
        let diag = self.diag.as_ref().map(|(_, r)| r.as_slice()).unwrap_or(&[]);
        Scaffold::new(pal, m)
            .limit_body_width()
            .top_bar(|u| {
                bar_title(u, m.sp(3), "Settings", pal, m);
            })
            .bottom_bar(|u| tab = tab_bar(u, Tab::Settings, pal, m))
            .show(root, |ui, _| {
                ui.add_space(m.sp(2));
                let rows = diag
                    .iter()
                    .map(|(l, v)| (*l, v))
                    .chain(std::iter::once(("Storage", &storage)));
                for (label, value) in rows {
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
        if let Err(e) = self.db.delete(id) {
            // NotesDb::delete already logged; keep the UI state.
            self.db_error = Some(format!("{}: {e}", crate::DB_DELETE_ERR));
        } else {
            self.clear_error_if(crate::DB_DELETE_ERR);
        }
        if self.screen() == Screen::Editor && self.edit_id == id {
            // Two-pane: the open note was deleted.
            let now = self.now();
            self.nav.pop(now, false);
            self.buffer.clear();
            self.loaded_body.clear();
            self.last_seen_buffer.clear();
            self.last_edit = None;
            self.edit_id = 0;
        }
        // MED-1: any delete invalidates a pending confirm, and the re-list
        // below rebuilds ids/dates.
        self.armed_delete = None;
        self.refresh_notes();
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
                // Refresh (also clears the armed delete — MED-1).
                self.refresh_notes();
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
            let color = if progress >= 1.0 {
                pal.accent
            } else {
                pal.text_faint
            };
            ui.painter().add(Shape::line(pts, Stroke::new(2.5, color)));
            ui.add_space(pull);
        }

        // HIGH-1b: with rows on screen but storage failing (typically a
        // save error carried out of the editor), keep a persistent hint
        // above the list instead of failing silently.
        if !self.notes.is_empty() && self.db_error.is_some() {
            let (strip, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), m.touch_target * 0.6),
                Sense::hover(),
            );
            ui.painter().text(
                strip.center(),
                Align2::CENTER_CENTER,
                "Storage error — some changes may not be saved",
                m.font(m.text.caption),
                pal.danger,
            );
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
        let dates = &self.row_dates;
        let db_error = self.db_error.as_deref();
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

            for (note, date) in notes.iter().zip(dates) {
                let (row, _) = ui.allocate_exact_size(vec2(w, row_h), Sense::hover());
                let card =
                    Rect::from_min_size(pos2(row.left() + card_m, row.top()), vec2(card_w, card_h));
                // Delete is registered after the row so it wins the hit test.
                let row_resp = ui.interact(card, Id::new(("row", note.id)), Sense::click());
                let del_c = pos2(card.right() - 32.0, card.center().y);
                let del_hit =
                    Rect::from_center_size(del_c, vec2(m.touch_target + 4.0, m.touch_target + 4.0));
                let del_resp = ui.interact(del_hit, Id::new(("del", note.id)), Sense::click());
                if del_resp.clicked() {
                    action = Some(ListAction::Delete(note.id));
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

                let fill = if selected == Some(note.id) {
                    pal.selected
                } else {
                    pal.surface
                };
                p.rect_filled(card, m.corner, fill);

                let armed = matches!(armed_delete, Some((id, _)) if id == note.id);
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
                // The date's slot: bottom-right of the text area. Its text is
                // the row's date, or the delete hint while the row is armed.
                // Either way it is one line, cut with an ellipsis on its right
                // when the column is narrower than its words (large text
                // sizes) rather than losing its first words off the column's
                // left edge.
                let (slot_text, slot_color) = if armed {
                    (DELETE_HINT, pal.danger)
                } else {
                    // Cached when the list loaded (MED-5), not reformatted
                    // per row per frame.
                    (date.as_str(), pal.text_faint)
                };
                let mut job = LayoutJob::simple_singleline(
                    slot_text.to_owned(),
                    m.font(m.text.caption),
                    slot_color,
                );
                job.wrap = TextWrapping {
                    overflow_character: Some('…'),
                    ..TextWrapping::truncate_at_width(text_w.max(0.0))
                };
                let slot_galley = p.layout_job(job);
                let slot_rect = Align2::RIGHT_BOTTOM.anchor_size(
                    pos2(tx + text_w, card.bottom() - card_pad),
                    slot_galley.size(),
                );
                pc.text(
                    pos2(tx, card.top() + card_pad),
                    Align2::LEFT_TOP,
                    notes::title_of(&note.body),
                    m.font(m.text.body_large),
                    pal.text,
                );
                if let Some(sn) = notes::snippet_of(&note.body) {
                    // The snippet runs along the slot's line, so whatever owns
                    // the slot has it to itself: the snippet stops short of it
                    // rather than running under it (or being painted over),
                    // the way it is already cut at the column's edge when it
                    // is merely long. If the slot's text leaves no room at
                    // all, the clip collapses to nothing instead of going
                    // negative.
                    let right = (slot_rect.left() - m.sp(1)).max(clip.left());
                    p.with_clip_rect(clip.with_max_x(right)).text(
                        pos2(tx, card.top() + card_pad + m.text.body_large * 1.45),
                        Align2::LEFT_TOP,
                        sn,
                        m.font(m.text.body),
                        pal.text_weak,
                    );
                }
                pc.galley(slot_rect.min, slot_galley, slot_color);
            }
            ui.add_space(m.sp(3));

            if notes.is_empty() {
                let mid = view.top() + view.height() * 0.4;
                let p = ui.painter();
                if let Some(_err) = db_error {
                    // HIGH-1b: the read failed — say so, instead of the
                    // misleading "no notes" empty state.
                    let t1 = p.text(
                        pos2(view.center().x, mid),
                        Align2::CENTER_TOP,
                        "Couldn't load notes",
                        m.font(m.text.headline),
                        pal.danger,
                    );
                    p.text(
                        pos2(view.center().x, t1.bottom() + m.sp(1)),
                        Align2::CENTER_TOP,
                        "Storage error — see Settings",
                        m.font(m.text.body_large),
                        pal.text_faint,
                    );
                } else {
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
            Some(ListAction::Delete(id)) => {
                // Two taps within 2 s on the cross confirms the delete,
                // matched by note id (MED-1) so a reorder between taps
                // cannot retarget it.
                match self.armed_delete.take() {
                    Some((armed_id, t)) if armed_id == id && t.elapsed() < DELETE_ARM_WINDOW => {
                        self.delete_note(id);
                    }
                    _ => self.armed_delete = Some((id, Instant::now())),
                }
            }
            None => {}
        }
    }

    fn editor_body(&mut self, ui: &mut Ui, pal: &Palette, m: &Metrics) {
        let id = Id::new(EDITOR_ID);
        // HIGH-1b: a small persistent indicator while the last save failed,
        // so "saved" cannot silently be a lie.
        // TODO(verify-on-device): confirm the strip reads as persistent
        // (not a flash) next to the soft keyboard, and that it clears after
        // a successful retry.
        if self.db_error.is_some() {
            let (strip, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), m.touch_target * 0.6),
                Sense::hover(),
            );
            ui.painter().text(
                strip.center(),
                Align2::CENTER_CENTER,
                "Save failed — changes not stored",
                m.font(m.text.caption),
                pal.danger,
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, RawInput};
    use std::time::Duration;

    /// Copy of `temp` in lib.rs's tests (private to that module), under its own prefix,
    /// plus a wipe: nothing cleans these directories up and process ids are recycled.
    fn temp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("rn-screens-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A text shape as a pass painted it.
    struct Painted {
        text: String,
        /// What of `text` is on screen: all of it, unless the galley is elided.
        shown: String,
        /// The glyph box.
        rect: Rect,
        /// The clip rect it was painted under: what shows of it.
        clip: Rect,
    }

    /// One egui pass at a phone's size, as the host runs it: the app draws
    /// into a root `Ui` covering the screen between begin_pass/end_pass.
    /// Returns the text the pass painted, so a test can ask what is on screen
    /// and where. Modelled on `run_pass` in lib.rs's tests (private to that module).
    fn run_pass(app: &mut App, ctx: &egui::Context, events: Vec<Event>, t: f64) -> Vec<Painted> {
        run_pass_sized(app, ctx, vec2(400.0, 800.0), events, t)
    }

    /// `run_pass` at any screen size.
    fn run_pass_sized(
        app: &mut App,
        ctx: &egui::Context,
        size: egui::Vec2,
        events: Vec<Event>,
        t: f64,
    ) -> Vec<Painted> {
        let screen = Rect::from_min_size(egui::Pos2::ZERO, size);
        ctx.begin_pass(RawInput {
            time: Some(t),
            screen_rect: Some(screen),
            events,
            ..Default::default()
        });
        let mut root = Ui::new(
            ctx.clone(),
            Id::new("test-root"),
            UiBuilder::new().max_rect(screen),
        );
        app.ui(&mut root);
        let mut out = ctx.end_pass();
        // No renderer applies the glyph-atlas delta; clear it or epaint
        // panics on the drop.
        out.textures_delta.clear();
        let mut painted = vec![];
        for clipped in &out.shapes {
            collect_text(&clipped.shape, clipped.clip_rect, &mut painted);
        }
        painted
    }

    /// Every `Shape::Text` in `shape`, looking inside `Shape::Vec` groups,
    /// which are painted under their parent's clip rect.
    fn collect_text(shape: &Shape, clip: Rect, into: &mut Vec<Painted>) {
        match shape {
            Shape::Text(text) => into.push(Painted {
                text: text.galley.text().to_owned(),
                shown: text.galley.rows.iter().map(|row| row.text()).collect(),
                rect: Rect::from_min_size(text.pos, text.galley.size()),
                clip,
            }),
            Shape::Vec(inner) => inner.iter().for_each(|s| collect_text(s, clip, into)),
            _ => {}
        }
    }

    /// The first text of that string a pass painted. Rows paint top to
    /// bottom, so when two rows carry the same string (two dates in the same
    /// minute) this is the top row's.
    fn find<'a>(all: &'a [Painted], text: &str) -> &'a Painted {
        all.iter()
            .find(|p| p.text == text)
            .unwrap_or_else(|| panic!("nothing painted {text:?}"))
    }

    /// A tap at `pos`: move, press, release, all in one pass. Copy of `click`
    /// in lib.rs's tests (private to that module).
    fn click(pos: egui::Pos2) -> Vec<Event> {
        let button = |pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![Event::PointerMoved(pos), button(true), button(false)]
    }

    /// A fresh app listing `bodies` as saved notes, the last one on top
    /// (newest first).
    fn app_with_notes(ctx: &egui::Context, name: &str, bodies: &[&str]) -> App {
        let mut app = App::new(temp(name).join("data"), ctx, None);
        for body in bodies {
            app.open_editor(0);
            app.buffer = (*body).to_owned();
            app.close_editor();
        }
        assert_eq!(app.notes.len(), bodies.len());
        app
    }

    /// A fresh app listing two saved notes, newest first.
    fn app_with_two_notes(ctx: &egui::Context, name: &str) -> App {
        app_with_notes(ctx, name, &["first note", "second note"])
    }

    /// How many delete hints were painted.
    fn hints(painted: &[Painted]) -> usize {
        painted.iter().filter(|p| p.text == DELETE_HINT).count()
    }

    /// How many of the list's cached row dates were painted.
    fn dates(app: &App, painted: &[Painted]) -> usize {
        painted
            .iter()
            .filter(|p| app.row_dates.contains(&p.text))
            .count()
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    /// The first tap on a row's cross only arms the delete, so the armed row
    /// has to say a second tap is wanted: its date gives way to the hint from
    /// the frame after the tap, and the other row is left alone.
    #[test]
    fn tapping_the_cross_shows_the_delete_hint_in_place_of_the_date() {
        let ctx = egui::Context::default();
        let mut app = app_with_two_notes(&ctx, "hint-shows");
        let id = app.notes[0].id;

        let idle = run_pass(&mut app, &ctx, vec![], 1.0);
        assert_eq!(hints(&idle), 0, "no hint before the tap");
        assert_eq!(dates(&app, &idle), 2, "both rows show their dates");

        // Read the cross's rect back from the widget registry: the same rect
        // egui hit-tests against.
        let cross = ctx
            .read_response(Id::new(("del", id)))
            .expect("the delete cross is laid out")
            .rect
            .center();
        run_pass(&mut app, &ctx, click(cross), 2.0);
        assert_eq!(
            app.armed_delete.map(|(armed, _)| armed),
            Some(id),
            "the tap arms the row"
        );

        // The tap arms after the rows were painted, so the hint is on screen
        // from the next frame.
        let armed = run_pass(&mut app, &ctx, vec![], 2.1);
        assert_eq!(hints(&armed), 1, "one hint, for the armed row");
        assert_eq!(
            dates(&app, &armed),
            1,
            "the armed row's date gave way; the other row keeps its own"
        );
    }

    /// The hint lives exactly as long as the arm: a re-list clears it, and so
    /// does the arm running out of its window (back-dated here, because the
    /// window runs on `Instant`).
    #[test]
    fn delete_hint_goes_when_the_arm_does() {
        let ctx = egui::Context::default();
        let mut app = app_with_two_notes(&ctx, "hint-clears");
        let id = app.notes[0].id;
        run_pass(&mut app, &ctx, vec![], 1.0);

        app.armed_delete = Some((id, Instant::now()));
        let armed = run_pass(&mut app, &ctx, vec![], 1.1);
        assert_eq!(hints(&armed), 1, "an armed row shows the hint");

        app.refresh_notes();
        let relisted = run_pass(&mut app, &ctx, vec![], 1.2);
        assert_eq!(hints(&relisted), 0, "a re-list clears the arm and the hint");
        assert_eq!(dates(&app, &relisted), 2, "the dates are back");

        let stale = Instant::now()
            .checked_sub(DELETE_ARM_WINDOW + Duration::from_secs(1))
            .expect("the clock has run for longer than the arm window");
        app.armed_delete = Some((id, stale));
        let expired = run_pass(&mut app, &ctx, vec![], 1.3);
        assert_eq!(hints(&expired), 0, "an expired arm shows no hint");
        assert!(app.armed_delete.is_none(), "the expiry drops the arm");
    }

    /// A note's second line, long enough to run the whole width of its row.
    const LONG_SNIPPET: &str =
        "this line after the title runs on well past the end of the text column and under the slot";

    /// The hint takes the date's slot (the text column wide, the date's own
    /// line tall, ending where the date ends) and nothing outside it. The
    /// snippet shares that line, so whatever owns the slot (the date, or the
    /// hint while the row is armed) has it to itself: the snippet stops one
    /// gap short of it instead of running under it or being painted over.
    /// Nothing else in the row gives way.
    #[test]
    fn armed_hint_takes_the_date_slot_and_a_long_snippet_makes_room() {
        let ctx = egui::Context::default();
        let long_note = format!("long note\n{LONG_SNIPPET}");
        let mut app = app_with_notes(&ctx, "hint-slot", &["first note", long_note.as_str()]);
        let id = app.notes[0].id;

        let idle = run_pass(&mut app, &ctx, vec![], 1.0);
        let cross = ctx
            .read_response(Id::new(("del", id)))
            .expect("the delete cross is laid out")
            .rect;
        app.armed_delete = Some((id, Instant::now()));
        let armed = run_pass(&mut app, &ctx, vec![], 1.1);

        let date = find(&idle, &app.row_dates[0]);
        let title = find(&armed, "long note");
        let snippet = find(&armed, LONG_SNIPPET);
        let hint = find(&armed, DELETE_HINT);

        // In the date's place. The slot is the text column wide (the title
        // starts it) and the date's own line tall, ending where the date ends.
        let slot = Rect::from_min_max(pos2(title.rect.left(), date.rect.top()), date.rect.max);
        assert!(
            slot.expand(0.01).contains_rect(hint.rect),
            "the hint stays in the date slot: {:?} outside {slot:?}",
            hint.rect
        );
        assert!(
            near(hint.rect.right(), date.rect.right()),
            "it ends where the date ended"
        );
        assert!(
            near(hint.rect.bottom(), date.rect.bottom()),
            "on the date's baseline"
        );
        assert!(
            hint.rect.right() <= cross.left(),
            "clear of the delete cross"
        );
        assert_eq!(
            hint.clip, title.clip,
            "painted under the text column's clip, like the rest of the row's text"
        );
        assert!(
            hint.clip.expand(0.01).contains_rect(hint.rect),
            "not cut off"
        );

        // The snippet shares the slot's line and runs past the whole of it
        // (the premises: without them the next assertions prove nothing)...
        assert!(
            snippet.rect.top() < hint.rect.bottom() && hint.rect.top() < snippet.rect.bottom(),
            "the snippet is on the slot's line"
        );
        assert!(
            snippet.rect.right() > hint.rect.right(),
            "the snippet runs past the whole slot"
        );
        // ...so it stops one gap short of the date when idle, and of the hint
        // when armed. The title above it does not give way.
        let gap = Metrics::DEFAULT.sp(1);
        let idle_snippet = find(&idle, LONG_SNIPPET);
        assert!(
            near(idle_snippet.clip.right(), date.rect.left() - gap),
            "idle, the snippet shows up to {} but should stop {gap} short of the date at {}",
            idle_snippet.clip.right(),
            date.rect.left()
        );
        assert!(
            near(snippet.clip.right(), hint.rect.left() - gap),
            "armed, the snippet shows up to {} but should stop {gap} short of the hint at {}",
            snippet.clip.right(),
            hint.rect.left()
        );
        assert_eq!(
            find(&idle, "long note").clip,
            title.clip,
            "the title does not give way"
        );
        assert_eq!(
            date.clip, title.clip,
            "the date is clipped to the text column too: one path for the slot's text"
        );
    }

    /// In a column narrower than its words (Android's font scale 2.0 on a
    /// 360 dp phone leaves about 180 pt) whatever owns the date's slot is cut
    /// short with an ellipsis on its right instead of losing its first words
    /// on its left, and the snippet has no room at all: its clip collapses to
    /// nothing instead of going negative.
    #[test]
    fn slot_text_truncates_in_a_column_narrower_than_itself() {
        let ctx = egui::Context::default();
        let long_note = format!("long note\n{LONG_SNIPPET}");
        let mut app = app_with_notes(&ctx, "slot-narrow", &["first note", long_note.as_str()]);
        let id = app.notes[0].id;
        run_pass(&mut app, &ctx, vec![], 1.0);
        app.armed_delete = Some((id, Instant::now()));
        let wide = run_pass(&mut app, &ctx, vec![], 1.1);
        let narrow = run_pass_sized(&mut app, &ctx, vec2(180.0, 800.0), vec![], 1.2);

        // The hint keeps its first words and loses its end to an ellipsis.
        let (whole, hint) = (find(&wide, DELETE_HINT), find(&narrow, DELETE_HINT));
        assert!(
            hint.rect.left() >= hint.clip.left() - 0.01,
            "the hint does not start left of its column: {:?} in {:?}",
            hint.rect,
            hint.clip
        );
        assert!(
            whole.rect.width() > hint.clip.width(),
            "the premise: the whole hint is wider than the column"
        );
        assert!(
            hint.shown.starts_with("Tap") && hint.shown.ends_with('…'),
            "the hint is cut at its end: {:?}",
            hint.shown
        );

        // The snippet has no room: its clip is empty, not negative.
        let snippet = find(&narrow, LONG_SNIPPET);
        assert!(
            snippet.clip.width() >= 0.0,
            "the snippet's clip is empty, not negative: {:?}",
            snippet.clip
        );
        assert!(
            snippet.clip.right() <= hint.rect.left(),
            "and it stops short of the hint"
        );

        // An idle row's date takes the same road: a long one is cut at its end.
        app.armed_delete = None;
        app.row_dates[0] = "Sep 30, 2025".to_owned();
        let idle = run_pass_sized(&mut app, &ctx, vec2(180.0, 800.0), vec![], 1.3);
        let date = find(&idle, "Sep 30, 2025");
        assert!(
            date.rect.left() >= date.clip.left() - 0.01,
            "the date does not start left of its column: {:?} in {:?}",
            date.rect,
            date.clip
        );
        assert!(
            date.shown.starts_with("Sep") && date.shown.ends_with('…'),
            "the date is cut at its end: {:?}",
            date.shown
        );
    }
}
