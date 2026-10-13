//! iOS and desktop entry point. On iOS winit's UIKit backend calls
//! `UIApplicationMain` from here, so this binary is the whole app; package it
//! into an `.app` with `resources/Info.plist` (`just ios`). On desktop it is
//! the fast iteration loop: `cargo run -p rustnotes`.

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    rustnotes::run(rustnotes::native_options(), data_dir())
}

/// Android runs the cdylib's `android_main`; this binary is never used there.
#[cfg(target_os = "android")]
fn main() {}

/// Where the notes database lives.
#[cfg(not(target_os = "android"))]
fn data_dir() -> std::path::PathBuf {
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()));
    if cfg!(target_os = "ios") {
        home.join("Documents") // the app container's Documents directory
    } else {
        home.join(".rustnotes") // desktop development data
    }
}
