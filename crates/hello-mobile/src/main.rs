//! iOS and desktop entry point, mirroring rustnotes' own: on iOS winit's
//! UIKit backend calls `UIApplicationMain` from here, so this binary is
//! the whole app; on desktop it is the proof's iteration loop
//! (`cargo run -p hello-mobile`). Android runs the cdylib's
//! `android_main` from the lib instead.

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    // Desktop-only logging, as in rustnotes: on iOS the output has
    // nowhere to go — panics route through the host's NSLog hook.
    #[cfg(not(target_os = "ios"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // The turbofishes are load-bearing: `A` appears in
    // `run`/`native_options`/`data_dir` only inside their bodies (the
    // `A::` consts), never in their signatures, so inference cannot
    // recover it from the argument types.
    rustnotes::run::<hello_mobile::HelloApp>(
        rustnotes::native_options::<hello_mobile::HelloApp>(),
        rustnotes::data_dir::<hello_mobile::HelloApp>(),
    )
}

/// Android runs the cdylib's `android_main` (in the lib's `android`
/// module); this binary is never used there.
#[cfg(target_os = "android")]
fn main() {}
