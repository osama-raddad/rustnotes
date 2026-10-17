#!/bin/zsh
# emu-touch — automated touch testing of the gallery on the Android
# emulator, verified by screenshot pixel-diffs.
#
# Prerequisites: zsh (the script's shell); adb on the PATH, with the
# emulator (or a device) attached and the gallery installed; Python 3
# with Pillow (`pip install pillow`) for the screenshot diffs.
#
# Two hard-won facts baked in:
# 1. `adb shell input tap/swipe` DOES reach NativeActivity apps. Earlier
#    "input is dead" sessions were a coordinate-scale error: the
#    emulator runs at pixels_per_point = 3.0 (1344x2992 px = 448x997
#    egui points). Always derive egui points as px/3, or taps land in
#    dead space and look like dead input.
# 2. Screenshot pixel-diffs are the oracle: a tap that works changes
#    >5% of pixels (panel switch); a scroll changes >10%; a no-op is
#    <1%. A changed-pixel count alone cannot tell a scroll from a text
#    selection (a selection highlight changes plenty of pixels too), so
#    every gesture that should come back is also diffed against the
#    screenshot from before it: a return to within 2% is the proof.
#
# The checks, in order (each one counts toward the exit status):
#   tap      the Foundations tab switches panels               (>5%)
#   scroll   a swipe up scrolls the page                       (>10%)
#   hold     the scrolled page stays put for 3s, no snap-back  (<1%)
#   return   swiping back down lands on the panel's first
#            screenshot exactly (a leftover selection fails)    (<2%)
#   rest     the returned page stays put for 3s                (<1%)
#   sideways a sideways swipe over a specimen row scrolls it    (>3%)
#   unswipe  the reverse swipe restores the row                 (<2%)
#
# The tap comes first because the checks after it start from the
# Foundations panel; it arms nothing. On a phone the gallery's scroll
# areas drag from the very first touch (`DragScroll::Always`), so a
# first swipe after launch would scroll too.
#
# The sideways row: the Brand tab has none (on a phone its theme cards
# stack one to a row), so the check scrolls the Foundations tab until
# the state tiles (the first row that scrolls sideways) sit mid-screen,
# with slow swipes so egui's fling adds little, then swipes sideways at
# a few heights around where the row should be: a swipe that misses the
# row changes nothing, the first one that hits it is the check.
# Retarget with the SIDEWAYS_* values below.
#
# Usage: tools/emu-touch.sh [serial]     # defaults to the first device
# Exits 0 when every check passes, 1 when one fails, 2 with no device.

set -euo pipefail

PKG=com.osamaraddad.vynatixgallery
SERIAL="${1:-$(adb devices | awk 'NR>1 && $2=="device"{print $1; exit}')}"
if [[ -z $SERIAL ]]; then
  echo "emu-touch: no device attached; start the emulator or pass a serial" >&2
  exit 2
fi
OUT="$(mktemp -d)"

# The sideways row, in device px (egui pt * 3). Measured headlessly at
# 448x997 pt with no status bar: unscrolled, the Foundations state
# tiles' band would sit at y 2038..2140 pt, so three slow 510 pt
# swipes (each adding ~15 pt of fling) bring it to about 463..565 pt.
# The device's status bar inset moves it down by its own height, hence
# the spread of heights to try, every 30 pt from 460 to 610 pt.
SIDEWAYS_SCROLLS=3
SIDEWAYS_SCROLL_FROM=2400
SIDEWAYS_SCROLL_TO=870
SIDEWAYS_SCROLL_MS=3000
SIDEWAYS_YS=(1380 1470 1560 1650 1740 1830)

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

swipe() { adb -s "$SERIAL" shell input swipe "$@"; }

# still NAME: shoot the screen once a second until two shots in a row differ
# by under 0.5%, and keep the last one as NAME.png. A fling can still be
# running a second or two after its swipe ends, so a fixed sleep is not enough.
still() {
  local name=$1 i
  shot "$name.0"
  for i in 1 2 3 4 5 6; do
    sleep 1
    shot "$name.$i"
    if (( $(diff_pct "$name.$((i - 1))" "$name.$i") < 0.5 )); then
      cp "$OUT/$name.$i.png" "$OUT/$name.png"
      return 0
    fi
  done
  echo "   $name still moving after 6 s"
  cp "$OUT/$name.$i.png" "$OUT/$name.png"
}

# back_to_top: wait for the page to stop, and swipe back again while it is
# not yet at the top edge, at most four more times.
back_to_top() {
  local n
  for n in 1 2 3 4; do
    still back
    (( $(diff_pct top back) < 2 )) && return 0
    swipe 540 700 540 2100 400
  done
  still back
}

# check NAME VALUE OP LIMIT: record a PASS or FAIL for VALUE OP LIMIT
# (OP is > or <) and print it.
summary=()
fail=0
check() {
  local name=$1 value=$2 op=$3 limit=$4 ok
  if [[ $op == ">" ]]; then
    (( value > limit )) && ok=1 || ok=0
  else
    (( value < limit )) && ok=1 || ok=0
  fi
  if (( ok )); then
    summary+=("$name=${value}%:PASS")
    echo "   ${value}% (want $op$limit) PASS"
  else
    summary+=("$name=${value}%:FAIL")
    echo "   ${value}% (want $op$limit) FAIL"
    fail=1
  fi
}

adb -s "$SERIAL" shell am force-stop $PKG 2>/dev/null || true
adb -s "$SERIAL" shell am start -n $PKG/android.app.NativeActivity >/dev/null 2>&1
sleep 3

# device px = egui pt * 3 (the emulator's pixels_per_point)

echo "-- tap the Foundations tab"
shot launch
adb -s "$SERIAL" shell input tap 370 600
sleep 1.2
shot top
check tap "$(diff_pct launch top)" ">" 5

echo "-- swipe up (scroll down the page)"
swipe 540 2100 540 700 400
sleep 2
shot down
check scroll "$(diff_pct top down)" ">" 10

echo "-- hold: the scrolled page stays put (no snap-back)"
sleep 3
shot held
check hold "$(diff_pct down held)" "<" 1

echo "-- swipe back down (twice, so the page lands on its top edge)"
swipe 540 700 540 2100 400
sleep 1
swipe 540 700 540 2100 400
back_to_top
check return "$(diff_pct top back)" "<" 2

echo "-- hold: the returned page stays put"
sleep 3
shot rested
check rest "$(diff_pct back rested)" "<" 1

echo "-- sideways: the state tiles scroll under the finger"
repeat $SIDEWAYS_SCROLLS; do
  swipe 540 $SIDEWAYS_SCROLL_FROM 540 $SIDEWAYS_SCROLL_TO $SIDEWAYS_SCROLL_MS
  sleep 1
done
sleep 1
SIDE=0
SIDE_Y=
for y in $SIDEWAYS_YS; do
  shot row
  swipe 1100 $y 300 $y 300
  sleep 1.5
  shot moved
  SIDE=$(diff_pct row moved)
  echo "   at y=$y: ${SIDE}%"
  if (( SIDE > 3 )); then
    SIDE_Y=$y
    break
  fi
done
check sideways "$SIDE" ">" 3

echo "-- unswipe: the reverse swipe restores the row"
if [[ -n $SIDE_Y ]]; then
  # 1000 px, longer than the forward swipe's 800 px, so the clamp at the
  # row's start (offset 0) is reachable. The shot waits until the row stops
  # moving, but a run can still settle short of the start (K4: 3.1% in one
  # of three runs), so this check stays informational.
  swipe 300 $SIDE_Y 1300 $SIDE_Y 300
  still restored
  check unswipe "$(diff_pct row restored)" "<" 2
  echo "   info: moved vs restored $(diff_pct moved restored)%"
else
  echo "   no row found to restore FAIL"
  summary+=("unswipe=n/a:FAIL")
  fail=1
fi

echo
if (( fail )); then
  echo "FAIL ${summary[*]}"
else
  echo "PASS ${summary[*]}"
fi
echo "screenshots in $OUT"
exit $fail
