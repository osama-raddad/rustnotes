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
    rustnotes::run(rustnotes::native_options(), data_dir())
}

/// Android runs the cdylib's `android_main`; this binary is never used there.
#[cfg(target_os = "android")]
fn main() {}

/// Where the notes database lives.
#[cfg(not(target_os = "android"))]
fn data_dir() -> std::path::PathBuf {
    // launchd always sets HOME in the app container; a silent fallback would
    // point at an unwritable path and fail every save with an opaque fs
    // error, so fail loudly instead.
    #[cfg(target_os = "ios")]
    let home = std::path::PathBuf::from(
        std::env::var("HOME").expect("HOME must be set in the app container"),
    );
    // Desktop: a missing HOME goes to the OS temp dir, not the process cwd
    // (which may be read-only or litter an unrelated directory).
    #[cfg(not(target_os = "ios"))]
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    if cfg!(target_os = "ios") {
        home.join("Documents") // the app container's Documents directory
    } else {
        home.join(".rustnotes") // desktop development data
    }
}
