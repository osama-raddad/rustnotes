//! iOS platform glue that eframe/winit do not provide. Everything here runs
//! on the main thread: eframe calls `raw_input_hook` from the UIKit run loop,
//! and the notification observers are registered on it.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, define_class, msg_send, sel};
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSString};
use objc2_ui_kit::{
    UIApplicationDidEnterBackgroundNotification, UIApplicationWillResignActiveNotification,
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

/// The system UI font file, resolved through CoreText. iOS devices have no
/// readable font at the core's candidate paths (the simulator masks this,
/// because its file reads resolve against the macOS host file system).
pub(crate) fn system_font_bytes() -> Option<(Vec<u8>, String)> {
    use objc2_core_foundation::{CFURLPathStyle, CFURL};
    use objc2_core_text::{kCTFontURLAttribute, CTFont, CTFontUIFontType};
    unsafe {
        let font = CTFont::new_ui_font_for_language(CTFontUIFontType::System, 0.0, None)
            .unwrap_or_else(|| {
                log::warn!("CoreText could not create the system font");
                None
            })?;
        let attr = font.attribute(kCTFontURLAttribute).unwrap_or_else(|| {
            log::warn!("system font has no URL attribute");
            None
        })?;
        let url = attr.downcast::<CFURL>().ok()?;
        let path = url.file_system_path(CFURLPathStyle::CFURLPOSIXPathStyle)?;
        let path = path.to_string();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                log::warn!("system font unreadable at {path:?}: {e}");
                return None;
            }
        };
        log::debug!("system font resolved at {path}");
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
    #[name = "RNKeyboardObserver"]
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

define_class!(
    /// Saves app state when iOS moves the app away from the foreground.
    /// winit-iOS never maps backgrounding to anything eframe could forward:
    /// `Focused(false)` only fires on `resignKeyWindow` (another window
    /// *within the app* becoming key — not swipe-home, app switcher, call
    /// banners or notification shade), and `App::on_exit` is unreachable
    /// because `UIApplicationMain` never returns. Without these observers
    /// the buffer of an edited note is lost whenever iOS kills the
    /// backgrounded process, so the host observes the lifecycle itself,
    /// exactly as it observes the keyboard.
    #[unsafe(super(NSObject))]
    #[name = "RNLifecycleObserver"]
    #[ivars = ()]
    struct LifecycleObserver;

    impl LifecycleObserver {
        /// One selector serves both notifications; saving twice in a row is
        /// idempotent (`App::save_current` no-ops outside a dirty editor).
        #[unsafe(method(saveState:))]
        fn save_state(&self, _notification: &NSNotification) {
            SAVE_HOOK.with(|h| {
                if let Some(hook) = h.borrow().as_ref() {
                    // Never unwind through the Objective-C dispatcher.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hook()));
                }
            });
        }
    }
);

thread_local! {
    /// Keyboard end frame in *screen* points, from the last notification.
    static KEYBOARD_FRAME: Cell<Option<CGRect>> = const { Cell::new(None) };
    static KEYBOARD_OBSERVER: RefCell<Option<Retained<KeyboardObserver>>> = const { RefCell::new(None) };
    static LIFECYCLE_OBSERVER: RefCell<Option<Retained<LifecycleObserver>>> = const { RefCell::new(None) };
    /// Invoked on WillResignActive / DidEnterBackground; installed by the
    /// host (see `set_lifecycle_save_hook`). `RefCell` because `Box` is not
    /// `Copy`; the box lives for the process (never replaced after setup).
    static SAVE_HOOK: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
    static LAST_POLL: Cell<f64> = const { Cell::new(f64::NEG_INFINITY) };
    static TRAITS: Cell<(bool, f32)> = const { Cell::new((false, 1.0)) };
    /// Reduce motion + UTC offset last read from the system.
    static MOTION_TZ: Cell<(bool, i32)> = const { Cell::new((false, 0)) };
    /// What the host has already been handed (dedupe for `set_*` calls).
    static MOTION_TZ_HANDED: Cell<(bool, i32)> = const { Cell::new((false, 0)) };
    /// Last OS font scale this host enforced as zoom (see `raw_input_hook`).
    static OS_SCALE: Cell<f32> = const { Cell::new(1.0) };
    static SAW_WINDOW: Cell<bool> = const { Cell::new(false) };
    static NIL_WINDOW_WARNED: Cell<bool> = const { Cell::new(false) };
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

fn ensure_lifecycle_observer() {
    LIFECYCLE_OBSERVER.with(|o| {
        if o.borrow().is_some() {
            return;
        }
        let obs: Retained<LifecycleObserver> =
            unsafe { msg_send![super(LifecycleObserver::alloc().set_ivars(())), init] };
        let center = NSNotificationCenter::defaultCenter();
        // TODO(verify-on-device): confirm both notifications arrive before
        // suspension and in the documented order (WillResignActive, then
        // DidEnterBackground); the save is idempotent, so a missing or
        // reordered one is safe.
        for name in [
            UIApplicationWillResignActiveNotification,
            UIApplicationDidEnterBackgroundNotification,
        ] {
            unsafe {
                center.addObserver_selector_name_object(&obs, sel!(saveState:), Some(name), None);
            }
        }
        *o.borrow_mut() = Some(obs);
    });
}

/// Install the callback invoked whenever iOS moves the app away from the
/// foreground. Must run on the main thread (the observer is registered
/// there); the host does that from its app-creation closure, which is the
/// first main-thread code eframe runs.
pub(crate) fn set_lifecycle_save_hook(hook: impl Fn() + 'static) {
    ensure_lifecycle_observer();
    SAVE_HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
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
/// "L" is the default. The accessibility tiers follow UIKit's body ratios
/// (~2.19/2.35/2.65) rather than collapsing to one cap.
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
        "AccessibilityXL" => 2.19,
        "AccessibilityXXL" => 2.35,
        "AccessibilityXXXL" => 2.65,
        _ => 1.0,
    }
}

/// Feed egui what winit does not report on iOS: safe-area insets that
/// include the keyboard (egui-winit's only cover the notch and home
/// indicator, and only update on resize), the dark/light appearance,
/// Dynamic Type, plus the reduce-motion preference and UTC offset for the
/// host to apply to the core app. Returns that pair only when it changed.
pub(crate) fn raw_input_hook(ctx: &egui::Context, raw: &mut egui::RawInput) -> Option<(bool, i32)> {
    ensure_keyboard_observer();
    // The keyWindow assumption: winit 0.30 creates a legacy UIWindow and
    // makes it key (no UIScene lifecycle), and Info.plist ships no scene
    // manifest — so the deprecated getter is populated. A winit upgrade
    // adopting scenes would leave it nil and silently disable ALL of this
    // glue; the one-shot warning below after warm-up is the tripwire.
    let Some(window) = key_window() else {
        if SAW_WINDOW.with(Cell::get) && !NIL_WINDOW_WARNED.with(Cell::get) {
            NIL_WINDOW_WARNED.with(|w| w.set(true));
            log::warn!(
                "UIApplication.keyWindow is nil after warm-up; safe area, \
                 keyboard, appearance and Dynamic Type glue disabled"
            );
        }
        return None;
    };
    SAW_WINDOW.with(|s| s.set(true));

    // Appearance and text size: cheap reads, but no need for every frame.
    // A missing clock still advances the gate instead of freezing the poll.
    let now = raw.time.unwrap_or_else(|| LAST_POLL.with(Cell::get) + 1.0);
    if now - LAST_POLL.with(Cell::get) > 0.5 {
        LAST_POLL.with(|l| l.set(now));
        let traits = unsafe {
            let tc: Retained<AnyObject> = msg_send![&*window, traitCollection];
            let style: isize = msg_send![&*tc, userInterfaceStyle];
            let cat: Retained<NSString> = msg_send![&*tc, preferredContentSizeCategory];
            (style == 2, content_size_scale(&cat.to_string()))
        };
        TRAITS.with(|t| t.set(traits));
        // Reduce motion (UIAccessibility) and the zone offset, from the
        // same poll; NSTimeZone is reached through raw msg_send because the
        // crate does not enable objc2-foundation's NSTimeZone feature.
        let motion_tz = unsafe {
            let reduce: bool = msg_send![class!(UIAccessibility), isReduceMotionEnabled];
            let tz: Retained<AnyObject> = msg_send![class!(NSTimeZone), systemTimeZone];
            let secs: isize = msg_send![&*tz, secondsFromGMT];
            (reduce, secs as i32)
        };
        MOTION_TZ.with(|m| m.set(motion_tz));
    }
    let (dark, scale) = TRAITS.with(Cell::get);
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    if ctx.options(|o| o.fallback_theme) != theme {
        ctx.options_mut(|o| o.fallback_theme = theme);
    }
    // Snap zoom to the OS font scale only when the OS-reported scale itself
    // changed; otherwise a user's pinch-zoom persists until the setting
    // changes. (Zoom remains the carrier for Dynamic Type — egui has no
    // separate font-size input.)
    let zoom = if (OS_SCALE.with(Cell::get) - scale).abs() > 1e-3 {
        OS_SCALE.with(|o| o.set(scale));
        ctx.set_zoom_factor(scale);
        // `set_zoom_factor` takes effect in this pass's begin_pass while
        // `zoom_factor()` still reports the old value this frame — divide
        // the insets by the value this pass enforces.
        scale.max(0.1) as f64
    } else {
        ctx.zoom_factor().max(0.1) as f64
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
    // TODO(verify-on-device): landscape orientations and docked/minimized
    // (hardware-keyboard) frames.
    let keyboard = KEYBOARD_FRAME.with(Cell::get).map_or(0.0, |frame| {
        let r: CGRect =
            unsafe { msg_send![&*window, convertRect:fromWindow:, frame, None::<&AnyObject>] };
        (height - r.origin.y).clamp(0.0, height)
    });
    let pt = |v: f64| (v / zoom) as f32;
    raw.safe_area_insets = Some(egui::SafeAreaInsets(egui::epaint::MarginF32 {
        left: pt(insets.left),
        top: pt(insets.top),
        right: pt(insets.right),
        bottom: pt(insets.bottom.max(keyboard)),
    }));

    let cur = MOTION_TZ.with(Cell::get);
    if cur != MOTION_TZ_HANDED.with(Cell::get) {
        MOTION_TZ_HANDED.with(|h| h.set(cur));
        return Some(cur);
    }
    None
}
