# Rust Notes — one entry point for every build task.
# Run `just` alone to list recipes. See README.md for details.

ndk := env_var('HOME') + '/Library/Android/sdk/ndk/27.1.12297006/toolchains/llvm/prebuilt/darwin-x86_64/bin'

# Toolchain env for C code (bundled SQLite) when cargo runs outside build.sh
export CC_aarch64_linux_android  := ndk + '/aarch64-linux-android24-clang'
export AR_aarch64_linux_android  := ndk + '/llvm-ar'

default:
    @just --list

# Build the release APK via the script pipeline (aapt2/apksigner, targetSdk 35)
android:
    ./build.sh

# Script pipeline: build + install + launch on a connected Android device
android-install:
    ./build.sh install

# Build the release APK via xbuild (auto-signed, targetSdk 33)
android-x:
    ./x.sh build --platform android --arch arm64 --release --format apk

# xbuild: deploy + launch + stream logs (Ctrl-C detaches; app keeps running)
android-run device="adb:emulator-5554":
    ./x.sh run --device {{device}} --platform android --arch arm64 --release

# Build the iOS app bundle for the simulator (ad-hoc signed)
ios:
    ./build-ios.sh

# iOS simulator: build + install + launch on the booted simulator
ios-run:
    ./build-ios.sh aarch64-apple-ios-sim install

# Build for a physical iOS device (sign with your identity before installing)
ios-device:
    ./build-ios.sh aarch64-apple-ios

# Type-check all three crates for their real targets
# (the workspace can't be checked as a whole: ndk-sys only compiles for Android)
check:
    cargo check -p rustnotes-core
    cargo check -p rustnotes --target aarch64-linux-android
    cargo check -p rustnotes-ios --target aarch64-apple-ios-sim
