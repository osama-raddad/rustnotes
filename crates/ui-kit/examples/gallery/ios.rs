//! iOS glue for the gallery example: the trimmed share of
//! `crates/rustnotes/src/ios.rs` a demo host needs. The kit installs
//! its own fonts, holds its theme in its own switcher and has nothing
//! to save, so what remains here is what the simulator demo cannot run
//! without: safe-area insets that include the keyboard (egui-winit's
//! only cover the notch and home indicator, and only update on
//! resize), and wgpu limits the iOS Metal devices actually meet. The
//! panic hook is kept for device runs, where stderr goes nowhere.
//!
//! Everything here runs on the main thread: eframe calls
//! `raw_input_hook` from the UIKit run loop, and the notification
//! observer is registered on it.

use eframe::egui;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, define_class, msg_send, sel, AnyThread};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSString};
use objc2_ui_kit::{
    UIEdgeInsets, UIKeyboardFrameEndUserInfoKey, UIKeyboardWillChangeFrameNotification,
};
use std::cell::{Cell, RefCell};

extern "C" {
    /// Foundation's `NSLog`. Declared locally because objc2-foundation 0.3
    /// does not bind it; no extra Cargo feature is needed to link it.
    fn NSLog(format: *mut AnyObject, ...);
}

/// Route panics through NSLog: on a device stderr is not connected to
/// anything a developer can read, so the default hook's output vanishes.
/// Best-effort — this cannot be made provably panic-safe (NSLog allocates),
/// so failures inside the hook just drop the line; see `ns_log`.
pub(crate) fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("panic: {info}");
        ns_log(&msg);
    }));
}

/// `NSLog(@"%@", s)` — the message must be the *argument*, never the format
/// string, because it can contain `%`. Allocation-light: one `format!`, two
/// short NSStrings, no Objective-C-side formatting.
fn ns_log(s: &str) {
    // Never unwind through the panic machinery twice, and never unwind
    // into whatever Objective-C frame might be on the stack.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        let Ok(c) = std::ffi::CString::new(s) else {
            return;
        };
        let fmt: Option<Retained<NSString>> =
            msg_send![class!(NSString), stringWithUTF8String: c"%@".as_ptr()];
        let msg: Option<Retained<NSString>> =
            msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
        if let (Some(fmt), Some(msg)) = (fmt, msg) {
            NSLog(
                Retained::as_ptr(&fmt) as *mut AnyObject,
                Retained::as_ptr(&msg) as *mut AnyObject,
            );
        }
    }));
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
    } else {
        // Release-safe on purpose (not `unreachable!`): if eframe ever stops
        // defaulting to `CreateNew`, the custom limits silently vanish and
        // the app is back to the startup exit this function prevents — warn
        // so the regression is visible.
        log::warn!("eframe WgpuSetup default changed; revisit ios::wgpu_options()");
    }
    cfg
}

define_class!(
    /// Receives keyboard frame notifications; winit exposes none.
    #[unsafe(super(NSObject))]
    #[name = "VynatixKeyboardObserver"]
    #[ivars = ()]
    struct KeyboardObserver;

    impl KeyboardObserver {
        #[unsafe(method(keyboardWillChangeFrame:))]
        fn keyboard_will_change_frame(&self, notification: &NSNotification) {
            // The rect arrives in *screen* coordinates (Apple's documented
            // contract for UIKeyboardFrameEndUserInfoKey); it is converted
            // into window space later, where the key window and its current
            // interface orientation are at hand.
            let frame = unsafe {
                notification.userInfo().and_then(|info| {
                    info.objectForKey(UIKeyboardFrameEndUserInfoKey).map(|v| {
                        let r: CGRect = msg_send![&*v, CGRectValue];
                        r
                    })
                })
            };
            KEYBOARD_FRAME.with(|k| k.set(frame));
        }
    }
);

thread_local! {
    /// Keyboard end frame in *screen* points, from the last notification.
    static KEYBOARD_FRAME: Cell<Option<CGRect>> = const { Cell::new(None) };
    static KEYBOARD_OBSERVER: RefCell<Option<Retained<KeyboardObserver>>> = const { RefCell::new(None) };
}

fn ensure_keyboard_observer() {
    KEYBOARD_OBSERVER.with(|o| {
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

/// Feed egui what winit does not report on iOS: safe-area insets that
/// include the keyboard, in points, converted into egui's zoom space.
/// The gallery holds its theme and type scale itself, so unlike the
/// rustnotes host there is no appearance, Dynamic Type or reduce-motion
/// to poll here.
pub(crate) fn apply_safe_area(ctx: &egui::Context, raw: &mut egui::RawInput) {
    ensure_keyboard_observer();
    // The keyWindow assumption: winit 0.30 creates a legacy UIWindow and
    // makes it key (no UIScene lifecycle), and the Info.plist ships no
    // scene manifest — so the deprecated getter is populated. A winit
    // upgrade adopting scenes would leave it nil and silently disable
    // this glue; a warned nil is better than a wrong inset.
    let Some(window) = key_window() else {
        log::warn!("UIApplication.keyWindow is nil; safe area glue disabled");
        return;
    };

    // Safe area in UIKit points, plus the keyboard's overlap at the bottom.
    let (insets, height): (UIEdgeInsets, f64) = unsafe {
        let insets: UIEdgeInsets = msg_send![&*window, safeAreaInsets];
        let bounds: CGRect = msg_send![&*window, bounds];
        (insets, bounds.size.height)
    };
    // Convert the keyboard rect from screen space into the window's current
    // (possibly rotated, top-left-origin) space — a nil source window means
    // "from screen space" — then clamp so it can never exceed the window or
    // invert egui's content rect.
    let keyboard = KEYBOARD_FRAME.with(Cell::get).map_or(0.0, |frame| {
        let r: CGRect =
            unsafe { msg_send![&*window, convertRect: frame, fromWindow: None::<&AnyObject>] };
        (height - r.origin.y).clamp(0.0, height)
    });
    let zoom = ctx.zoom_factor().max(0.1) as f64;
    let pt = |v: f64| (v / zoom) as f32;
    raw.safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
        left: pt(insets.left),
        top: pt(insets.top),
        right: pt(insets.right),
        bottom: pt(insets.bottom.max(keyboard)),
    }));
}
