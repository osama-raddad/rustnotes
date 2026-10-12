#!/usr/bin/env bash
# Wrapper around xbuild (https://github.com/rust-mobile/xbuild) that puts the
# NDK toolchain on PATH (cc-rs resolves llvm-ar by name) and runs from the
# android crate, where manifest.yaml lives.
#
#   ./x.sh build --release --format apk   # -> target/x/release/android/rustnotes.apk
#   ./x.sh run --device adb:emulator-5554 --release   # deploy + launch + stream logs
set -euo pipefail
cd "$(dirname "$0")"
export PATH="$HOME/Library/Android/sdk/ndk/27.1.12297006/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"
cd crates/rustnotes-android
exec x "$@"
