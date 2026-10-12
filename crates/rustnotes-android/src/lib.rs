//! Rust Notes — Android shell.
//!
//! android-activity (NativeActivity) drives the lifecycle; this shell owns the
//! EGL context, translates Android key/motion events into core [`CoreEvent`]s,
//! and queries the status-bar inset via JNI. All app logic lives in
//! rustnotes-core.

mod egl;

use android_activity::input::{InputEvent, KeyAction, MotionAction};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use egl::Egl;
use rustnotes_core::{App, CoreEvent, CoreKey, Platform, Renderer};

/// Services the Android shell provides to the core.
struct AndroidPlatform<'a> {
    app: &'a AndroidApp,
}

impl Platform for AndroidPlatform<'_> {
    fn show_keyboard(&self) {
        self.app.show_soft_input(false);
    }
    fn hide_keyboard(&self) {
        self.app.hide_soft_input(false);
    }
}

/// Android AKEYCODE_* values -> ASCII char, honoring shift. Key events are how
/// NativeActivity delivers soft-keyboard input (English/basic latin).
fn key_to_char(code: u32, meta: u32) -> Option<char> {
    let shift = meta & (0x01 | 0x40 | 0x80) != 0; // SHIFT_ON | SHIFT_LEFT_ON | SHIFT_RIGHT_ON
    let c = match code {
        29..=54 => {
            let letter = (b'a' + (code - 29) as u8) as char;
            if shift {
                letter.to_ascii_uppercase()
            } else {
                letter
            }
        }
        7..=16 => {
            let d = (b'0' + (code - 7) as u8) as char;
            if shift {
                match d {
                    '1' => '!',
                    '2' => '@',
                    '3' => '#',
                    '4' => '$',
                    '5' => '%',
                    '6' => '^',
                    '7' => '&',
                    '8' => '*',
                    '9' => '(',
                    _ => ')',
                }
            } else {
                d
            }
        }
        62 => ' ',
        55 => if shift { '<' } else { ',' },
        56 => if shift { '>' } else { '.' },
        68 => if shift { '~' } else { '`' },
        69 => if shift { '_' } else { '-' },
        70 => if shift { '+' } else { '=' },
        71 => if shift { '{' } else { '[' },
        72 => if shift { '}' } else { ']' },
        73 => if shift { '|' } else { '\\' },
        74 => if shift { ':' } else { ';' },
        75 => if shift { '"' } else { '\'' },
        76 => if shift { '?' } else { '/' },
        17 => '*',
        18 => '#',
        77 => '@',
        81 => '+',
        _ => return None,
    };
    Some(c)
}

fn translate_key(code: u32, meta: u32) -> Option<CoreKey> {
    match code {
        4 => Some(CoreKey::Back),
        66 => Some(CoreKey::Enter),
        67 => Some(CoreKey::Backspace),
        21 => Some(CoreKey::Left),
        22 => Some(CoreKey::Right),
        19 => Some(CoreKey::Up),
        20 => Some(CoreKey::Down),
        _ => key_to_char(code, meta).map(CoreKey::Char),
    }
}

/// Post an async JNI query for the system-bar top inset on the Java main
/// thread. Returns the receiver; poll it with `try_recv` from the main loop
/// (never block on it — the Java thread may be parked in a lifecycle barrier).
fn post_top_inset_query(app: &AndroidApp) -> std::sync::mpsc::Receiver<i32> {
    let (tx, rx) = std::sync::mpsc::channel::<i32>();
    let app2 = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr().cast()) };
        let res = vm.attach_current_thread(|env| -> jni::errors::Result<i32> {
            let activity = unsafe {
                jni::objects::JObject::from_raw(
                    env,
                    app2.activity_as_ptr() as jni::sys::jobject,
                )
            };
            let r = (|| -> jni::errors::Result<i32> {
                let window = env
                    .call_method(
                        &activity,
                        jni::jni_str!("getWindow"),
                        jni::jni_sig!("()Landroid/view/Window;"),
                        &[],
                    )?
                    .l()?;
                let decor = env
                    .call_method(
                        &window,
                        jni::jni_str!("getDecorView"),
                        jni::jni_sig!("()Landroid/view/View;"),
                        &[],
                    )?
                    .l()?;
                let insets = env
                    .call_method(
                        &decor,
                        jni::jni_str!("getRootWindowInsets"),
                        jni::jni_sig!("()Landroid/view/WindowInsets;"),
                        &[],
                    )?
                    .l()?;
                let tclass = env.find_class(jni::jni_str!("android/view/WindowInsets$Type"))?;
                let sysbars = env
                    .call_static_method(
                        &tclass,
                        jni::jni_str!("systemBars"),
                        jni::jni_sig!("()I"),
                        &[],
                    )?
                    .i()?;
                let bars = env
                    .call_method(
                        &insets,
                        jni::jni_str!("getInsets"),
                        jni::jni_sig!("(I)Landroid/graphics/Insets;"),
                        &[jni::JValue::from(sysbars)],
                    )?
                    .l()?;
                Ok(env
                    .get_field(&bars, jni::jni_str!("top"), jni::jni_sig!("I"))?
                    .i()?)
            })();
            std::mem::forget(activity);
            r
        });
        let _ = tx.send(res.unwrap_or(0));
    }));
    rx
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("rustnotes")
            .with_max_level(log::LevelFilter::Info),
    );
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));

    let dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp"));
    let mut state = App::new(dir);

    let mut renderer: Option<(Egl, Renderer)> = None;
    let mut destroyed = false;
    let mut input_pending = false;
    let mut inset_rx: Option<std::sync::mpsc::Receiver<i32>> = None;

    while !destroyed {
        app.poll_events(Some(std::time::Duration::from_millis(120)), |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    let rect = app.content_rect();
                    let _ = rect;
                    if let Some(win) = app.native_window() {
                        let ptr = win.ptr().as_ptr() as *mut std::ffi::c_void;
                        match (|| -> Result<(Egl, Renderer), String> {
                            let egl = Egl::new(ptr)?;
                            let gl = unsafe {
                                glow::Context::from_loader_function(
                                    |n| egl.get_proc_address(n) as *const _,
                                )
                            };
                            let (w, h) = egl.surface_size();
                            let r = Renderer::new(gl, w, h)?;
                            Ok((egl, r))
                        })() {
                            Ok((egl, mut r)) => {
                                let (w, h) = egl.surface_size();
                                r.set_size(w, h);
                                log::info!("renderer {w}x{h}, font: {}", r.fonts.source);
                                renderer = Some((egl, r));
                                state.dirty = true;
                                // Kick off the async inset query now that the
                                // window exists.
                                inset_rx = Some(post_top_inset_query(&app));
                            }
                            Err(e) => log::error!("renderer init failed: {e}"),
                        }
                    }
                }
                PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                    if let Some((mut egl, mut r)) = renderer.take() {
                        r.shutdown();
                        egl.destroy();
                    }
                }
                PollEvent::Main(MainEvent::WindowResized { .. })
                | PollEvent::Main(MainEvent::ContentRectChanged { .. }) => {
                    if let Some((egl, r)) = renderer.as_mut() {
                        let (w, h) = egl.surface_size();
                        r.set_size(w, h);
                    }
                    state.dirty = true;
                }
                PollEvent::Main(MainEvent::RedrawNeeded { .. }) => {
                    if let Some((egl, r)) = renderer.as_mut() {
                        let (w, h) = egl.surface_size();
                        r.set_size(w, h);
                        state.draw(r);
                        egl.swap();
                    }
                }
                PollEvent::Main(MainEvent::GainedFocus) => {
                    // inset is queried once below via query_top_inset
                }
                PollEvent::Main(MainEvent::InputAvailable) => input_pending = true,
                PollEvent::Main(MainEvent::Pause) => {
                    state.save_current();
                    state.dirty = true;
                }
                PollEvent::Main(MainEvent::Resume { .. }) => {
                    state.dirty = true;
                }
                PollEvent::Main(MainEvent::Destroy) => destroyed = true,
                _ => {}
            }
        });

        if destroyed {
            break;
        }

        // Drain the async inset query when the Java thread gets to it.
        if let Some(rx) = inset_rx.take() {
            match rx.try_recv() {
                Ok(v) => {
                    let top = v.max(0) as f32;
                    log::info!("system bar top inset: {top}");
                    state.set_content_top(top);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => inset_rx = Some(rx),
                Err(_) => {}
            }
        }

        if input_pending {
            input_pending = false;
            let plat = AndroidPlatform { app: &app };
            if let Ok(mut iter) = app.input_events_iter() {
                loop {
                    let more = iter.next(|ev| {
                        let handled = match ev {
                            InputEvent::MotionEvent(m) => {
                                let action = m.action();
                                let p = m.pointer_at_index(0);
                                let (x, y) = (p.x(), p.y());
                                match action {
                                    MotionAction::Down => Some(CoreEvent::TouchDown { x, y }),
                                    MotionAction::Move => Some(CoreEvent::TouchMove { x, y }),
                                    MotionAction::Up => Some(CoreEvent::TouchUp { x, y }),
                                    MotionAction::Cancel => Some(CoreEvent::TouchCancel),
                                    _ => None,
                                }
                            }
                            InputEvent::KeyEvent(k) => {
                                if k.action() != KeyAction::Down {
                                    None
                                } else {
                                    let code: u32 = k.key_code().into();
                                    let meta = k.meta_state().0;
                                    translate_key(code, meta).map(CoreEvent::Key)
                                }
                            }
                            _ => None,
                        };
                        match handled {
                            Some(ev) => {
                                let r = renderer.as_mut().map(|(_, r)| r);
                                if state.handle_event(ev, &plat, r) {
                                    InputStatus::Handled
                                } else {
                                    InputStatus::Unhandled
                                }
                            }
                            None => InputStatus::Unhandled,
                        }
                    });
                    if !more {
                        break;
                    }
                }
            }
        }

        state.tick();
        if state.dirty {
            if let Some((egl, r)) = renderer.as_mut() {
                state.draw(r);
                egl.swap();
            }
        }
    }

    state.save_current();
    log::info!("android_main exiting");
}
