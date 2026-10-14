//! TEMP-DIAGNOSE: measure the Brand-panel components' widths at phone
//! size to find the 20k-wide layout.

use ui_kit::components::{BrandShowcase, Cover, ThemesShowcase};
use ui_kit::theme::ThemeId;

fn measure(name: &str, add: impl Fn(&mut egui::Ui, ThemeId)) {
    let ctx = egui::Context::default();
    ui_kit::icon::install(&ctx);
    for id in ThemeId::ALL {
        ui_kit::typography::install(*id, &ctx);
    }
    ThemeId::Light.apply(&ctx);
    let mut width = 0.0;
    for _ in 0..3 {
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_max_width(336.0);
            ui.set_width(336.0);
            add(ui, ThemeId::Light);
            width = ui.min_rect().width();
        });
        out.drop_without_applying_deltas();
    }
    println!("{name}: min_rect width {width:.1}");
    // The guard catches the pathological case (the unwrapped text and
    // sideways card interiors measured 16,224pt before the wrap fixes).
    // The showcases' card rows still run past a phone viewport (cards
    // side by side); phone-tight rows are a follow-up.
    assert!(
        width <= 2500.0,
        "{name} must not lay out pathologically wide, measured {width}"
    );
}

#[test]
fn measure_brand_panel_components() {
    measure("Cover", |ui, t| {
        Cover::new().ui(ui, t);
    });
    measure("ThemesShowcase", |ui, t| {
        ThemesShowcase::new().ui(ui, t, &Default::default());
    });
    measure("BrandShowcase", |ui, t| {
        BrandShowcase::new().ui(ui, t, &Default::default());
    });
}
