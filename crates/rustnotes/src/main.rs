//! iOS and desktop entry point. On iOS winit's UIKit backend calls
//! `UIApplicationMain` from here, so this binary is the whole app; package it
//! into an `.app` with `resources/Info.plist` (`just ios`). On desktop it is
//! the fast iteration loop: `cargo run -p rustnotes`.

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    // Not linked into iOS builds (env_logger is desktop-only in Cargo.toml);
    // on a device its output has nowhere to go — panics route through
    // `ios::install_panic_hook` instead.
    #[cfg(not(target_os = "ios"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // The turbofishes are load-bearing: `A` appears in
    // `run`/`native_options`/`data_dir` only inside their bodies (the
    // `A::` consts), never in their signatures, so inference cannot
    // recover it from the argument types.
    rustnotes::run::<rustnotes_core::App>(
        rustnotes::native_options::<rustnotes_core::App>(),
        rustnotes::data_dir::<rustnotes_core::App>(),
    )
}

/// Android runs the cdylib's `android_main`; this binary is never used there.
#[cfg(target_os = "android")]
fn main() {}
