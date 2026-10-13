//! iOS platform glue that eframe/winit do not provide. Everything here runs on
//! the main thread (eframe calls `raw_input_hook` from the UIKit run loop).

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, define_class, msg_send, sel, AnyThread};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSString};
use objc2_ui_kit::{UIEdgeInsets, UIKeyboardFrameEndUserInfoKey, UIKeyboardWillChangeFrameNotification};
use std::cell::{Cell, RefCell};

/// The system UI font file, resolved through CoreText. iOS devices have no
/// readable font at the core's candidate paths (the simulator masks this,
/// because its file reads resolve against the macOS host file system).
pub(crate) fn system_font_bytes() -> Option<(Vec<u8>, String)> {
    use objc2_core_foundation::{CFURLPathStyle, CFURL};
    use objc2_core_text::{kCTFontURLAttribute, CTFont, CTFontUIFontType};
    unsafe {
        let font = CTFont::new_ui_font_for_language(CTFontUIFontType::System, 0.0, None)?;
        let attr = font.attribute(kCTFontURLAttribute)?;
        let url = attr.downcast::<CFURL>().ok()?;
        let path = url.file_system_path(CFURLPathStyle::CFURLPOSIXPathStyle)?;
        let path = path.to_string();
        let bytes = std::fs::read(&path).ok()?;
        if bytes.is_empty() {
            return None;
        }
        Some((bytes, path))
    }
}

/// egui-wgpu asks for WebGPU's default limits, which iOS Metal devices (the
/// simulator included) do not all meet: `max_inter_stage_shader_variables`
/// is 15 there against a default of 16, and eframe exits at startup. Ask for
/// what the adapter supports; egui's shaders need far less.
pub(crate) fn wgpu_options() -> eframe::WgpuConfiguration {
    use eframe::egui_wgpu::WgpuSetup;
    use eframe::wgpu;
    let mut cfg = eframe::WgpuConfiguration::default();
    if let WgpuSetup::CreateNew(create) = &mut cfg.wgpu_setup {
        create.device_descriptor = std::sync::Arc::new(|adapter: &wgpu::Adapter| {
            let supported = adapter.limits();
            wgpu::DeviceDescriptor {
                label: Some("egui wgpu device"),
                required_limits: wgpu::Limits {
                    max_texture_dimension_2d: supported.max_texture_dimension_2d.min(8192),
                    ..supported
                },
                ..Default::default()
            }
        });
    }
    cfg
}

define_class!(
    /// Receives keyboard frame notifications; winit exposes none.
    #[unsafe(super(NSObject))]
    #[name = "RNKeyboardObserver"]
    #[ivars = ()]
    struct KeyboardObserver;

    impl KeyboardObserver {
        #[unsafe(method(keyboardWillChangeFrame:))]
        fn keyboard_will_change_frame(&self, notification: &NSNotification) {
            let top = unsafe {
                notification.userInfo().and_then(|info| {
                    info.objectForKey(UIKeyboardFrameEndUserInfoKey).map(|v| {
                        let r: CGRect = msg_send![&*v, CGRectValue];
                        r.origin.y
                    })
                })
            };
            KEYBOARD_TOP.with(|k| k.set(top));
        }
    }
);

thread_local! {
    /// Top edge of the keyboard in screen points, from the last notification.
    static KEYBOARD_TOP: Cell<Option<f64>> = const { Cell::new(None) };
    static OBSERVER: RefCell<Option<Retained<KeyboardObserver>>> = const { RefCell::new(None) };
    static LAST_POLL: Cell<f64> = const { Cell::new(f64::NEG_INFINITY) };
    static TRAITS: Cell<(bool, f32)> = const { Cell::new((false, 1.0)) };
}

fn ensure_keyboard_observer() {
    OBSERVER.with(|o| {
        if o.borrow().is_some() {
            return;
        }
        let obs: Retained<KeyboardObserver> =
            unsafe { msg_send![super(KeyboardObserver::alloc().set_ivars(())), init] };
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                &obs,
                sel!(keyboardWillChangeFrame:),
                Some(UIKeyboardWillChangeFrameNotification),
                None,
            );
        }
        *o.borrow_mut() = Some(obs);
    });
}

/// `UIApplication.sharedApplication.keyWindow`, if any.
fn key_window() -> Option<Retained<AnyObject>> {
    unsafe {
        let app: Option<Retained<AnyObject>> = msg_send![class!(UIApplication), sharedApplication];
        let app = app?;
        msg_send![&*app, keyWindow]
    }
}

/// UIContentSizeCategory -> text scale (Dynamic Type body ratios, rounded).
/// "L" is the default.
fn content_size_scale(category: &str) -> f32 {
    match category.trim_start_matches("UICTContentSizeCategory") {
        "XS" => 0.82,
        "S" => 0.88,
        "M" => 0.94,
        "L" => 1.0,
        "XL" => 1.12,
        "XXL" => 1.24,
        "XXXL" => 1.36,
        "AccessibilityM" => 1.6,
        "AccessibilityL" => 1.9,
        "AccessibilityXL" | "AccessibilityXXL" | "AccessibilityXXXL" => 2.0,
        _ => 1.0,
    }
}

/// Feed egui what winit does not report on iOS: safe-area insets that include
/// the keyboard (egui-winit's only cover the notch and home indicator, and
/// only update on resize), the dark/light appearance, and Dynamic Type.
pub(crate) fn raw_input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) {
    ensure_keyboard_observer();
    let Some(window) = key_window() else { return };

    // Appearance and text size: cheap reads, but no need for every frame.
    let now = raw.time.unwrap_or(0.0);
    if now - LAST_POLL.with(Cell::get) > 0.5 {
        LAST_POLL.with(|l| l.set(now));
        let traits = unsafe {
            let tc: Retained<AnyObject> = msg_send![&*window, traitCollection];
            let style: isize = msg_send![&*tc, userInterfaceStyle];
            let cat: Retained<NSString> = msg_send![&*tc, preferredContentSizeCategory];
            (style == 2, content_size_scale(&cat.to_string()))
        };
        TRAITS.with(|t| t.set(traits));
    }
    let (dark, scale) = TRAITS.with(Cell::get);
    let theme = if dark { egui::Theme::Dark } else { egui::Theme::Light };
    if ctx.options(|o| o.fallback_theme) != theme {
        ctx.options_mut(|o| o.fallback_theme = theme);
    }
    if (ctx.zoom_factor() - scale).abs() > 1e-3 {
        ctx.set_zoom_factor(scale);
    }

    // Safe area in UIKit points, plus the keyboard's overlap at the bottom.
    let (insets, height): (UIEdgeInsets, f64) = unsafe {
        let insets: UIEdgeInsets = msg_send![&*window, safeAreaInsets];
        let bounds: CGRect = msg_send![&*window, bounds];
        (insets, bounds.size.height)
    };
    let keyboard = KEYBOARD_TOP
        .with(Cell::get)
        .map_or(0.0, |top| (height - top).max(0.0));
    let zoom = ctx.zoom_factor().max(0.1) as f64;
    let pt = |v: f64| (v / zoom) as f32;
    raw.safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
        left: pt(insets.left),
        top: pt(insets.top),
        right: pt(insets.right),
        bottom: pt(insets.bottom.max(keyboard)),
    }));
}
