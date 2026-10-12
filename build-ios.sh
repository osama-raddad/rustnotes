#!/usr/bin/env bash
# Build + bundle Rust Notes for iOS — no Xcode project, no Swift, no ObjC.
#
#   ./build-ios.sh                      -> build/ios/RustNotes.app (simulator)
#   ./build-ios.sh <target> [install]   -> custom target; "install" installs
#                                          and launches on the booted simulator
#
# Targets:
#   aarch64-apple-ios-sim  iOS simulator (Apple Silicon) — default
#   aarch64-apple-ios      iOS device (requires your own codesign identity)
set -euo pipefail
cd "$(dirname "$0")"

TARGET="${1:-aarch64-apple-ios-sim}"
APP_ID="com.osamaraddad.rustnotes.ios"
APP="build/ios/Rust Notes.app"

echo "==> cargo build (rustnotes-ios, $TARGET)"
cargo build -p rustnotes-ios --target "$TARGET" --release

echo "==> bundle"
rm -rf build/ios
mkdir -p "$APP"
cp "target/$TARGET/release/rustnotes" "$APP/rustnotes"
cp ios/Info.plist "$APP/Info.plist"

if [[ "$TARGET" == *"-sim" ]]; then
    codesign --force --sign - --timestamp=none "$APP"
else
    echo "==> device build: sign with your identity before installing, e.g."
    echo "    codesign --force --sign 'Apple Development: you@x' '$APP'"
fi

echo "==> $APP"

if [[ "${2:-}" == "install" ]]; then
    xcrun simctl install booted "$APP"
    xcrun simctl launch booted "$APP_ID"
fi
