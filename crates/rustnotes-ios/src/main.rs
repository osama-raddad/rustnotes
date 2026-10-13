//! Rust Notes — iOS shell, 100% Rust via objc2 (no Swift, no ObjC source).
//!
//! UIApplicationMain drives everything from `fn main()`. The app delegate
//! creates the window; the view controller hosts a CAEAGLLayer-backed GLES3
//! context (hand-rolled EAGL, mirroring the Android shell's hand-rolled EGL),
//! a CADisplayLink frame loop, UIResponder touch forwarding, and a hidden
//! UITextField that summons the keyboard and feeds us key events.
#![allow(deprecated)] // Retained::cast: safe upcasts to AnyObject
#![allow(non_snake_case)] // method names mirror their ObjC selectors

use objc2::rc::Retained;
use objc2::rc::Allocated;
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
use objc2::{
    class, define_class, extern_class, extern_conformance, extern_methods, msg_send, msg_send_id,
    sel, AnyThread, ClassType, MainThreadMarker, MainThreadOnly,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    NSArray, NSDictionary, NSNumber, NSRange, NSObject, NSString, NSNotification,
    NSNotificationCenter, NSRunLoop, NSSet, NSTimer,
};
use objc2_quartz_core::{CADisplayLink, CALayer};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey, UIColor, UIEvent,
    UIKeyboardFrameEndUserInfoKey, UIKeyboardWillChangeFrameNotification, UIScreen,
    UITextField, UITextFieldDelegate, UITouch, UIViewController, UIWindow,
};
use rustnotes_core::{App, CoreEvent, CoreKey, Platform};
use std::sync::Arc;
use glow::HasContext as _;

fn main() {
    ios_log_init();
    let _mtm = MainThreadMarker::new().expect("must run on the main thread");
    unsafe {
        // Force class registration before the name-based lookup in UIKit.
        let _ = RNAppDelegate::class();
        let _ = RNViewController::class();
        objc2_ui_kit::UIApplicationMain(
            0,
            std::ptr::NonNull::dangling(),
            None,
            Some(&NSString::from_str("RNAppDelegate")),
        );
    }
}

// ---------------------------------------------------------------- logging

unsafe extern "C" {
    fn NSLog(format: *mut AnyObject, ...);
}

fn nslog(s: &str) {
    let fmt = NSString::from_str("%@");
    let msg = NSString::from_str(s);
    unsafe {
        NSLog(
            &*fmt as *const NSString as *mut AnyObject,
            &*msg as *const NSString as *mut AnyObject,
        );
    }
}

struct IosLogger;
impl log::Log for IosLogger {
    fn enabled(&self, _m: &log::Metadata) -> bool {
        true
    }
    fn log(&self, r: &log::Record) {
        nslog(&format!("{}", r.args()));
    }
    fn flush(&self) {}
}

/// UIContentSizeCategory -> text scale (Apple's Dynamic Type body ratios,
/// rounded). "L" is the default.
fn content_size_scale(category: &str) -> f32 {
    let c = category.trim_start_matches("UICTContentSizeCategory");
    match c {
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

fn ios_log_init() {
    static LOGGER: IosLogger = IosLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}

// ---------------------------------------------------------------- shell state

struct Shell {
    app: App,
    gl: glow::Context, // the shell's handle for the FBO (the core painter has its own)
    eagl: Retained<EAGLContext>,
    layer: Retained<CALayer>,
    fb: glow::NativeFramebuffer,
    rb: glow::NativeRenderbuffer,
    scale: f32,
    kb_bottom_pts: f32, // keyboard overlap over the view, in points
    view_h_pts: f32,
}

thread_local! {
    static SHELL: std::cell::RefCell<Option<Shell>> = const { std::cell::RefCell::new(None) };
    // Keyboard notifications fire re-entrantly (from become/resignFirstResponder
    // inside a SHELL borrow) — record here, apply on the next frame tick.
    static PENDING_KB: std::cell::Cell<(f64, bool)> = const { std::cell::Cell::new((f64::MAX, false)) };
    // Kept separate so Platform callbacks never re-enter SHELL (RefCell).
    static TEXTFIELD: std::cell::RefCell<Option<Retained<UITextField>>> =
        const { std::cell::RefCell::new(None) };
    static WINDOW: std::cell::RefCell<Option<Retained<UIWindow>>> = const { std::cell::RefCell::new(None) };
    static KEEPALIVE: std::cell::RefCell<Vec<Retained<AnyObject>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn with_shell<R>(f: impl FnOnce(&mut Shell) -> R) -> Option<R> {
    SHELL.with(|s| {
        let mut b = s.borrow_mut();
        b.as_mut().map(f)
    })
}

struct IosPlatform;

impl Platform for IosPlatform {
    fn show_keyboard(&self) {
        TEXTFIELD.with(|t| {
            if let Some(tf) = t.borrow().as_ref() {
                let _ = tf.becomeFirstResponder();
            }
        });
    }
    fn hide_keyboard(&self) {
        TEXTFIELD.with(|t| {
            if let Some(tf) = t.borrow().as_ref() {
                let _ = tf.resignFirstResponder();
            }
        });
    }
}

// ---------------------------------------------------------------- GL helpers

unsafe fn make_gl() -> glow::Context {
    glow::Context::from_loader_function(|name| {
        let mut buf = Vec::with_capacity(name.len() + 1);
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        unsafe { libc::dlsym(libc::RTLD_DEFAULT, buf.as_ptr().cast()) as *const _ }
    })
}

// ---------------------------------------------------------------- EAGL (typed)

extern_class!(
    #[unsafe(super(NSObject))]
    #[name = "EAGLContext"]
    pub struct EAGLContext;
);

extern_conformance!(
    unsafe impl NSObjectProtocol for EAGLContext {}
);

extern_methods!(
    unsafe impl EAGLContext {
        #[unsafe(method(initWithAPI:))]
        pub unsafe fn initWithAPI(this: Allocated<Self>, api: usize) -> Retained<Self>;

        #[unsafe(method(setCurrentContext:))]
        pub unsafe fn setCurrentContext(context: Option<&Self>) -> bool;

        #[unsafe(method(renderbufferStorage:fromDrawable:))]
        pub unsafe fn renderbufferStorage_fromDrawable(
            &self,
            target: usize,
            drawable: Option<&AnyObject>,
        ) -> bool;

        #[unsafe(method(presentRenderbuffer:))]
        pub unsafe fn presentRenderbuffer(&self, renderbuffer: usize) -> bool;
    }
);

// ---------------------------------------------------------------- ObjC classes

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "RNAppDelegate"]
    #[ivars = ()]
    struct RNAppDelegate;

    unsafe impl NSObjectProtocol for RNAppDelegate {}

    unsafe impl UIApplicationDelegate for RNAppDelegate {
        #[unsafe(method(application:didFinishLaunchingWithOptions:))]
        unsafe fn application_didFinishLaunchingWithOptions(
            &self,
            _application: &UIApplication,
            _launch_options: Option<&NSDictionary<UIApplicationLaunchOptionsKey, AnyObject>>,
        ) -> bool {
            let mtm = MainThreadMarker::new().unwrap();
            let screen = UIScreen::mainScreen(mtm);
            let bounds = screen.bounds();
            let window = UIWindow::initWithFrame(UIWindow::alloc(mtm), bounds);
            let vc = RNViewController::new();
            window.setRootViewController(Some(&vc));
            window.makeKeyAndVisible();
            WINDOW.with(|w| *w.borrow_mut() = Some(window));
            nslog("didFinishLaunching: window ready");
            true
        }

        #[unsafe(method(applicationWillResignActive:))]
        fn applicationWillResignActive(&self, _application: &UIApplication) {
            with_shell(|s| s.app.save_current());
        }

        #[unsafe(method(applicationDidEnterBackground:))]
        fn applicationDidEnterBackground(&self, _application: &UIApplication) {
            with_shell(|s| s.app.save_current());
        }
    }
);

define_class!(
    #[unsafe(super(UIViewController))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "RNViewController"]
    #[ivars = ()]
    struct RNViewController;

    unsafe impl NSObjectProtocol for RNViewController {}

    impl RNViewController {
        #[unsafe(method(rnStep:))]
        unsafe fn rn_step(&self, _dl: &CADisplayLink) {
            self.rn_tick();
        }

        #[unsafe(method(rnTimerFire:))]
        fn rn_timer_fire(&self, _t: &NSTimer) {
            self.rn_tick();
        }

        #[unsafe(method(rnKeyboardWillChangeFrame:))]
        unsafe fn rn_keyboard_will_change_frame(&self, notification: &NSNotification) {
            unsafe {
                let frame = notification.userInfo().and_then(|info| {
                    info.objectForKey(&UIKeyboardFrameEndUserInfoKey).map(|v| {
                        let r: CGRect = msg_send![&*v, CGRectValue];
                        r
                    })
                });
                let kb_top = frame.map(|f| f.origin.y);
                PENDING_KB.with(|p| p.set((kb_top.unwrap_or(f64::MAX), kb_top.is_some())));
            }
        }

        #[unsafe(method(viewDidLoad))]
        unsafe fn rn_view_did_load(&self) {
            unsafe {
                self.rn_setup();
            }
        }

        #[unsafe(method(viewDidLayoutSubviews))]
        unsafe fn rn_view_did_layout(&self) {
            unsafe {
                let bounds = self.view().expect("view").bounds();
                let (w_pts, h_pts) = (bounds.size.width, bounds.size.height);
                with_shell(|s| s.view_h_pts = h_pts as f32);
                // Always re-run: CAEAGLLayer only grants storage once the
                // layer is attached to a window (viewDidLoad is too early).
                self.rn_resize(w_pts, h_pts);
                self.rn_apply_insets();
            }
        }

        #[unsafe(method(touchesBegan:withEvent:))]
        unsafe fn rn_touches_began(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            unsafe { self.rn_touch(touches, 0) }
        }

        #[unsafe(method(touchesMoved:withEvent:))]
        unsafe fn rn_touches_moved(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            unsafe { self.rn_touch(touches, 1) }
        }

        #[unsafe(method(touchesEnded:withEvent:))]
        unsafe fn rn_touches_ended(&self, touches: &NSSet<UITouch>, _event: Option<&UIEvent>) {
            unsafe { self.rn_touch(touches, 2) }
        }

        #[unsafe(method(touchesCancelled:withEvent:))]
        unsafe fn rn_touches_cancelled(
            &self,
            _touches: &NSSet<UITouch>,
            _event: Option<&UIEvent>,
        ) {
            with_shell(|s| {
                s.app.handle_event(CoreEvent::TouchCancel, &IosPlatform);
            });
        }
    }

    unsafe impl UITextFieldDelegate for RNViewController {
        #[unsafe(method(textField:shouldChangeCharactersInRange:replacementString:))]
        unsafe fn textField_shouldChangeCharactersInRange_replacementString(
            &self,
            _tf: &UITextField,
            range: NSRange,
            string: &NSString,
        ) -> bool {
            let mut n = range.length;
            while n > 0 {
                with_shell(|s| {
                    s.app
                        .handle_event(CoreEvent::Key(CoreKey::Backspace), &IosPlatform)
                });
                n -= 1;
            }
            for c in string.to_string().chars() {
                if c == '\n' {
                    continue;
                }
                with_shell(|s| {
                    s.app.handle_event(CoreEvent::Key(CoreKey::Char(c)), &IosPlatform)
                });
            }
            false // we mirror edits into our own buffer
        }

        #[unsafe(method(textFieldShouldReturn:))]
        unsafe fn textFieldShouldReturn(&self, _tf: &UITextField) -> bool {
            with_shell(|s| {
                s.app
                    .handle_event(CoreEvent::Key(CoreKey::Enter), &IosPlatform)
            });
            false
        }
    }
);

impl RNViewController {
    fn rn_tick(&self) {
        // Apply any keyboard frame recorded by the (re-entrant) notification.
        let (kb_top, has) = PENDING_KB.with(|p| p.replace((f64::MAX, false)));
        if has {
            with_shell(|s| {
                let h = s.view_h_pts as f64;
                s.kb_bottom_pts = if kb_top < h { ((h - kb_top).max(0.0)) as f32 } else { 0.0 };
            });
            self.rn_apply_insets();
        }
        self.rn_poll_appearance();
        with_shell(|s| {
            s.app.tick();
            if s.app.dirty {
                s.render();
            }
        });
    }

    /// Feed the OS appearance and Dynamic Type size to the core. Polled
    /// (cheap trait-collection reads); the core ignores unchanged values.
    fn rn_poll_appearance(&self) {
        let Some(view) = self.view() else { return };
        let (dark, scale) = unsafe {
            let tc: Retained<AnyObject> = msg_send![&*view, traitCollection];
            let style: isize = msg_send![&*tc, userInterfaceStyle];
            let cat: Retained<NSString> = msg_send![&*tc, preferredContentSizeCategory];
            (style == 2, content_size_scale(&cat.to_string()))
        };
        with_shell(|s| {
            s.app.set_appearance(dark);
            s.app.set_text_scale(scale);
        });
    }

    fn rn_apply_insets(&self) {
        let insets = self.view().expect("view").safeAreaInsets();
        with_shell(|s| {
            s.app.set_content_top(insets.top as f32 * s.scale);
            let bottom_pts = s.kb_bottom_pts.max(insets.bottom as f32);
            s.app.set_content_bottom(bottom_pts as f32 * s.scale);
        });
    }

    fn new() -> Retained<Self> {
        let mtm = MainThreadMarker::new().unwrap();
        unsafe {
            msg_send![
                super(Self::alloc(mtm).set_ivars(()), UIViewController::class()),
                init
            ]
        }
    }

    unsafe fn rn_touch(&self, touches: &NSSet<UITouch>, phase: u8) {
        let Some(touch) = touches.anyObject() else { return };
        let view = self.view().expect("view");
        let loc = touch.locationInView(Some(&view));
        let ev = with_shell(|s| {
            let x = loc.x as f32 * s.scale;
            let y = loc.y as f32 * s.scale;
            match phase {
                0 => CoreEvent::TouchDown { x, y },
                1 => CoreEvent::TouchMove { x, y },
                _ => CoreEvent::TouchUp { x, y },
            }
        });
        if let Some(ev) = ev {
            with_shell(|s| s.app.handle_event(ev, &IosPlatform));
        }
    }

    unsafe fn rn_setup(&self) {
        unsafe {
            let view = self.view().expect("view loaded");
            view.setBackgroundColor(Some(&UIColor::whiteColor()));
            let mtm = MainThreadMarker::new().unwrap();
            let screen = UIScreen::mainScreen(mtm);
            let scale = screen.scale() as f32;
            let bounds = view.bounds();
            nslog(&format!(
                "setup: {:.0}x{:.0}pt @ {scale}x",
                bounds.size.width, bounds.size.height
            ));

            // --- CAEAGLLayer (GLES drawable)
            let layer: Retained<CALayer> = msg_send![class!(CAEAGLLayer), new];
            let layer_obj: Retained<AnyObject> = Retained::cast(layer.clone());
            let objs: [Retained<AnyObject>; 2] = [
                Retained::cast(NSString::from_str("RGBA8")),
                Retained::cast(NSNumber::numberWithBool(false)),
            ];
            let keys: [Retained<AnyObject>; 2] = [
                Retained::cast(NSString::from_str("kEAGLDrawablePropertyColorFormat")),
                Retained::cast(NSString::from_str("kEAGLDrawablePropertyRetainedBacking")),
            ];
            let objs_arr = NSArray::from_retained_slice(&objs);
            let keys_arr = NSArray::from_retained_slice(&keys);
            let props: Retained<NSObject> =
                msg_send![class!(NSDictionary), dictionaryWithObjects: &*objs_arr forKeys: &*keys_arr];
            let _: () = msg_send![&*layer_obj, setDrawableProperties: &*props];
            layer.setContentsScale(screen.scale());
            view.layer().addSublayer(&layer);

            // --- EAGL context (GLES3)
            let eagl = EAGLContext::initWithAPI(EAGLContext::alloc(), 3);
            assert!(EAGLContext::setCurrentContext(Some(&eagl)), "EAGL setCurrent");

            // --- GL frame/renderbuffer
            let gl = make_gl();
            let fb = gl.create_framebuffer().expect("fb");
            let rb = gl.create_renderbuffer().expect("rb");

            // --- core app
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let dir = std::path::PathBuf::from(home).join("Documents");
            let app = App::new(dir);

            SHELL.with(|s| {
                *s.borrow_mut() = Some(Shell {
                    app,
                    gl,
                    eagl,
                    layer,
                    fb,
                    rb,
                    scale,
                    kb_bottom_pts: 0.0,
                    view_h_pts: bounds.size.height as f32,
                })
            });

            self.rn_resize(bounds.size.width, bounds.size.height);

            // --- egui painter (second GL handle for the core)
            with_shell(|s| {
                let (w, h) = s.surface_size_px();
                match s.app.attach_gl(Arc::new(make_gl())) {
                    Ok(()) => {
                        s.app.set_viewport(w, h, Some(s.scale));
                        nslog(&format!("renderer {w}x{h}, font: {}", s.app.font_source()));
                    }
                    Err(e) => nslog(&format!("renderer init failed: {e}")),
                }
            });

            // --- hidden text field = keyboard input
            let tf = UITextField::new(mtm);
            tf.setAlpha(0.02);
            view.addSubview(&tf);
            let del: &ProtocolObject<dyn UITextFieldDelegate> = ProtocolObject::from_ref(self);
            tf.setDelegate(Some(del));
            TEXTFIELD.with(|t| *t.borrow_mut() = Some(tf));

            // --- display link frame loop
            let target: &AnyObject = &*(self as *const Self as *const AnyObject);
            let dl: Retained<CADisplayLink> = msg_send_id![
                class!(CADisplayLink),
                displayLinkWithTarget: target
                selector: sel!(rnStep:)
            ];
            let rl = NSRunLoop::currentRunLoop();
            let common = NSString::from_str("kCFRunLoopCommonModes");
            dl.addToRunLoop_forMode(&rl, common.as_ref());
            KEEPALIVE.with(|k| k.borrow_mut().push(Retained::cast(dl)));

            // --- NSTimer fallback: display links can stall on headless sims
            let timer_target: &AnyObject = &*(self as *const Self as *const AnyObject);
            let timer: Retained<NSTimer> = msg_send_id![
                class!(NSTimer),
                scheduledTimerWithTimeInterval: 0.03f64
                target: timer_target
                selector: sel!(rnTimerFire:)
                userInfo: std::ptr::null::<AnyObject>()
                repeats: true
            ];
            KEEPALIVE.with(|k| k.borrow_mut().push(Retained::cast(timer)));

            // --- keyboard frame notifications
            let observer: &AnyObject = &*(self as *const Self as *const AnyObject);
            NSNotificationCenter::defaultCenter()
                .addObserver_selector_name_object(
                    observer,
                    sel!(rnKeyboardWillChangeFrame:),
                    Some(&UIKeyboardWillChangeFrameNotification),
                    None,
                );
        }
    }

    unsafe fn rn_resize(&self, w_pts: f64, h_pts: f64) {
        unsafe {
            let frame = CGRect {
                origin: CGPoint::new(0.0, 0.0),
                size: CGSize::new(w_pts, h_pts),
            };
            with_shell(|s| {
                let _: () = msg_send![&*s.layer, setFrame: frame];
                // Rebind renderbuffer storage to the resized layer.
                let layer_obj: Retained<AnyObject> = Retained::cast(s.layer.clone());
                s.gl.bind_renderbuffer(glow::RENDERBUFFER, Some(s.rb));
                s.eagl.renderbufferStorage_fromDrawable(glow::RENDERBUFFER as usize, None);
                let ok = s.eagl.renderbufferStorage_fromDrawable(
                    glow::RENDERBUFFER as usize,
                    Some(&*layer_obj),
                );
                if !ok {
                    nslog("renderbufferStorage failed");
                }
                s.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(s.fb));
                s.gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::RENDERBUFFER,
                    Some(s.rb),
                );
                let status = s.gl.check_framebuffer_status(glow::FRAMEBUFFER);
                if status != glow::FRAMEBUFFER_COMPLETE {
                    nslog(&format!("FBO incomplete: 0x{status:x}"));
                }
                let (w, h) = s.surface_size_px();
                s.app.set_viewport(w, h, Some(s.scale));
                s.app.dirty = true;
            });
        }
    }
}

impl Shell {
    fn surface_size_px(&self) -> (i32, i32) {
        let frame: CGRect = unsafe { msg_send![&*self.layer, bounds] };
        (
            (frame.size.width as f32 * self.scale) as i32,
            (frame.size.height as f32 * self.scale) as i32,
        )
    }

    fn render(&mut self) {
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fb));
            self.app.frame(&IosPlatform);
            let ok = self.eagl.presentRenderbuffer(self.rb.0.get() as usize);
            if !ok {
                nslog("presentRenderbuffer failed");
            }
        }
    }
}
