//! iOS and desktop entry point.
//!
//! iOS: winit's UIKit backend calls `UIApplicationMain` from this `main`, so
//! the binary is the whole app. Package it into an `.app` with an Info.plist
//! (CFBundleExecutable = mobile_app) and `codesign`; no Xcode project needed.
//! eframe compiles for iOS but does not list it as an officially supported
//! target, so test on a device early.
//!
//! Desktop: handy for fast iteration on layout and translations.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

fn main() -> eframe::Result {
    #[cfg(not(target_os = "android"))]
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    mobile_app::run(mobile_app::native_options())
}
