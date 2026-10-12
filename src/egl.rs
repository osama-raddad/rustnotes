//! Minimal hand-rolled EGL 1.4 bindings for creating an OpenGL ES 3 context
//! on an Android ANativeWindow. We link directly against the system libEGL.so.

use std::ffi::c_void;
use std::ptr;

pub type EGLDisplay = *mut c_void;
pub type EGLSurface = *mut c_void;
pub type EGLContext = *mut c_void;
pub type EGLConfig = *mut c_void;

const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_WINDOW_BIT: i32 = 0x0004;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_OPENGL_ES3_BIT: i32 = 0x0040;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_NONE: i32 = 0x3038;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;

#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(native_display: *mut c_void) -> EGLDisplay;
    fn eglInitialize(dpy: EGLDisplay, major: *mut i32, minor: *mut i32) -> u32;
    fn eglTerminate(dpy: EGLDisplay) -> u32;
    fn eglChooseConfig(
        dpy: EGLDisplay,
        attribs: *const i32,
        configs: *mut EGLConfig,
        num_configs: i32,
        num_match: *mut i32,
    ) -> u32;
    fn eglCreateWindowSurface(
        dpy: EGLDisplay,
        config: EGLConfig,
        window: *mut c_void,
        attribs: *const i32,
    ) -> EGLSurface;
    fn eglDestroySurface(dpy: EGLDisplay, surface: EGLSurface) -> u32;
    fn eglCreateContext(
        dpy: EGLDisplay,
        config: EGLConfig,
        share: EGLContext,
        attribs: *const i32,
    ) -> EGLContext;
    fn eglDestroyContext(dpy: EGLDisplay, ctx: EGLContext) -> u32;
    fn eglMakeCurrent(dpy: EGLDisplay, draw: EGLSurface, read: EGLSurface, ctx: EGLContext) -> u32;
    fn eglSwapBuffers(dpy: EGLDisplay, surface: EGLSurface) -> u32;
    fn eglSwapInterval(dpy: EGLDisplay, interval: i32) -> u32;
    fn eglQuerySurface(dpy: EGLDisplay, surface: EGLSurface, attribute: i32, value: *mut i32) -> u32;
    fn eglGetProcAddress(name: *const u8) -> *mut c_void;
    fn eglGetError() -> i32;
}

pub struct Egl {
    pub display: EGLDisplay,
    pub context: EGLContext,
    pub surface: EGLSurface,
}

fn check(ok: u32, what: &str) -> Result<(), String> {
    if ok == 0 {
        Err(format!("{what} failed: EGL error 0x{:x}", unsafe {
            eglGetError()
        }))
    } else {
        Ok(())
    }
}

impl Egl {
    /// Create a GLES3 context + window surface on the given ANativeWindow.
    pub fn new(window: *mut c_void) -> Result<Egl, String> {
        unsafe {
            let dpy = eglGetDisplay(ptr::null_mut()); // EGL_DEFAULT_DISPLAY
            if dpy.is_null() {
                return Err("eglGetDisplay failed".into());
            }
            check(
                eglInitialize(dpy, ptr::null_mut(), ptr::null_mut()),
                "eglInitialize",
            )?;

            let attribs = [
                EGL_SURFACE_TYPE,
                EGL_WINDOW_BIT,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES3_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_NONE,
            ];
            let mut config: EGLConfig = ptr::null_mut();
            let mut n: i32 = 0;
            check(
                eglChooseConfig(dpy, attribs.as_ptr(), &mut config, 1, &mut n),
                "eglChooseConfig",
            )?;
            if n < 1 {
                return Err("no matching EGL config".into());
            }

            let surface = eglCreateWindowSurface(dpy, config, window, ptr::null());
            if surface.is_null() {
                return Err(format!("eglCreateWindowSurface failed: 0x{:x}", eglGetError()));
            }

            let ctx_attribs = [EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE];
            let context = eglCreateContext(dpy, config, ptr::null_mut(), ctx_attribs.as_ptr());
            if context.is_null() {
                return Err(format!("eglCreateContext failed: 0x{:x}", eglGetError()));
            }

            check(
                eglMakeCurrent(dpy, surface, surface, context),
                "eglMakeCurrent",
            )?;
            // vsync
            eglSwapInterval(dpy, 1);

            Ok(Egl {
                display: dpy,
                context,
                surface,
            })
        }
    }

    pub fn get_proc_address(&self, name: &str) -> *mut c_void {
        let mut buf = Vec::with_capacity(name.len() + 1);
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        unsafe { eglGetProcAddress(buf.as_ptr()) }
    }

    pub fn surface_size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe {
            eglQuerySurface(self.display, self.surface, EGL_WIDTH, &mut w);
            eglQuerySurface(self.display, self.surface, EGL_HEIGHT, &mut h);
        }
        (w, h)
    }

    pub fn swap(&self) {
        unsafe { eglSwapBuffers(self.display, self.surface) };
    }

    pub fn destroy(&mut self) {
        unsafe {
            eglMakeCurrent(self.display, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
            eglDestroySurface(self.display, self.surface);
            eglDestroyContext(self.display, self.context);
            eglTerminate(self.display);
        }
        self.surface = ptr::null_mut();
        self.context = ptr::null_mut();
        self.display = ptr::null_mut();
    }
}
