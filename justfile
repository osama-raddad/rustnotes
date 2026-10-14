# Rust Notes — the single build file. No .sh scripts; every pipeline is a
# recipe. Run `just` alone to list them.
#
# The packaging recipes are generalized over apps: every app-facing recipe
# takes an `app` parameter (default 'rustnotes') whose value is at once the
# cargo package name, the crates/ dir name, and the app-identity key — one
# string doing triple duty. `just android` is exactly `just android
# app=rustnotes`: same paths, same package id, same pipeline as before the
# second app existed. Everything the key alone can't tell a recipe (cdylib
# lib name, Android package, label, keystore, resources dir, …) comes from
# the app-identity table below — the embryo of the endpoint CLI's per-app
# config (plan Q5 gates extracting it any further).

homedir   := env_var('HOME')
sdk       := env_var_or_default('ANDROID_HOME', homedir + '/Library/Android/sdk')
# Host tag of the NDK prebuilt dir, derived at recipe time (just's os()
# beats shelling out to `uname -s`; the NDK ships no darwin-aarch64
# prebuilt — both Intel and Apple-Silicon Macs use darwin-x86_64, under
# Rosetta — so this stays two-way, macOS/Linux).
host      := if os() == 'macos' { 'darwin-x86_64' } else { 'linux-x86_64' }
toolchain := sdk + '/ndk/27.1.12297006/toolchains/llvm/prebuilt/' + host + '/bin'
bt        := sdk + '/build-tools/35.0.0'
platform  := sdk + '/platforms/android-35/android.jar'

# NDK toolchain on PATH (xbuild's cc-rs resolves llvm-ar by name), C env for
# bundled SQLite (rusqlite) in cargo builds, and the Android linker — exported
# so .cargo/config.toml stays free of absolute machine paths
export PATH := toolchain + ':' + env_var('PATH')
export CC_aarch64_linux_android := toolchain + '/aarch64-linux-android24-clang'
export AR_aarch64_linux_android := toolchain + '/llvm-ar'
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER := toolchain + '/aarch64-linux-android24-clang'

default:
    @just --list

# ---- app identity -----------------------------------------------------------

# The identity table: everything the packaging recipes know about an app
# beyond its `app` key. Emits shell assignments, one per line, for eval:
#   key          the app key, normalized (see below) — recipes use $key for
#                artifact paths (build/$key.apk) so both spellings of the
#                argument produce the same artifacts
#   crate        cargo package name (-p); equals the app key by construction
#   cdylib       lib name without the lib/ prefix and .so suffix — NOT always
#                the crate name: hello-mobile's Cargo [lib] name is
#                hello_mobile, so its jniLib is libhello_mobile.so
#   bin          cargo bin name (the iOS bundle executable / desktop run)
#   android_pkg  Android package id — a matched set with the `package` attr
#                in the app's AndroidManifest.xml AND the App impl's
#                ANDROID_DATA_DIR_FALLBACK, which encodes the package name in
#                its path. A mismatched pair is a silent bug (the app
#                installs and launches, then "falls back" into a directory
#                Android doesn't own); the Rust trait cannot enforce it, so
#                this table is the one place all three meet.
#   label        display label; doubles as the iOS .app dir name (space
#                included: "Rust Notes.app")
#   ios_bundle   iOS bundle id (repo convention: android pkg + ".ios")
#   res          the app's resources dir (AndroidManifest.xml + Info.plist);
#                rustnotes' lives at the repo root for backward compat,
#                later apps under crates/<app>/resources — asymmetry kept,
#                resolved when the endpoint CLI owns per-app config
#   ks / ks_alias / ks_dname
#                the app's OWN debug keystore file, alias, and dname — see
#                the keystore recipe for why files, not one file with two
#                aliases
#   entry        cargo feature compiling this app's own android_main in the
#                rustnotes host ('android-entry' for rustnotes; '' for
#                hello-mobile, whose android_main lives in its own lib
#                unconditionally). Needed when a recipe builds with
#                --no-default-features (the GameActivity flavor does, to
#                swap the backend without dragging the default back in):
#                everything default provides besides the backend must be
#                re-added by name, and this is that name.
# The activity backend (NativeActivity vs GameActivity) is deliberately NOT
# a table row: it is a build flavor of the host, not a fact about the app —
# the same app packages under either. Recipes take it as an `activity`
# parameter ('native', default | 'game'), orthogonal to this table.
# The key may be spelled two ways on the command line — `just android
# hello-mobile` and `just android app=hello-mobile` — because just binds
# the `app=…` token whole as a positional argument; the prefix is stripped
# here, once, and `$key` (never `{{app}}`) names artifacts.
# Recipes consume this in the two-step form:
#   cfg=$(just app-identity "{{app}}"); eval "$cfg"
# never `eval "$(just app-identity …)"` — under set -euo pipefail the
# assignment aborts on an unknown app, whereas the eval-string form would
# swallow the failure and eval an empty string into defaults-from-nothing.
[private]
app-identity app:
    #!/bin/zsh
    key="{{app}}"
    key=${key#app=}
    case "$key" in
    rustnotes)
        echo "key='rustnotes'"
        echo "crate='rustnotes'"
        echo "cdylib='rustnotes'"
        echo "bin='rustnotes'"
        echo "android_pkg='com.osamaraddad.rustnotes'"
        echo "label='Rust Notes'"
        echo "ios_bundle='com.osamaraddad.rustnotes.ios'"
        echo "res='resources'"
        echo "ks='debug.keystore'"
        echo "ks_alias='rustnotes'"
        echo "ks_dname='CN=Rust Notes Debug,O=Rust,C=US'"
        echo "entry='android-entry'"
        ;;
    hello-mobile)
        echo "key='hello-mobile'"
        echo "crate='hello-mobile'"
        echo "cdylib='hello_mobile'"
        echo "bin='hello-mobile'"
        echo "android_pkg='com.example.hellomobile'"
        echo "label='Hello Mobile'"
        echo "ios_bundle='com.example.hellomobile.ios'"
        echo "res='crates/hello-mobile/resources'"
        echo "ks='hello-mobile.debug.keystore'"
        echo "ks_alias='hellomobile'"
        echo "ks_dname='CN=Hello Mobile Debug,O=Rust,C=US'"
        echo "entry=''"
        ;;
    ui-kit-gallery)
        echo "key='ui-kit-gallery'"
        echo "crate='ui-kit-gallery'"
        echo "cdylib='ui_kit_gallery'"
        echo "bin='ui-kit-gallery'"
        echo "android_pkg='com.osamaraddad.vynatixgallery'"
        echo "label='Vynatix Gallery'"
        echo "ios_bundle='com.osamaraddad.vynatixgallery.ios'"
        echo "res='crates/ui-kit-gallery/resources'"
        echo "ks='ui-kit-gallery.debug.keystore'"
        echo "ks_alias='vynatixgallery'"
        echo "ks_dname='CN=Vynatix Gallery Debug,O=Rust,C=US'"
        echo "entry=''"
        ;;
    *)
        echo "ERROR: unknown app '{{app}}' (expected 'rustnotes', 'hello-mobile' or 'ui-kit-gallery')." >&2
        exit 1
        ;;
    esac

# ---- Android ----------------------------------------------------------------

# Fetch (with caching) and sha256-verify the games-activity AAR — the Java
# half of the GameActivity backend, dexed into GA APKs by android-stage.
# Lockfile-style pin, the same job a Cargo.lock does for crates:
#   artifact  androidx.games:games-activity
#   version   4.4.0 — the release android-activity 0.6.1's native glue is
#             built and documented against (backwards compatible with
#             4.0.0; see its README's GameActivity section)
#   sha256    3a8b0887ac55a0a6ab56021ced7353a250539eac811278b74bccd8a5b678d9c9
#   url       https://dl.google.com/android/maven2/androidx/games/games-activity/4.4.0/games-activity-4.4.0.aar
# A mismatch fails loudly and never continues: a tampered cache must not
# ride into a signed APK. Note the AAR's pom declares NO dependencies —
# false comfort: its bytecode extends androidx.appcompat.app.AppCompatActivity
# (see the plan addendum; the runtime consequence is recorded there).
# Bump = update BOTH sites: the sha256 appears here (comment) and in the
# shasum check below; a version bump that touches one leaves the other
# pinning the old digest.
[private]
android-aar:
    #!/bin/zsh
    set -euo pipefail
    mkdir -p build/cache
    aar=build/cache/games-activity-4.4.0.aar
    if [ ! -f "$aar" ]; then
        tmp="$aar.tmp.$$"
        trap 'rm -f "$tmp"' EXIT
        curl -fsSL -o "$tmp" "https://dl.google.com/android/maven2/androidx/games/games-activity/4.4.0/games-activity-4.4.0.aar"
        mv "$tmp" "$aar"
    fi
    if ! echo "3a8b0887ac55a0a6ab56021ced7353a250539eac811278b74bccd8a5b678d9c9  $aar" | shasum -a 256 -c - >/dev/null 2>&1; then
        echo "ERROR: sha256 mismatch for $aar — supply-chain guard tripped." >&2
        echo "Delete the file and rerun; if it mismatches again the pin or the source is wrong. Never continue past a mismatch." >&2
        exit 1
    fi

# Cross-compile the app's cdylib and stage it for packaging
# (rust-toolchain.toml already installs the aarch64-linux-android target —
# no rustup call needed — for every app). build/apk stays a single shared
# staging dir, wiped per run exactly as when there was one app: only one
# app is packaged at a time, and a stale lib from the other app must never
# ride along into the zip.
# activity=native (default) builds with default features — NativeActivity,
# the byte-identical pipeline of old. activity=game SWAPS the backend (see
# the crates' Cargo.toml for the swap-not-add rule) via --no-default-features,
# re-adding the app's own entry feature from the identity table, and dexes
# the games-activity AAR's classes.jar next to the staged lib — the APK's
# only Java, and the reason its manifest variant says hasCode="true".
[private]
android-stage app='rustnotes' activity='native':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    # `activity=` prefix stripped here for the same reason app-identity
    # strips `app=`: just binds a `x=y` token whole into the positional
    # slot it lands in, so both spellings must normalize.
    act="{{activity}}"; act=${act#activity=}
    case "$act" in
    native)
        cargo build -p "$crate" --lib --target aarch64-linux-android --release
        ;;
    game)
        just android-aar
        # Build-time disclosure, in-recipe: this APK installs and then
        # dies at launch (missing androidx closure — README's opt-in
        # section has the wall). Warned, not refused: check-links' GA
        # legs and the probe surface depend on the build succeeding.
        echo "NOTE: activity=game APKs crash at launch until the androidx java closure is packaged — see README 'Android: the GameActivity opt-in'. NativeActivity is the launchable backend." >&2
        feats="android-game-activity${entry:+,$entry}"
        cargo build -p "$crate" --lib --target aarch64-linux-android --release \
            --no-default-features --features "$feats"
        ;;
    *)
        echo "ERROR: unknown activity backend '$act' (expected 'native' or 'game')." >&2
        exit 1
        ;;
    esac
    rm -rf build/apk
    mkdir -p build/apk/lib/arm64-v8a
    cp "build/target/aarch64-linux-android/release/lib$cdylib.so" build/apk/lib/arm64-v8a/
    if [ "$act" = game ]; then
        tmp=$(mktemp -d)
        trap 'rm -rf "$tmp"' EXIT
        unzip -q build/cache/games-activity-4.4.0.aar classes.jar -d "$tmp"
        "{{bt}}/d8" --release --lib "{{platform}}" --min-api 24 \
            --output build/apk "$tmp/classes.jar"
    fi

# Preflight keytool (a JDK — macOS ships none) and mint the app's debug
# keystore atomically (temp file + mv), so a missing JDK or a killed keytool
# fails here — before the long cross-build — instead of deep inside
# apksigner. One keystore FILE per app, not one file with an alias per app:
# keytool rewrites the whole keystore file, non-atomically, for every
# genkeypair — a shared file would put a live keystore (the other app's
# signing key!) under that rewrite and break the temp-file+mv atomicity this
# recipe exists to provide. Per-app files also re-mint independently.
[private]
keystore app='rustnotes':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    command -v keytool >/dev/null || { echo "ERROR: keytool (a JDK) is required for debug signing — e.g. brew install --cask temurin."; exit 1; }
    if [ ! -f "$ks" ]; then
        tmp="$ks.tmp.$$"
        trap 'rm -f "$tmp"' EXIT
        keytool -genkeypair -keystore "$tmp" -alias "$ks_alias" \
            -keyalg RSA -keysize 2048 -validity 10000 -storepass android \
            -keypass android -dname "$ks_dname"
        mv "$tmp" "$ks"
    fi

# Release APK for any app: cargo -> aapt2 -> zip -> zipalign -> apksigner
# (targetSdk 35 — enforced by the aapt2 flags here, mirrored by every app's
# manifest; min-sdk 24 likewise).
#
# activity=native (default) is the old pipeline byte-for-byte: the manifest
# under $res, lib alone in the zip. activity=game swaps in the .game.xml
# manifest variant and adds the staged classes.dex to the zip — that is the
# whole difference; zipalign and signing do not care what is inside.
#
# 16 KB coupling, per app: extractNativeLibs="true" (the app's
# AndroidManifest under $res) + the DEFLATE-compressed .so stored by
# `zip` + `zipalign -f 4` form one matched set — the loader never mmaps the
# lib from inside the APK, and what 16 KB kernels actually check is the
# ELF's p_align (16384, from the rustflags in .cargo/config.toml, which
# being global covers every app's cdylib). Both activity flavors sit
# inside that set unchanged: classes.dex is not an ELF and changes nothing
# about alignment or loading, and the GA manifest variant keeps
# extractNativeLibs="true". Flipping ANY app's manifest to
# extractNativeLibs="false" (uncompressed .so) requires `zip -0 -X` +
# `zipalign -f -p 4` below for that app, or 16 KB-kernel devices stop
# launching it. Each manifest variant is otherwise a matched set (full
# configChanges list + hasCode="false" + NativeActivity, or GameActivity's
# shorter backend-documented list + hasCode="true" + dex) — copied verbatim
# per app. The ANR rationale is the NA variant's: there, a missed
# configChanges entry recreates the activity and NativeActivity's destroy
# waits on android_main. The GA variant's short list is what the backend's
# own example manifests document — recreation is handled by that glue.
android app='rustnotes' activity='native': (keystore app) (android-stage app activity)
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    act="{{activity}}"; act=${act#activity=}   # same prefix normalization as android-stage
    manifest="AndroidManifest.xml"
    if [ "$act" = game ]; then
        manifest="AndroidManifest.game.xml"
    fi
    "{{bt}}/aapt2" link -o "build/$key.unsigned.apk" \
        --manifest "$res/$manifest" -I "{{platform}}" \
        --min-sdk-version 24 --target-sdk-version 35
    if [ "$act" = game ]; then
        (cd build/apk && zip -qrX "../$key.unsigned.apk" lib classes.dex)
    else
        (cd build/apk && zip -qrX "../$key.unsigned.apk" lib)
    fi
    "{{bt}}/zipalign" -f 4 "build/$key.unsigned.apk" "build/$key.aligned.apk"
    "{{bt}}/apksigner" sign --ks "$ks" --ks-key-alias "$ks_alias" \
        --ks-pass pass:android --key-pass pass:android \
        --out "build/$key.apk" "build/$key.aligned.apk"
    echo "==> build/$key.apk"

# APK + install & launch on an adb target (default: first connected device).
# Falls back to uninstall+install when the signature changed (e.g. after
# switching between the xbuild and keystore pipelines — clears app data).
# The launch component follows the activity flavor: the class the manifest
# declares is the class `am` must name.
android-install app='rustnotes' serial='' activity='native':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    act="{{activity}}"; act=${act#activity=}   # same prefix normalization as android-stage
    case "$act" in native|game) ;; *) echo "ERROR: unknown activity backend '$act' (expected 'native' or 'game')." >&2; exit 1 ;; esac
    just android "$key" "$act"
    s="{{serial}}"
    if [ -z "$s" ]; then
        s=$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print $1; exit}')
    fi
    [ -n "$s" ] || { echo "ERROR: no adb device connected."; exit 1; }
    echo "==> target: $s"
    adb -s "$s" install -r "build/$key.apk" || (adb -s "$s" uninstall "$android_pkg" && adb -s "$s" install "build/$key.apk")
    comp=android.app.NativeActivity
    if [ "$act" = game ]; then
        comp=com.google.androidgamesdk.GameActivity
    fi
    adb -s "$s" shell am start -n "$android_pkg/$comp"

# Run on a connected Android device (physical first, then emulator); if none,
# boot the first AVD and run there.
run-android app='rustnotes' activity='native':
    #!/bin/zsh
    set -euo pipefail
    act="{{activity}}"; act=${act#activity=}   # same prefix normalization as android-stage
    case "$act" in native|game) ;; *) echo "ERROR: unknown activity backend '$act' (expected 'native' or 'game')." >&2; exit 1 ;; esac
    s=$(adb devices -l 2>/dev/null | awk '{if($2=="device"){if(/usb:/&&!p)p=$1;else if(!e)e=$1}}END{print p?p:e}')
    if [ -n "$s" ]; then
        just android-install "{{app}}" "$s" "$act"
        exit 0
    fi
    emu="{{sdk}}/emulator/emulator"
    avd=$("$emu" -list-avds 2>/dev/null | head -1)
    [ -n "$avd" ] || { echo "ERROR: no Android device connected and no AVD found."; exit 1; }
    echo "==> no device — booting AVD '$avd'"
    nohup "$emu" -avd "$avd" >/dev/null 2>&1 & disown
    adb wait-for-device
    s=$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print $1; exit}')
    [ -n "$s" ] || { echo "ERROR: emulator started but adb lost it."; exit 1; }
    ok=0
    for i in {1..90}; do
        b=$(adb -s "$s" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
        if [ "$b" = "1" ]; then ok=1; break; fi
        sleep 2
    done
    [ "$ok" = 1 ] || { echo "ERROR: emulator did not boot within 180s."; exit 1; }
    just android-install "{{app}}" "$s" "$act"

# Release APK via xbuild (auto-signed, targetSdk 35). xbuild runs per crate,
# so cd into the app's crate dir — the dir name equals the app key for every
# app in the identity table. NativeActivity only: xbuild has no GameActivity
# path (no --no-default-features passthrough, no games-activity support), so
# the opt-in does not reach these recipes — see README 'Android: the
# GameActivity opt-in'.
android-x app='rustnotes':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    cd "crates/$key" && x build --platform android --arch arm64 --release --format apk

# xbuild: deploy + launch + stream logs (Ctrl-C detaches; app keeps running)
# (--device implies --platform; x rejects passing both). `app` is first, as
# in every app-facing recipe — with device first, `just android-run
# app=hello-mobile` would bind the whole token to the DEVICE (just binds
# positional tokens whole) and silently keep packaging rustnotes.
# NativeActivity only, same xbuild limitation as android-x above.
android-run app='rustnotes' device='adb:emulator-5554':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    cd "crates/$key" && x run --device {{device}} --arch arm64 --release

# ---- iOS --------------------------------------------------------------------

# Echo the UUID of the first *connected* iPhone. devicectl's "available"
# state also lists paired-but-offline devices, which would fail later inside
# devicectl install / xcodebuild with unrelated errors.
[private]
ios-device-id:
    @xcrun devicectl list devices 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="connected"){for(j=1;j<=NF;j++) if($j ~ /^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$/){print $j; exit}}}'

# Fail unless exactly one iOS simulator is booted. Shared by ios-run and
# screenshot-ios: simctl's `booted` selector silently picks an arbitrary one
# of many, and a wrong-device install or screenshot is worse than a clear
# error.
[private]
ios-one-booted-sim:
    #!/bin/zsh
    set -euo pipefail
    n=$(xcrun simctl list devices booted 2>/dev/null | grep -c Booted || true)
    [ "$n" -eq 1 ] || { echo "ERROR: expected exactly one booted iOS simulator, found $n — boot exactly one and rerun."; exit 1; }

# Cross-compile and assemble the app's .app bundle. IPHONEOS_DEPLOYMENT_TARGET
# mirrors MinimumOSVersion (13.0) in the app's Info.plist — without it the
# effective iOS floor floats on cc-rs/objc2 defaults at build time. build/ios
# stays one shared dir, wiped per build as before: one app bundles at a time,
# and the .app of the other app must not survive the wipe.
[private]
ios-build app target:
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    IPHONEOS_DEPLOYMENT_TARGET=13.0 cargo build -p "$crate" --bin "$bin" --target {{target}} --release
    rm -rf build/ios
    mkdir -p "build/ios/$label.app"
    cp "build/target/{{target}}/release/$bin" "build/ios/$label.app/$bin"
    cp "$res/Info.plist" "build/ios/$label.app/Info.plist"

# Simulator .app bundle (ad-hoc signed)
ios app='rustnotes': (ios-build app 'aarch64-apple-ios-sim')
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    codesign --force --sign - --timestamp=none "build/ios/$label.app"
    echo "==> build/ios/$label.app"

# .app + install & launch on the booted iOS simulator (exactly one — see
# ios-one-booted-sim).
ios-run app='rustnotes': (ios app)
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    just ios-one-booted-sim
    xcrun simctl install booted "build/ios/$label.app"
    xcrun simctl launch booted "$ios_bundle"

# Run on a connected iPhone; if none, run on the simulator (booting the first
# available iPhone simulator if needed).
run-ios app='rustnotes':
    #!/bin/zsh
    set -euo pipefail
    dev="${DEVICE:-$(just ios-device-id)}"
    if [ -n "$dev" ]; then
        DEVICE="$dev" just ios-install "{{app}}"
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
    just ios-run "{{app}}"

# Device build (unsigned — `just ios-install` signs + installs + launches)
ios-device app='rustnotes': (ios-build app 'aarch64-apple-ios')
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    echo "==> build/ios/$label.app (unsigned — run 'just ios-install app=$key')"

# Sign the device .app with a provisioning profile covering the app's iOS
# bundle id and the identity whose certificate the profile embeds. Override
# with IDENTITY='<name-or-hash>' / PROFILE=path.
#
# Per-app by scan, not by guard: the loop below matches each profile's
# application-identifier against THIS app's bundle id (or a wildcard), so a
# second app can never silently sign with rustnotes' profile — a missing
# profile for its own bundle id is the error. Minting is `ios-profile`,
# which overrides the helper's baked bundle id per app.
[private]
ios-sign app='rustnotes':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    app="build/ios/$label.app"
    profdir="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
    [ -d "$profdir" ] || profdir="$HOME/Library/MobileDevice/Provisioning Profiles"
    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
    ids=$(security find-identity -v -p codesigning)
    grep -qE '^ *[0-9]+\)' <<< "$ids" || { echo "ERROR: no codesigning identity in the keychain."; echo "  Fix: Xcode > Settings > Accounts > (your Apple ID) > download a development certificate."; exit 1; }
    profile="${PROFILE:-}"
    if [ -n "$profile" ]; then
        security cms -D -i "$profile" > "$tmp/profile.plist"
    else
        now=$(date -u +%Y%m%d%H%M%S)
        for p in "$profdir"/*.mobileprovision(om); do
            [ -e "$p" ] || continue
            security cms -D -i "$p" > "$tmp/profile.plist" 2>/dev/null || continue
            appid=$(plutil -extract Entitlements.application-identifier raw -o - "$tmp/profile.plist" 2>/dev/null) || continue
            rest=${appid#*.} # strip team id: TEAMID.bundle.id → bundle.id (TEAMID.* → *)
            [ "$rest" = "$ios_bundle" ] || [ "$rest" = "*" ] || continue
            exp=$(plutil -extract ExpirationDate raw -o - "$tmp/profile.plist" 2>/dev/null) || continue
            # plutil prints ExpirationDate as ISO-8601 ("YYYY-MM-DDTHH:MM:SSZ")
            # on newer macOS and "YYYY-MM-DD HH:MM:SS +0000" on older —
            # normalize to the latter first, then both sides to comparable
            # UTC integers (a plain string compare pits ' ' against 'T' at
            # position 10).
            exp=${exp/T/ }
            exp=${exp/Z/ +0000}
            exp=$(date -ju -f '%Y-%m-%d %H:%M:%S %z' "$exp" +%Y%m%d%H%M%S) || continue
            [[ "$exp" > "$now" ]] || continue
            profile="$p"
            break
        done
    fi
    if [ -z "$profile" ]; then
        echo "ERROR: no provisioning profile covering $ios_bundle in:"
        echo "  $profdir"
        echo "Fix: run 'just ios-profile app=$key' (rebuilds the bundled throwaway Xcode"
        echo "project, tools/ios-profile-helper, overriding its bundle id to mint a fresh"
        echo "7-day personal-team profile — iPhone must be connected). Then rerun"
        echo "'just ios-install app=$key'."
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

# Refresh the 7-day development profile: rebuild the throwaway Xcode project
# (tools/ios-profile-helper) with automatic signing. Run when ios-install
# reports no valid profile. iPhone connected once.
# The helper project's pbxproj bakes a personal DEVELOPMENT_TEAM and
# com.osamaraddad.rustnotes.ios as its bundle id; the command-line overrides
# below beat pbxproj settings, so contributors pass TEAM=<their-team-id>
# (or export TEAM) without editing anything checked in, and each app mints
# under its own bundle id from the identity table (free personal teams:
# up to 10 App IDs per 7 days — the per-app override spends one per app).
ios-profile app='rustnotes' TEAM=env_var_or_default('TEAM', 'P86GBGS2B7'):
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    dev="${DEVICE:-$(just ios-device-id)}"
    [ -n "$dev" ] || { echo "ERROR: no connected device — profile refresh needs the iPhone once."; exit 1; }
    xcodebuild -project tools/ios-profile-helper/ios.xcodeproj -scheme ios \
        -destination "id=$dev" -allowProvisioningUpdates \
        -allowProvisioningDeviceRegistration DEVELOPMENT_TEAM="{{TEAM}}" \
        PRODUCT_BUNDLE_IDENTIFIER="$ios_bundle" build

# .app + sign + install + launch on a connected iPhone (DEVICE=<uuid> to
# pick). Signing is per-app: ios-sign matches a profile to the app's own
# bundle id (mint one with `just ios-profile app=<key>` when none covers it).
ios-install app='rustnotes': (ios-device app) (ios-sign app)
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    dev="${DEVICE:-$(just ios-device-id)}"
    [ -n "$dev" ] || { echo "ERROR: no connected device — plug in and trust the iPhone."; exit 1; }
    echo "==> device: $dev"
    xcrun devicectl device install app --device "$dev" "build/ios/$label.app"
    xcrun devicectl device process launch --device "$dev" "$ios_bundle"

# ---- screenshots --------------------------------------------------------------

# Capture the Android screen to build/screenshot-<app>-android.png — the
# per-device comparison the extraction plan does by hand today (launch with
# `just android-install app=<app>` first, then screenshot). Serial
# resolution identical to android-install: explicit param, else the first
# `adb devices` device, clear error if none. `adb exec-out`, never
# `adb shell`: the shell's pty converts LF to CRLF and corrupts the PNG.
screenshot-android app='rustnotes' serial='':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    s="{{serial}}"
    if [ -z "$s" ]; then
        s=$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print $1; exit}')
    fi
    [ -n "$s" ] || { echo "ERROR: no adb device connected."; exit 1; }
    mkdir -p build
    adb -s "$s" exec-out screencap -p > "build/screenshot-$key-android.png"
    echo "==> build/screenshot-$key-android.png"

# Capture the booted iOS simulator's screen to build/screenshot-<app>-ios.png
# (launch with `just ios-run app=<app>` first). Same exactly-one-booted-sim
# guard as ios-run: simctl's `booted` is arbitrary when several run.
screenshot-ios app='rustnotes':
    #!/bin/zsh
    set -euo pipefail
    cfg=$(just app-identity "{{app}}")
    eval "$cfg"
    just ios-one-booted-sim
    mkdir -p build
    xcrun simctl io booted screenshot "build/screenshot-$key-ios.png"
    echo "==> build/screenshot-$key-ios.png"

# ---- misc -------------------------------------------------------------------

# Type-check every crate for its real targets (rustnotes, hello-mobile
# and ui-kit-gallery are each checked three ways: Android lib, iOS-sim
# bin, host desktop bin)
# (the workspace can't be checked as a whole: ndk-sys only compiles for Android)
# (ui-kit is pure egui with no platform glue, so the host check covers it;
# its gallery app carries the platform legs through the rustnotes host)
check:
    cargo check -p rustnotes-core
    cargo check -p ui-kit
    cargo check -p ui-kit-gallery --lib --target aarch64-linux-android
    cargo check -p ui-kit-gallery --bin ui-kit-gallery --target aarch64-apple-ios-sim
    cargo check -p ui-kit-gallery --bin ui-kit-gallery
    cargo check -p rustnotes --lib --target aarch64-linux-android
    cargo check -p rustnotes --bin rustnotes --target aarch64-apple-ios-sim
    cargo check -p rustnotes --bin rustnotes
    cargo check -p hello-mobile --lib --target aarch64-linux-android
    cargo check -p hello-mobile --bin hello-mobile --target aarch64-apple-ios-sim
    cargo check -p hello-mobile --bin hello-mobile

# Link both Android cdylibs (debug), both backends: the one guard
# `cargo check` cannot give — a duplicate `android_main` symbol only
# surfaces at link, and only on Android's lld (macOS's ld64 drops the
# duplicate silently). Each app links alone, per-package: rustnotes with
# its default android-entry feature, hello-mobile with it opted out
# (crates/rustnotes/Cargo.toml has the symbol-collision rationale). The
# GameActivity legs prove the opt-in combination links too — one
# `android_main` per app under the swapped backend — which is the whole
# point of taking the backend as a feature: same entry contract, different
# activity class underneath. Not wired into CI — the full cross-compile
# is slow; run before an Android release.
check-links:
    cargo build -p rustnotes --lib --target aarch64-linux-android
    cargo build -p hello-mobile --lib --target aarch64-linux-android
    cargo build -p rustnotes --lib --target aarch64-linux-android \
        --no-default-features --features android-game-activity,android-entry
    cargo build -p hello-mobile --lib --target aarch64-linux-android \
        --no-default-features --features android-game-activity

# Format every crate (what `just lint` enforces)
fmt:
    cargo fmt --all

# Unit tests for the core and ui-kit crates, plus the gallery's headless
# input tests (the host crates are platform glue — no tests)
test:
    cargo test -p rustnotes-core
    cargo test -p ui-kit
    cargo test -p ui-kit-gallery

# Format check, then clippy over all four crates on the host feature set
# (the Android/iOS glue is additionally type-checked by `just check`)
lint:
    cargo fmt --all -- --check
    cargo clippy -p rustnotes-core -p rustnotes -p hello-mobile -p ui-kit -p ui-kit-gallery --all-targets -- -D warnings

clean:
    rm -rf build
