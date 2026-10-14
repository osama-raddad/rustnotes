#!/bin/zsh
# emu-touch — automated touch testing of the gallery on the Android
# emulator, verified by screenshot pixel-diffs.
#
# Two hard-won facts baked in:
# 1. `adb shell input tap/swipe` DOES reach NativeActivity apps. Earlier
#    "input is dead" sessions were a coordinate-scale error: the
#    emulator runs at pixels_per_point = 3.0 (1344x2992 px = 448x997
#    egui points). Always derive egui points as px/3, or taps land in
#    dead space and look like dead input.
# 2. Screenshot pixel-diffs are the oracle: a tap that works changes
#    >5% of pixels (panel switch); a scroll changes >10%; a no-op is
#    <1%.
#
# Usage: tools/emu-touch.sh [serial]     # defaults to the first device

set -euo pipefail

PKG=com.osamaraddad.vynatixgallery
SERIAL="${1:-$(adb devices | awk 'NR>1 && $2=="device"{print $1; exit}')}
"
SERIAL=$(echo $SERIAL | tr -d '[:space:]')
OUT="$(mktemp -d)"

shot() { adb -s "$SERIAL" exec-out screencap -p > "$OUT/$1.png"; }

diff_pct() {
  python3 - "$OUT/$1.png" "$OUT/$2.png" <<'PY'
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert('RGB')
b = Image.open(sys.argv[2]).convert('RGB')
w, h = a.size
t = len(range(0, h, 10)) * len(range(0, w, 10))
d = sum(1 for y in range(0, h, 10) for x in range(0, w, 10)
        if a.getpixel((x, y)) != b.getpixel((x, y)))
print(f"{100 * d / t:.1f}")
PY
}

adb -s "$SERIAL" shell am force-stop $PKG 2>/dev/null || true
adb -s "$SERIAL" shell am start -n $PKG/android.app.NativeActivity >/dev/null 2>&1
sleep 3

# device px = egui pt * 3 (the emulator's pixels_per_point)
fail=0

echo "-- tap the Foundations tab"
shot a
adb -s "$SERIAL" shell input tap 370 600
sleep 1.2
shot b
TAP=$(diff_pct a b)
echo "   changed: ${TAP}% (want >5)"
(( ${TAP%.*} > 5 )) || fail=1

echo "-- swipe up (scroll down the page)"
shot a
adb -s "$SERIAL" shell input swipe 540 2100 540 700 400
sleep 1.2
shot b
DOWN=$(diff_pct a b)
echo "   changed: ${DOWN}% (want >10)"
(( ${DOWN%.*} > 10 )) || fail=1

echo "-- swipe back down (return to origin)"
shot a
adb -s "$SERIAL" shell input swipe 540 700 540 2100 400
sleep 1.2
shot b
BACK=$(diff_pct a b)
echo "   changed: ${BACK}% (want ~0: exact return)"

echo "-- hold: no snap-back"
shot a
sleep 3
shot b
HOLD=$(diff_pct a b)
echo "   changed: ${HOLD}% (want ~0)"

echo
echo "tap=$TAP% scroll-down=$DOWN% scroll-back=$BACK% hold=$HOLD%"
echo "screenshots in $OUT"
exit $fail
