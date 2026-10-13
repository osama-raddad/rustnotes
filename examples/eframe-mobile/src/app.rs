//! The eframe application: state, responsive layout, localisation, theming.
//!
//! Mobile immediate-mode rules this file follows:
//! - `ui` runs every frame. No I/O, no allocation-heavy work, no
//!   `set_fonts` / `set_visuals` unless something actually changed.
//! - Layout is derived from the current content rect each frame, so a
//!   rotation (delivered by winit as a resize) re-flows on the next frame
//!   without losing state.
//! - Touch targets are at least 44 points tall.

use crate::theme::Theme;
use rust_i18n::t;

/// Languages the app ships translations for (see locales/app.yml).
const LOCALES: &[(&str, &str)] = &[("en", "English"), ("ar", "العربية"), ("zh-CN", "中文")];

/// Minimum touch-target height on mobile, in points.
const TOUCH_TARGET: f32 = 44.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Persistent app state. Everything here is saved by eframe on exit and on
/// Android `onPause`, so the user comes back to where they were.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MobileApp {
    /// rust-i18n locale code, e.g. "en", "ar", "zh-CN".
    locale: String,
    theme: Theme,
    /// Demo state so the sample screen does something.
    counter: u32,
    name: String,

    /// Runtime-only: what the last frame applied, to avoid redundant work.
    #[serde(skip)]
    applied_theme: Option<Theme>,
    #[serde(skip)]
    orientation: Option<Orientation>,
}

impl Default for MobileApp {
    fn default() -> Self {
        Self {
            locale: "en".to_owned(),
            theme: Theme::System,
            counter: 0,
            name: "World".to_owned(),
            applied_theme: None,
            orientation: None,
        }
    }
}

impl MobileApp {
    /// Called once by eframe. One-time setup belongs here, not in `ui`.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 1. Fonts first: the glyph atlas must know Noto before any text lays out.
        crate::fonts::install(&cc.egui_ctx);

        // 2. Restore state saved by a previous run, if any.
        let mut app: Self = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        // 3. Locale is process-global in rust-i18n; sync it from state.
        rust_i18n::set_locale(&app.locale);

        // 4. Mobile ergonomics: bigger spacing and touch-friendly widgets,
        //    applied to both the dark and the light style.
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.button_padding = egui::vec2(16.0, 10.0);
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style.spacing.interact_size.y = TOUCH_TARGET;
            style.interaction.selectable_labels = false; // no accidental text drags
        });

        app.apply_theme(&cc.egui_ctx);
        app
    }

    /// Push the selected theme into egui, once per change.
    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.applied_theme == Some(self.theme) {
            return;
        }
        self.theme.apply(ctx);
        self.applied_theme = Some(self.theme);
    }

    /// Portrait vs landscape, from the area safe for content.
    /// `ctx.content_rect()` is the viewport minus `safe_area_insets`, which
    /// egui-winit fills on iOS (notch, home indicator). On Android
    /// `raw_input_hook` fills them from the root view's `WindowInsets`
    /// (system bars, display cutout and keyboard), which is correct under
    /// Android 15+'s enforced edge-to-edge (see `android::safe_area_insets`).
    /// Desktop has no insets.
    fn orientation(ctx: &egui::Context) -> Orientation {
        let rect = ctx.content_rect();
        if rect.height() >= rect.width() {
            Orientation::Portrait
        } else {
            Orientation::Landscape
        }
    }

    // ---- screen pieces -----------------------------------------------------

    fn settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("app.title"));

        // Language picker: ComboBox is touch-friendly and closes on tap-away.
        let current = LOCALES
            .iter()
            .find(|(code, _)| *code == self.locale)
            .map(|(_, label)| *label)
            .unwrap_or("?");
        let mut new_locale: Option<&str> = None;
        egui::ComboBox::from_label(t!("app.language"))
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (code, label) in LOCALES {
                    if ui.selectable_label(self.locale == *code, *label).clicked() {
                        new_locale = Some(code);
                    }
                }
            });
        if let Some(code) = new_locale {
            self.locale = code.to_owned();
            rust_i18n::set_locale(code);
        }

        // Theme picker; the change is applied on the next frame by apply_theme.
        egui::ComboBox::from_label(t!("app.theme"))
            .selected_text(self.theme.label())
            .show_ui(ui, |ui| {
                for theme in Theme::ALL {
                    ui.selectable_value(&mut self.theme, *theme, theme.label());
                }
            });
    }

    fn content_panel(&mut self, ui: &mut egui::Ui) {
        // `t!` with interpolation; the YAML uses %{name} / %{count}.
        ui.label(t!("app.greeting", name = self.name));
        ui.text_edit_singleline(&mut self.name);

        ui.label(t!("app.counter", count = self.counter));
        let button = egui::Button::new(t!("app.tap_me"))
            .min_size(egui::vec2(ui.available_width().min(240.0), TOUCH_TARGET));
        if ui.add(button).clicked() {
            self.counter += 1;
        }

        let orientation_key = match self.orientation.unwrap_or(Orientation::Portrait) {
            Orientation::Portrait => "app.orientation.portrait",
            Orientation::Landscape => "app.orientation.landscape",
        };
        ui.weak(t!(orientation_key));
    }
}

impl eframe::App for MobileApp {
    /// eframe 0.36: the root `Ui` covers the whole viewport with no margin
    /// or background. Wrap everything in a `CentralPanel` for both.
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();

        // Theme changes are applied lazily, once, after the picker moved.
        self.apply_theme(&ctx);

        // Re-evaluate orientation every frame: a rotation arrives as a plain
        // resize (thanks to android:configChanges) and the next frame
        // simply lays out differently. Nothing is destroyed or rebuilt.
        let orientation = Self::orientation(&ctx);
        if self.orientation != Some(orientation) {
            self.orientation = Some(orientation);
            log::info!("orientation: {orientation:?}");
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.global_style()).inner_margin(0.0))
            .show(root, |ui| {
                // The panel paints the full viewport (so the notch area has
                // the right colour) but our widgets stay inside the safe
                // area so nothing sits under the notch or home indicator.
                // The 16 pt gutter is applied here, once; the child Ui
                // ignores the panel frame's margin, so none is set there.
                let safe = ctx.content_rect().shrink(16.0);
                let mut safe_ui = ui.new_child(egui::UiBuilder::new().max_rect(safe));
                let ui = &mut safe_ui;

                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| match orientation {
                        Orientation::Portrait => {
                            // Stack: settings on top, content below.
                            ui.vertical(|ui| {
                                self.settings_panel(ui);
                                ui.separator();
                                self.content_panel(ui);
                            });
                        }
                        Orientation::Landscape => {
                            // Side by side: settings left, content right.
                            ui.horizontal_top(|ui| {
                                let half = ui.available_width() / 2.0;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(half, ui.available_height()),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| self.settings_panel(ui),
                                );
                                ui.separator();
                                ui.vertical(|ui| self.content_panel(ui));
                            });
                        }
                    });
            });
    }

    /// Android only: feed egui the platform state egui-winit leaves out.
    /// Runs before every frame, so rotation, keyboard and dark-mode changes
    /// are picked up on the next frame.
    #[cfg(target_os = "android")]
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // Safe area: egui-winit only fills this on iOS. Setting it here makes
        // `ctx.content_rect()` (layout and orientation) respect it.
        if let Some(insets) = crate::android::safe_area_insets(ctx, ctx.pixels_per_point()) {
            raw_input.safe_area_insets = Some(insets);
        }

        // Theme::System: winit reports no OS theme on Android, so egui uses
        // `fallback_theme`. Point it at the configuration's night mode.
        if let Some(night) = crate::android::night_mode() {
            if ctx.options(|o| o.fallback_theme) != night {
                ctx.options_mut(|o| o.fallback_theme = night);
            }
        }
    }

    /// eframe calls this on exit and on Android pause/suspend.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
