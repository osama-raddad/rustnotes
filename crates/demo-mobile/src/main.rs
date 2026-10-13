//! iOS and desktop entry point, mirroring the other apps': on iOS winit's
//! UIKit backend calls `UIApplicationMain` from here, so this binary is
//! the whole app; on desktop it is the demo's iteration loop
//! (`cargo run -p demo-mobile`). Android runs the cdylib's
//! `android_main` from the lib instead.

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    // Desktop-only logging, as in the other apps: on iOS the output has
    // nowhere to go — panics route through the host's NSLog hook.
    #[cfg(not(target_os = "ios"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // The turbofishes are load-bearing: `A` appears in
    // `run`/`native_options`/`data_dir` only inside their bodies (the
    // `A::` consts), never in their signatures, so inference cannot
    // recover it from the argument types.
    rustnotes::run::<demo_mobile::DemoApp>(
        rustnotes::native_options::<demo_mobile::DemoApp>(),
        rustnotes::data_dir::<demo_mobile::DemoApp>(),
    )
}

/// Android runs the cdylib's `android_main` (in the lib's `android`
/// module); this binary is never used there.
#[cfg(target_os = "android")]
fn main() {}
