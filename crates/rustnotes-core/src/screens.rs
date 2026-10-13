//! Screens: the note list, the editor, and the two-pane (list–detail)
//! arrangement used when the window is wide and tall enough.
//!
//! Every screen renders into a [`Scaffold`]; sizes come from [`Metrics`] and
//! colours from [`Palette`], so nothing here carries a literal colour or a
//! hand-rolled safe-area calculation.

use crate::layout::{
    self, add_button, back_button, bar_title, BarLine, Metrics, Palette, Scaffold, WindowClass,
};
use crate::{notes, util, App, Platform, Screen, DELETE_ARM_WINDOW, EDITOR_ID};
use egui::{
    pos2, vec2, Align, Align2, Id, Layout, Rect, ScrollArea, Sense, Stroke, Ui, UiBuilder,
};
use std::time::Instant;

enum ListAction {
    Open(usize),
    Delete(usize),
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
    /// Root of the UI for one frame.
    pub(crate) fn ui(&mut self, root: &mut Ui, plat: &dyn Platform) {
        let ctx = root.ctx().clone();
        let pal = Palette::of(&ctx);
        let m = Metrics::DEFAULT;

        if WindowClass::of(&ctx).two_pane_ok() {
            self.two_pane(root, plat, &pal, &m);
            return;
        }

        match self.screen {
            Screen::List => {
                let count = self.notes.len();
                let mut add = false;
                Scaffold::new(&pal, &m)
                    .limit_body_width()
                    .top_bar(|ui| add = list_bar(ui, count, &pal, &m))
                    .show(root, |ui, _| self.list_body(ui, plat, &pal, &m, None));
                if add {
                    self.armed_delete = None;
                    self.open_editor(0, plat);
                }
            }
            Screen::Editor => {
                let new_note = self.edit_id == 0;
                let mut back = false;
                Scaffold::new(&pal, &m)
                    .background(pal.surface)
                    .limit_body_width()
                    .top_bar(|ui| back = editor_bar(ui, new_note, true, &pal, &m))
                    .show(root, |ui, _| self.editor_body(ui, &pal, &m));
                if back {
                    self.close_editor(plat);
                }
            }
        }
    }

    /// List on the left, editor (or a placeholder) on the right.
    fn two_pane(&mut self, root: &mut Ui, plat: &dyn Platform, pal: &Palette, m: &Metrics) {
        let ctx = root.ctx().clone();
        let full = ctx.viewport_rect();
        let inset_top = ctx.content_rect().top() - full.top();
        let count = self.notes.len();
        let detail_open = self.screen == Screen::Editor;
        let selected = (detail_open && self.edit_id != 0).then_some(self.edit_id);
        let new_note = self.edit_id == 0;
        let mut add = false;

        Scaffold::new(pal, m).show(root, |ui, _| {
            // The scaffold clips its body; the bars extend into the status-bar
            // strip above it, so open the clip to the whole viewport.
            ui.set_clip_rect(full);
            let r = ui.max_rect();
            let list_w = (r.width() * 0.36).clamp(320.0, 420.0).min(r.width());
            let left = Rect::from_min_max(r.min, pos2(r.min.x + list_w, r.max.y));
            let right = Rect::from_min_max(pos2(left.right(), r.min.y), r.max);

            // Detail surface, edge to edge behind the right pane.
            ui.painter().rect_filled(
                Rect::from_min_max(pos2(right.left(), full.top()), full.max),
                0.0,
                pal.surface,
            );

            // List pane.
            let list_bar_rect = Rect::from_min_size(left.min, vec2(left.width(), m.bar_height));
            layout::bar(ui, list_bar_rect, inset_top, 0.0, BarLine::Bottom, pal, |u| {
                add = list_bar(u, count, pal, m)
            });
            let list_rect = Rect::from_min_max(pos2(left.min.x, list_bar_rect.bottom()), left.max);
            let mut list_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(list_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            list_ui.set_clip_rect(list_rect);
            self.list_body(&mut list_ui, plat, pal, m, selected);

            ui.painter().line_segment(
                [pos2(left.right(), full.top()), pos2(left.right(), full.bottom())],
                Stroke::new(1.0, pal.hairline),
            );

            // Detail pane.
            let detail_bar = Rect::from_min_size(right.min, vec2(right.width(), m.bar_height));
            layout::bar(ui, detail_bar, inset_top, 0.0, BarLine::Bottom, pal, |u| {
                if detail_open {
                    editor_bar(u, new_note, false, pal, m);
                }
            });
            let detail_rect = Rect::from_min_max(pos2(right.left(), detail_bar.bottom()), right.max);
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
            self.armed_delete = None;
            self.open_editor(0, plat);
        }
    }

    fn list_body(
        &mut self,
        ui: &mut Ui,
        plat: &dyn Platform,
        pal: &Palette,
        m: &Metrics,
        selected: Option<i64>,
    ) {
        // Armed delete expires after 2 s even without input.
        if let Some((_, t)) = self.armed_delete {
            let elapsed = t.elapsed();
            if elapsed > DELETE_ARM_WINDOW {
                self.armed_delete = None;
            } else {
                ui.ctx().request_repaint_after(DELETE_ARM_WINDOW - elapsed);
            }
        }

        let mut scroll = ScrollArea::vertical()
            .id_salt("list")
            .auto_shrink(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if std::mem::take(&mut self.reset_list_scroll) {
            scroll = scroll.vertical_scroll_offset(0.0);
        }

        let notes = &self.notes;
        let armed_delete = self.armed_delete;
        let mut action: Option<ListAction> = None;
        scroll.show(ui, |ui| {
            let view = ui.available_rect_before_wrap();
            ui.add_space(m.sp(2));
            let w = ui.available_width();
            let card_m = m.sp(2);
            let card_pad = m.sp(3);
            let row_h = 78.0;
            let card_h = row_h - m.sp(1);
            let del_r = 16.0;
            let side_w = 64.0; // reserved for the delete button

            for (idx, note) in notes.iter().enumerate() {
                let (row, _) = ui.allocate_exact_size(vec2(w, row_h), Sense::hover());
                let card = Rect::from_min_size(
                    pos2(row.left() + card_m, row.top()),
                    vec2(w - card_m * 2.0, card_h),
                );
                // Delete is registered after the row so it wins the hit test.
                let row_resp = ui.interact(card, Id::new(("row", note.id)), Sense::click());
                let del_c = pos2(card.right() - 32.0, card.center().y);
                let del_hit = Rect::from_center_size(del_c, vec2(m.touch_target + 4.0, m.touch_target + 4.0));
                let del_resp = ui.interact(del_hit, Id::new(("del", note.id)), Sense::click());
                if del_resp.clicked() {
                    action = Some(ListAction::Delete(idx));
                } else if row_resp.clicked() {
                    action = Some(ListAction::Open(idx));
                }
                if !ui.is_rect_visible(card) {
                    continue;
                }

                let p = ui.painter();
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

        match action {
            Some(ListAction::Open(idx)) => {
                if let Some(id) = self.notes.get(idx).map(|n| n.id) {
                    self.armed_delete = None;
                    self.open_editor(id, plat);
                }
            }
            Some(ListAction::Delete(idx)) => {
                // Two taps within 2 s on the cross confirms the delete.
                match self.armed_delete.take() {
                    Some((i, t)) if i == idx && t.elapsed() < DELETE_ARM_WINDOW => {
                        if let Some(id) = self.notes.get(idx).map(|n| n.id) {
                            self.db.delete(id);
                            if self.screen == Screen::Editor && self.edit_id == id {
                                // Two-pane: the open note was deleted.
                                self.screen = Screen::List;
                                self.buffer.clear();
                                self.edit_id = 0;
                            }
                        }
                        self.notes = self.db.list();
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
                ui.add(
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
            });
    }
}
