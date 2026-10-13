//! The Vynatix gallery: a theme switcher and a palette swatch grid over
//! the kit's tokens. Run with `cargo run -p ui-kit --example gallery`.
//! Minimal by design (issue 01): the heading keeps the system font until
//! the type work lands, and each later issue grows a section here.

use eframe::egui::{self, CornerRadius, Sense, Vec2};

use ui_kit::theme::ThemeId;
use ui_kit::tokens::{radii, spacing, ThemeColors};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vynatix UI kit")
            .with_inner_size([1000.0, 780.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Vynatix UI kit",
        options,
        Box::new(|cc| {
            let theme = ThemeId::Light;
            theme.apply(&cc.egui_ctx);
            Ok(Box::new(Gallery { theme }))
        }),
    )
}

struct Gallery {
    theme: ThemeId,
}

/// A role name for display: strip the `md-sys-color-` prefix and set the
/// words in sentence case ("surface-container-low" becomes "Surface
/// container low").
fn sentence_case(role: &str) -> String {
    let stripped = role.strip_prefix("md-sys-color-").unwrap_or(role);
    let mut out = String::with_capacity(stripped.len());
    for (i, word) in stripped.split('-').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
        }
        out.push_str(chars.as_str());
    }
    out
}

impl eframe::App for Gallery {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        egui::CentralPanel::default().show(root, |ui| {
            // Placeholder heading: the system font until issue 02.
            ui.heading("Vynatix UI kit");
            ui.add_space(spacing::SPACE_4);

            ui.label("Themes");
            ui.add_space(spacing::SPACE_2);
            ui.horizontal(|ui| {
                for id in ThemeId::ALL {
                    let mut picked = self.theme;
                    ui.selectable_value(&mut picked, *id, id.name());
                    if picked != self.theme {
                        self.theme = picked;
                        picked.apply(&ctx);
                    }
                }
            });

            ui.add_space(spacing::SPACE_6);
            ui.label(format!("Colour roles · {}", self.theme.name()));
            ui.add_space(spacing::SPACE_2);

            let colors = self.theme.colors();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("colour-roles")
                    .num_columns(2)
                    .spacing([spacing::SPACE_4, spacing::SPACE_2])
                    .show(ui, |ui| {
                        for role in ThemeColors::ROLES {
                            let color = colors
                                .role(role)
                                .expect("ROLES names all resolve to fields");
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(radii::RADIUS_XS as u8),
                                color,
                            );
                            ui.label(sentence_case(role));
                            ui.end_row();
                        }
                    });
            });
        });
    }
}
