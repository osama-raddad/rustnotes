# Rust Notes — the single build file. No .sh scripts; every pipeline is a
# recipe. Run `just` alone to list them.

homedir   := env_var('HOME')
sdk       := env_var_or_default('ANDROID_HOME', homedir + '/Library/Android/sdk')
toolchain := sdk + '/ndk/27.1.12297006/toolchains/llvm/prebuilt/darwin-x86_64/bin'
bt        := sdk + '/build-tools/35.0.0'
platform  := sdk + '/platforms/android-35/android.jar'
ks        := 'debug.keystore'
apk       := 'rustnotes.apk'
ios_app   := 'build/ios/Rust Notes.app'
ios_id    := 'com.osamaraddad.rustnotes.ios'

# NDK toolchain on PATH (xbuild's cc-rs resolves llvm-ar by name) and C env
# for bundled SQLite when cargo runs outside the old build.sh
export PATH := toolchain + ':' + env_var('PATH')
export CC_aarch64_linux_android := toolchain + '/aarch64-linux-android24-clang'
export AR_aarch64_linux_android := toolchain + '/llvm-ar'

default:
    @just --list

# ---- Android ---------------------------------------------------------------

# Cross-compile the cdylib and stage it for packaging
[private]
android-stage:
    @rustup target add aarch64-linux-android
    cargo build -p rustnotes --target aarch64-linux-android --release
    rm -rf build/apk
    mkdir -p build/apk/lib/arm64-v8a
    cp target/aarch64-linux-android/release/librustnotes.so build/apk/lib/arm64-v8a/

# Release APK: cargo -> aapt2 -> zip -> zipalign -> apksigner (targetSdk 35)
android: android-stage
    {{bt}}/aapt2 link -o build/rustnotes.unsigned.apk \
        --manifest AndroidManifest.xml -I {{platform}} \
        --min-sdk-version 24 --target-sdk-version 35
    cd build/apk && zip -qr ../rustnotes.unsigned.apk lib
    {{bt}}/zipalign -f 4 build/rustnotes.unsigned.apk build/rustnotes.aligned.apk
    @test -f {{ks}} || keytool -genkeypair -keystore {{ks}} -alias rustnotes \
        -keyalg RSA -keysize 2048 -validity 10000 -storepass android \
        -keypass android -dname "CN=Rust Notes Debug,O=Rust,C=US"
    {{bt}}/apksigner sign --ks {{ks}} --ks-key-alias rustnotes \
        --ks-pass pass:android --key-pass pass:android \
        --out {{apk}} build/rustnotes.aligned.apk
    @echo "==> {{apk}}"

# APK + install & launch on a connected device.
# Falls back to uninstall+install when the signature changed (e.g. after
# switching between the xbuild and keystore pipelines — clears app data).
android-install: android
    adb install -r {{apk}} || (adb uninstall com.osamaraddad.rustnotes && adb install {{apk}})
    adb shell am start -n com.osamaraddad.rustnotes/android.app.NativeActivity

# Release APK via xbuild (auto-signed, targetSdk 33)
android-x:
    cd crates/rustnotes-android && x build --platform android --arch arm64 --release --format apk

# xbuild: deploy + launch + stream logs (Ctrl-C detaches; app keeps running)
android-run device='adb:emulator-5554':
    cd crates/rustnotes-android && x run --device {{device}} --platform android --arch arm64 --release

# ---- iOS --------------------------------------------------------------------

# Cross-compile and assemble the .app bundle
[private]
ios-build target:
    cargo build -p rustnotes-ios --target {{target}} --release
    rm -rf build/ios
    mkdir -p "{{ios_app}}"
    cp target/{{target}}/release/rustnotes "{{ios_app}}/rustnotes"
    cp ios/Info.plist "{{ios_app}}/Info.plist"

# Simulator .app bundle (ad-hoc signed)
ios: (ios-build 'aarch64-apple-ios-sim')
    codesign --force --sign - --timestamp=none "{{ios_app}}"
    @echo "==> {{ios_app}}"

# .app + install & launch on the booted iOS simulator
ios-run: ios
    xcrun simctl install booted "{{ios_app}}"
    xcrun simctl launch booted {{ios_id}}

# Device build (sign with your identity before installing)
ios-device: (ios-build 'aarch64-apple-ios')
    @echo "Device build — sign with your identity before installing:"
    @echo "  codesign --force --sign 'Apple Development: you@x' '{{ios_app}}'"

# ---- misc -------------------------------------------------------------------

# Type-check all three crates for their real targets
# (the workspace can't be checked as a whole: ndk-sys only compiles for Android)
check:
    cargo check -p rustnotes-core
    cargo check -p rustnotes --target aarch64-linux-android
    cargo check -p rustnotes-ios --target aarch64-apple-ios-sim
