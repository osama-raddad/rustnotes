# Rust Notes — the single build file. No .sh scripts; every pipeline is a
# recipe. Run `just` alone to list them.

homedir   := env_var('HOME')
sdk       := env_var_or_default('ANDROID_HOME', homedir + '/Library/Android/sdk')
toolchain := sdk + '/ndk/27.1.12297006/toolchains/llvm/prebuilt/darwin-x86_64/bin'
bt        := sdk + '/build-tools/35.0.0'
platform  := sdk + '/platforms/android-35/android.jar'
ks        := 'debug.keystore'
apk       := 'build/rustnotes.apk'
ios_app   := 'build/ios/Rust Notes.app'
ios_id    := 'com.osamaraddad.rustnotes.ios'

# NDK toolchain on PATH (xbuild's cc-rs resolves llvm-ar by name) and C env
# for bundled SQLite (rusqlite) in cargo builds
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
    cargo build -p rustnotes --lib --target aarch64-linux-android --release
    rm -rf build/apk
    mkdir -p build/apk/lib/arm64-v8a
    cp build/target/aarch64-linux-android/release/librustnotes.so build/apk/lib/arm64-v8a/

# Release APK: cargo -> aapt2 -> zip -> zipalign -> apksigner (targetSdk 35)
android: android-stage
    {{bt}}/aapt2 link -o build/rustnotes.unsigned.apk \
        --manifest resources/AndroidManifest.xml -I {{platform}} \
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

# APK + install & launch on an adb target (default: first connected device).
# Falls back to uninstall+install when the signature changed (e.g. after
# switching between the xbuild and keystore pipelines — clears app data).
android-install serial='':
    #!/bin/zsh
    set -euo pipefail
    just android
    s="{{serial}}"
    if [ -z "$s" ]; then
        s=$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print $1; exit}')
    fi
    [ -n "$s" ] || { echo "ERROR: no adb device connected."; exit 1; }
    echo "==> target: $s"
    adb -s "$s" install -r {{apk}} || (adb -s "$s" uninstall com.osamaraddad.rustnotes && adb -s "$s" install {{apk}})
    adb -s "$s" shell am start -n com.osamaraddad.rustnotes/android.app.NativeActivity

# Run on a connected Android device (physical first, then emulator); if none,
# boot the first AVD and run there.
run-android:
    #!/bin/zsh
    set -euo pipefail
    s=$(adb devices -l 2>/dev/null | awk '{if($2=="device"){if(/usb:/&&!p)p=$1;else if(!e)e=$1}}END{print p?p:e}')
    if [ -n "$s" ]; then
        just android-install "$s"
        exit 0
    fi
    emu="{{sdk}}/emulator/emulator"
    avd=$("$emu" -list-avds 2>/dev/null | head -1)
    [ -n "$avd" ] || { echo "ERROR: no Android device connected and no AVD found."; exit 1; }
    echo "==> no device — booting AVD '$avd'"
    nohup "$emu" -avd "$avd" >/dev/null 2>&1 & disown
    adb wait-for-device
    s=$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print $1; exit}')
    for i in {1..90}; do
        b=$(adb -s "$s" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
        if [ "$b" = "1" ]; then break; fi
        sleep 2
    done
    just android-install "$s"

# Release APK via xbuild (auto-signed, targetSdk 33)
android-x:
    cd crates/rustnotes-android && x build --platform android --arch arm64 --release --format apk

# xbuild: deploy + launch + stream logs (Ctrl-C detaches; app keeps running)
# (--device implies --platform; x rejects passing both)
android-run device='adb:emulator-5554':
    cd crates/rustnotes-android && x run --device {{device}} --arch arm64 --release

# ---- iOS --------------------------------------------------------------------

# Cross-compile and assemble the .app bundle
[private]
ios-build target:
    cargo build -p rustnotes --bin rustnotes --target {{target}} --release
    rm -rf build/ios
    mkdir -p "{{ios_app}}"
    cp build/target/{{target}}/release/rustnotes "{{ios_app}}/rustnotes"
    cp resources/Info.plist "{{ios_app}}/Info.plist"

# Simulator .app bundle (ad-hoc signed)
ios: (ios-build 'aarch64-apple-ios-sim')
    codesign --force --sign - --timestamp=none "{{ios_app}}"
    @echo "==> {{ios_app}}"

# .app + install & launch on the booted iOS simulator
ios-run: ios
    xcrun simctl install booted "{{ios_app}}"
    xcrun simctl launch booted {{ios_id}}

# Run on a connected iPhone; if none, run on the simulator (booting the first
# available iPhone simulator if needed).
run-ios:
    #!/bin/zsh
    set -euo pipefail
    dev=$(xcrun devicectl list devices 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="connected"||$i=="available"){for(j=1;j<=NF;j++) if($j ~ /^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$/){print $j; exit}}}')
    if [ -n "$dev" ]; then
        DEVICE="$dev" just ios-install
        exit 0
    fi
    echo "==> no iPhone connected — simulator"
    if ! xcrun simctl list devices booted 2>/dev/null | grep -q Booted; then
        udid=$(xcrun simctl list devices available 2>/dev/null | awk '/iPhone/{if(match($0,/[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}/)){print substr($0,RSTART,RLENGTH); exit}}')
        [ -n "$udid" ] || { echo "ERROR: no iPhone simulator available."; exit 1; }
        echo "==> booting simulator $udid"
        xcrun simctl boot "$udid"
        xcrun simctl bootstatus "$udid" -b
    fi
    open -a Simulator
    just ios-run

# Device build (unsigned — `just ios-install` signs + installs + launches)
ios-device: (ios-build 'aarch64-apple-ios')
    @echo "==> {{ios_app}} (unsigned — run 'just ios-install')"

# Sign the device .app with a provisioning profile covering {{ios_id}} and
# the identity whose certificate the profile embeds. Override with
# IDENTITY='<name-or-hash>' / PROFILE=path.
[private]
ios-sign:
    #!/bin/zsh
    set -euo pipefail
    app="{{ios_app}}"
    profdir="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
    [ -d "$profdir" ] || profdir="$HOME/Library/MobileDevice/Provisioning Profiles"
    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
    ids=$(security find-identity -v -p codesigning)
    grep -qE '^ *[0-9]+\)' <<< "$ids" || { echo "ERROR: no codesigning identity in the keychain."; echo "  Fix: Xcode > Settings > Accounts > (your Apple ID) > download a development certificate."; exit 1; }
    profile="${PROFILE:-}"
    if [ -n "$profile" ]; then
        security cms -D -i "$profile" > "$tmp/profile.plist"
    else
        now=$(date -u +%Y-%m-%dT%H:%M:%SZ)
        for p in "$profdir"/*.mobileprovision(om); do
            [ -e "$p" ] || continue
            security cms -D -i "$p" > "$tmp/profile.plist" 2>/dev/null || continue
            appid=$(plutil -extract Entitlements.application-identifier raw -o - "$tmp/profile.plist" 2>/dev/null) || continue
            rest=${appid#*.} # strip team id: TEAMID.bundle.id → bundle.id (TEAMID.* → *)
            [ "$rest" = "{{ios_id}}" ] || [ "$rest" = "*" ] || continue
            exp=$(plutil -extract ExpirationDate raw -o - "$tmp/profile.plist" 2>/dev/null) || continue
            [[ "$exp" > "$now" ]] || continue
            profile="$p"
            break
        done
    fi
    if [ -z "$profile" ]; then
        echo "ERROR: no provisioning profile covering {{ios_id}} in:"
        echo "  $profdir"
        echo "Fix: run 'just ios-profile' (rebuilds the bundled throwaway Xcode"
        echo "project, tools/ios-profile-helper, to mint a fresh 7-day personal-team"
        echo "profile — iPhone must be connected). Then rerun 'just ios-install'."
        echo "(Paid-team alternative: register the App ID + create a development"
        echo " profile at developer.apple.com, download it into the dir above.)"
        exit 1
    fi
    # Pick the identity whose certificate is embedded in the profile, matched
    # by SHA-1 hash — the keychain can hold several identically-named
    # identities, which codesign rejects as ambiguous when named.
    if [ -n "${IDENTITY:-}" ]; then
        signer="$IDENTITY"
    else
        signer=""
        certhashes=$(python3 -c 'import hashlib,plistlib,sys; prof=plistlib.load(open(sys.argv[1],"rb")); print(" ".join(hashlib.sha1(c).hexdigest().upper() for c in prof.get("DeveloperCertificates",[])))' "$tmp/profile.plist")
        for c in ${=certhashes}; do
            m=$(print -r -- "$ids" | grep -i "$c" | head -1) || m=""
            if [ -n "$m" ]; then signer=$(print -r -- "$m" | awk '{print $2}'); break; fi
        done
        # Fall back to the first identity when none matches the profile.
        [ -n "$signer" ] || signer=$(print -r -- "$ids" | sed -n 's/^ *[0-9]*) \([0-9A-F]\{40\}\) .*/\1/p' | head -1)
    fi
    echo "==> signing: $signer"
    echo "==> profile: $profile"
    plutil -extract Entitlements xml1 -o "$tmp/entitlements.plist" "$tmp/profile.plist"
    cp "$profile" "$app/embedded.mobileprovision"
    codesign --force --sign "$signer" --entitlements "$tmp/entitlements.plist" --timestamp=none "$app"
    codesign --verify "$app"

# Refresh the 7-day development profile for {{ios_id}}: rebuild the throwaway
# Xcode project (tools/ios-profile-helper, team pre-set) with automatic signing.
# Run when ios-install reports no valid profile. iPhone connected once.
ios-profile:
    #!/bin/zsh
    set -euo pipefail
    dev="${DEVICE:-$(xcrun devicectl list devices 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="connected"||$i=="available"){for(j=1;j<=NF;j++) if($j ~ /^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$/){print $j; exit}}}')}"
    [ -n "$dev" ] || { echo "ERROR: no connected device — profile refresh needs the iPhone once."; exit 1; }
    xcodebuild -project tools/ios-profile-helper/ios.xcodeproj -scheme ios \
        -destination "id=$dev" -allowProvisioningUpdates \
        -allowProvisioningDeviceRegistration build

# .app + sign + install + launch on a connected iPhone (DEVICE=<uuid> to pick)
ios-install: ios-device ios-sign
    #!/bin/zsh
    set -euo pipefail
    dev="${DEVICE:-$(xcrun devicectl list devices 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="connected"||$i=="available"){for(j=1;j<=NF;j++) if($j ~ /^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$/){print $j; exit}}}')}"
    [ -n "$dev" ] || { echo "ERROR: no connected device — plug in and trust the iPhone."; exit 1; }
    echo "==> device: $dev"
    xcrun devicectl device install app --device "$dev" "{{ios_app}}"
    xcrun devicectl device process launch --device "$dev" "{{ios_id}}"

# ---- misc -------------------------------------------------------------------

# Type-check all three crates for their real targets
# (the workspace can't be checked as a whole: ndk-sys only compiles for Android)
check:
    cargo check -p rustnotes-core
    cargo check -p rustnotes --lib --target aarch64-linux-android
    cargo check -p rustnotes --bin rustnotes --target aarch64-apple-ios-sim
