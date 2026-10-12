#!/usr/bin/env bash
# Build + package Rust Notes as an APK — no Gradle, no Java code.
#
#   ./build.sh          -> rustnotes.apk
#   ./build.sh install  -> also install on a connected device/emulator
set -euo pipefail
cd "$(dirname "$0")"

SDK="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
NDK_VERSION="27.1.12297006"
HOST_TAG="darwin-x86_64"
API="24"
TARGET="aarch64-linux-android"
BT="$SDK/build-tools/35.0.0"
PLATFORM_JAR="$SDK/platforms/android-35/android.jar"
TOOLCHAIN="$SDK/ndk/$NDK_VERSION/toolchains/llvm/prebuilt/$HOST_TAG"

# Toolchain env for C dependencies (rusqlite bundled SQLite)
export CC_aarch64_linux_android="$TOOLCHAIN/bin/aarch64-linux-android${API}-clang"
export CXX_aarch64_linux_android="$TOOLCHAIN/bin/aarch64-linux-android${API}-clang++"
export AR_aarch64_linux_android="$TOOLCHAIN/bin/llvm-ar"
export RANLIB_aarch64_linux_android="$TOOLCHAIN/bin/llvm-ranlib"

echo "==> rustup target"
rustup target add "$TARGET"

echo "==> cargo build ($TARGET, release)"
cargo build --target "$TARGET" --release

echo "==> package APK"
rm -rf build
mkdir -p build/apk/lib/arm64-v8a
cp "target/$TARGET/release/librustnotes.so" build/apk/lib/arm64-v8a/

"$BT/aapt2" link \
    -o build/rustnotes.unsigned.apk \
    --manifest AndroidManifest.xml \
    -I "$PLATFORM_JAR" \
    --min-sdk-version "$API" \
    --target-sdk-version 35

(cd build/apk && zip -qr ../rustnotes.unsigned.apk lib)
"$BT/zipalign" -f 4 build/rustnotes.unsigned.apk build/rustnotes.aligned.apk

if [ ! -f debug.keystore ]; then
    echo "==> generating debug keystore"
    keytool -genkeypair -keystore debug.keystore \
        -alias rustnotes -keyalg RSA -keysize 2048 -validity 10000 \
        -storepass android -keypass android \
        -dname "CN=Rust Notes Debug,O=Rust,C=US"
fi

"$BT/apksigner" sign \
    --ks debug.keystore --ks-key-alias rustnotes \
    --ks-pass pass:android --key-pass pass:android \
    --out rustnotes.apk build/rustnotes.aligned.apk

echo "==> built rustnotes.apk"

if [ "${1:-}" = "install" ]; then
    adb install -r rustnotes.apk
    adb shell am start -n com.osamaraddad.rustnotes/android.app.NativeActivity
fi
