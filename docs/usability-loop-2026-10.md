# Framework usability loop — October 2026

Status: in progress. Ten iterations of usability test → triage → fix → verify.

Branch `feat/framework-usability-iterations`, cut from `feat/design-notes-fixes`
(draft PR #23, tip `7918329`). This loop's PR stacks on #23.

## Scope

"The framework" is the pure-egui stack this repo builds:

- `crates/ui-kit` — the Vynatix design system: tokens, typography, states,
  and the components (button through snackbar).
- `crates/ui-kit-gallery` — the kit's demo and QA surface, every component live.
- `crates/rustnotes` (host) and `crates/rustnotes-core` (layout) — the eframe
  host and the first consumer app, which exercises the same layout system.

Test surfaces:

- Android: Pixel 10 Pro XL emulator, `emulator-5554`, 1344×2992 px at
  pixels_per_point 3 (448×997 pt). Packages `com.osamaraddad.vynatixgallery`
  and `com.osamaraddad.rustnotes`. Tablet AVDs `Snakebyte_Tablet` and
  `Snakebyte_Tablet_Design` exist for the size-class pass.
- iOS: iPhone 16 Pro simulator, UDID `1F96BFA0-C3BF-4C38-BBF5-1A9CADCA0C9D`,
  402×874 pt. Bundles `com.osamaraddad.vynatixgallery.ios` and
  `com.osamaraddad.rustnotes.ios`. Taps and screenshots only: `idb ui swipe`
  does not reach egui apps from this environment, so iOS gestures are
  reported as unverified.

Keystores: the worktree carries copies of the keystores that signed the
installed APKs (`debug.keystore` from `usability-fixes`,
`ui-kit-gallery.debug.keystore` from `design-notes-fixes`). Reinstalls keep
app data. The main checkout's `debug.keystore` does not match the installed
rustnotes, so installing from there would need an uninstall.

## Loop

1. Build and install both apps on both devices when the branch changed.
2. Test pass: an Android tester and an iOS tester run in parallel. They are
   read-only on code and save screenshots to `build/usability-audit/iter-N/`.
3. Triage: dedupe, then rank as blocker, major, minor, or nit. Blockers and
   majors ship in the same iteration; minors ship when they are cheap.
4. Fix batch under `subagent-driven-development`: one implementer at a time,
   then a spec-compliance review, then a code-quality review.
5. Verify: `just test`, `just lint`, and `just check` on the touched crates.
   The touched surfaces are re-screenshotted in the next iteration's pass.
6. Commit, push, and update this ledger.

## Schedule

Each iteration also runs a smoke check of both apps: launch, first screen,
one tap.

1. Gallery Brand and Foundations, phone portrait, default theme. Open
   follow-ups from the Oct 14 pass: photo-tile captions not truncated, Brand
   principle card too tall on phones. Re-verify the PR #23 vertical swipe on
   a specimen row.
2. Gallery Components: every control's default, touch-hold, disabled, and
   focus states.
3. Gallery Feedback: snackbar and banner stacking, dismiss, the theme menu
   popover.
4. Landscape on both apps: the mirrored cutout inset from PR #23 on the
   Pixel, and chrome symmetry.
5. Large text: Android `font_scale` 1.3 and 1.5, iOS Dynamic Type extra-large,
   reduced motion. Truncation, wrap floors, ellipses.
6. Night mode across all six themes: contrast tokens. Extend the WCAG tests.
7. rustnotes touch flows: list, create, edit, swipe-to-delete,
   pull-to-refresh, two-tap delete, editor hold-to-select, autosave.
8. Size classes: tablet AVD for list–detail two-pane; phone landscape
   single-pane.
9. iOS visual pass, taps only. Gestures stay unverified there.
10. Regression sweep: lint, test, check, `tools/emu-touch.sh`,
    re-screenshot every surface touched in 1–9, and a review of the branch
    diff.

## Iteration log

Each iteration records the build SHA, the test-pass result, the findings
with IDs and severities, the fix commits, anything deferred and why, and the
checks that passed.

### Iteration 1: baseline pass (gallery Brand and Foundations, rustnotes smoke)

Build: branch tip `7918329` (no changes yet). Both apps rebuilt from this
worktree. `just test` passed (84, 227, 1, 23, 14 and 5 tests, 0 failures).

Testers: one Android tester (Pixel 10 Pro XL emulator) and one iOS tester
(iPhone 16 Pro simulator), run in parallel. Screenshots are under
`build/usability-audit/iter-1/{android,ios,repro}/`.

Incident: `just run-ios` prefers a connected iPhone over the simulator. The
baseline run installed Vynatix Gallery on a connected iPhone 17 Pro
(`Räddad iPhone`, `07DBE8D0-DEAC-50B8-AE9B-492146EE6B11`). It was never
launched, because the phone was locked. It has not been removed. From now on
use `just ios-run` for simulator-only installs.

Findings (ID, severity, surface, and disposition):

- **1-A3, major, gallery.** Every panel shares one page scroll offset, because
  `PAGE_SCROLL` is a fixed id in `gallery.rs`. Scroll Brand down, tap
  Foundations, and it opens mid-page. **Fix: task 1 this iteration.**
- **1-I1, major, Theme popover toggle.** The OFF knob hangs half outside the
  track. The ON knob stops mid-track. **Fix: task 2 this iteration.**
- **1-A2, major, Brand photo tiles.** Captions overflow their tiles, and
  "Gothenburg" is clipped to "Gothenbu". **Fix: task 3 this iteration.**
- **1-A1, major, Foundations STATES row.** "Start a project" is clipped at
  both ends in all five tiles. **Fix: task 4 this iteration.**
- **1-R1, blocker, host lifecycle. Open; needs a decision.** After a Back
  exit at the gallery root, relaunching in the same process shows the Android
  splash and never reaches the first frame. Logcat: "Creating: Activity", then
  "WaitableNativeActivityState::drop!", and no "Vynatix Gallery ready". The app
  recovers only in a fresh process. Reproduced 2 of 2 on the gallery. The
  Rust Notes relaunch could not be confirmed in the same sequence, because the
  gallery came forward instead. This is a NativeActivity lifecycle issue, not a
  panel bug. The README's "leaving via back at the root is clean" claim does
  not hold for relaunch in the cached process.
- **1-R2, major, seen once, not reproduced.** Back with the search popover open
  exits the app. Expected: Back closes the popover first. Deferred to iteration
  3, with the popover work.

Deferred to later iterations:

- 1-A4 and 1-I2, Brand hero band: hairline seams cut every swatch, and the last
  swatch ends 23 px short of the margin. Iteration 2.
- 1-A5 and 1-I3, top bar: the wordmark sits above the bar's centre line, and
  the trailing gap is wider than the leading gap. Iteration 2.
- 1-A7, captions: dangling "·" and an orphan "1" in the 60/30/10 captions.
  Iteration 2. The Teal Light swatch is nearly invisible, which is iteration 6.
- 1-A10, "Book a call" shows a pressed state, and the other taps do nothing.
  They may be specimens by design; needs a decision. Iteration 2.
- 1-A13, touch targets: the "Tab to me" pill is 18 pt tall, and state buttons
  are about 58 pt. Iteration 2. Rust Notes' ✕ (32 pt) and "+" (40 pt) are
  iteration 7.
- 1-A6, fixed-width tiles hug the left edge on a phone. Iteration 5.
- 1-A9, principle card 01 is about 443 pt tall. This is the known follow-up
  from the Oct 14 pass. Iteration 5.
- 1-A11, Foundations spacing and elevation specimens are staggered, "Radius
  Lg · 12" wraps, and the first tile's left border is clipped. Iteration 5.
- 1-A12, eight colour-role swatches are invisible without an outline, and the
  eyebrow spec shows "1.4545455". Iteration 6.
- 1-I4, theme popover rows are 32 pt apart, the toggles sit 7 pt off the radio
  column, and their labels are 28 pt off. Iteration 3.
- 1-I5, iOS only: 28% of the screen is chrome before any content. Verify on
  Android first. Iteration 9.
- 1-I7 and 1-I8 (nits), Foundations clipped specimen at the fold and theme
  popover placement. Iterations 5 and 3.
- 1-I9, Rust Notes date right-aligned and floating (nit). Iteration 7.
- 1-R3, search popover: the magnifier overlaps the placeholder, and the popover
  sits 6 px from the edge (single capture). Iteration 3.

Fix batch for iteration 1: tasks 1 to 4 (findings 1-A3, 1-I1, 1-A2, 1-A1),
run one at a time under `subagent-driven-development`. Each task gets a spec
review, then a quality review.

Shipped in iteration 1 (each verified green by the coordinator):

- `77e82bf` gallery: each panel keeps its own page scroll offset (1-A3). Spec
  compliant. Quality approved; four optional doc nits deferred.
- `4ca8a35` ui-kit: the toggle knob stays inside the track (1-I1). The painter
  treated the knob's left edge as its centre. Spec compliant. Quality approved.
- `339a1bf` ui-kit: photo-tile captions wrap inside their tile (1-A2). Spec
  compliant. Quality approved, and the reviewer recommended keeping the ellipsis
  helper. A synthetic test covers it.
- `ad1b558` (now `67d5848`, amended for the message) ui-kit-gallery: state-row
  buttons fit their labels (1-A1). The tiles widen from 96 to 143 pt in Light,
  which the spec review accepted. The commit message's caption-wrap claim was
  corrected after review. Spec compliant with caveats. Quality approved.
- `a4dfc7f` test: the specimen-row swipe test keeps a sideways pointer jump,
  parked leftward. Test-only. Its comment and a body-height guard were fixed.

Known gaps, recorded rather than fixed:

- **K1, rightward pointer jump.** A rightward jump in the first touch frame
  moves a specimen row, which is clamped at offset 0. The vertical-drag test
  therefore parks the pointer leftward. Real touches send `PointerGone` between
  fingers, so the jump should only come from mixed mouse and touch input, and
  it is not expected on a phone. A fix, or an `#[ignore]`d regression test, is
  still open.
- **K2, axis-lock coverage.** The jump test's coverage of the origin-based axis
  lock is argued from the code. A mutation check was denied by the classifier,
  so it is unverified.
- **K3, freeze on relaunch (1-R1, blocker, open).** Still open. This blocks
  unattended testing after a Back exit, so testers avoid Back and use
  `am force-stop` between runs. The options are: end the process when the
  activity finishes at the root, so every launch is fresh (simple, changes the
  cached-process behaviour the README describes); or investigate the
  android-activity re-entry directly (open-ended). A decision is needed.

Device re-verification of the four fixes (build `a4dfc7f`, both targets):

- **Android (Pixel 10 Pro XL), all PASS.** V1 captions wrap inside their tiles.
  V2 "Start a project" is whole in every state tile. V3 Brand returns to its own
  offset after Foundations (checked on the Brand side). V4 the knob sits inside
  the track in OFF and ON. V5 smoke passes.
- **iOS (iPhone 16 Pro), V4 PASS.** The knob sits inside the track in OFF and
  ON. V1, V2 and V3 are below the fold, and taps cannot scroll here, so they are
  NOT CHECKED.
- Observed again and already deferred: the Brand swatch seams (1-A4, iteration
  2) and the single-column Brand layout (1-A6, iteration 5). Not isolated: whether
  Foundations restores its own offset after leaving it.

Iteration 1 closed. Its fixes are green in the suites and verified on Android.
