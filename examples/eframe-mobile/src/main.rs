//! iOS and desktop entry point.
//!
//! iOS: winit's UIKit backend calls `UIApplicationMain` from this `main`, so
//! the binary is the whole app. Package it into an `.app` with an Info.plist
//! (CFBundleExecutable = mobile_app) and `codesign`; no Xcode project needed.
//! eframe compiles for iOS but does not list it as an officially supported
//! target, so test on a device early.
//!
//! Desktop: handy for fast iteration on layout and translations.

fn main() -> eframe::Result {
    // RUST_LOG wins when set; otherwise log at info.
    #[cfg(not(target_os = "android"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    #[cfg_attr(not(target_os = "ios"), allow(unused_mut))]
    let mut options = mobile_app::native_options();

    // eframe has no default storage directory on iOS (its `storage_dir()`
    // returns None there), so without an explicit path `cc.storage` is None
    // and nothing persists. The sandbox's Documents folder survives relaunches.
    #[cfg(target_os = "ios")]
    if let Some(home) = std::env::var_os("HOME") {
        options.persistence_path = Some(
            std::path::PathBuf::from(home)
                .join("Documents")
                .join("app.ron"),
        );
    }

    mobile_app::run(options)
}
