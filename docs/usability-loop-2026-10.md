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

### Iteration 2: Components panel

Testers: Android (Components walk, interactions, pressed state, focus, top bar,
known-item re-checks) and iOS (taps only). Screenshots are under
`build/usability-audit/iter-2/`. Build: `a4dfc7f` plus the ledger commits.

Confirmed working: the Android pass found checkbox (tri-state), radio, toggle,
and chip all change state on tap, on the control and on its label. The chip
dismiss and Restore both work. The pressed look appears on hold. Known items
from iteration 1: the lone "1" in the 60/30/10 captions is fixed, and the photo
captions wrap inside their tiles.

Findings and dispositions:

- **2-I1, major, Feedback snackbar.** The "Saved." snackbar stays past its 6 s
  until the next touch. It was still up at 7 s, 9 s and 17 s idle, and a touch
  cleared it. Likely a missing timed repaint, so the timer expires without a
  frame to draw it. **Fix: task 5.**
- **2-A1, major, Components metric cards.** Card 2 breaks mid-word, card 3 puts
  one letter on each line, and the tall card sets the row height, leaving about
  320 pt blank. **Fix: task 6.**
- **2-A2, major, Components text inputs.** The text has no left padding, and the
  search magnifier overlaps the placeholder. **Fix: task 7.**
- **2-A3, major, Components select popover.** The popover stays open after an
  outside tap, reproduced three times. **Fix: task 8.**
- **1-A4 and 2-I2, minor, Brand swatch seams.** Still present in both passes.
  **Fix: task 9.**
- **1-A5, 2-A5 and 2-I3, minor, top-bar wordmark.** The wordmark sits about
  10 pt above the bar's centre line, and the trailing gap is wider than the
  leading gap. **Fix: task 10.**
- **2-I4, minor, wordmark rendering.** The wordmark's edges are hard, with no
  anti-aliasing, unlike other text. Deferred to iteration 6 (rendering and
  contrast).
- **2-A4, minor, blank strip after the keyboard closes.** The bottom of the page
  stays empty until the next scroll. Deferred to iteration 5.
- **2-A6, minor, interactive controls below 44 pt.** Filter chips are 31 pt,
  radio rows are 32 pt apart, checkbox rows 35 pt, select options 33 pt, and the
  "Filter results" Small button 36 pt. Deferred to iteration 5 (layout and touch
  targets).
- **2-A7, minor, focus ring and tab order.** The ring is clipped at the left edge
  of the Brand tab and merges with the underline. The tab order runs Theme, then
  Search, the reverse of the visual order. Deferred to iteration 3 (chrome and
  popovers).
- **2-A8, minor, Tab-to-me pill.** About 17 pt tall, with the label touching its
  outline. This is the 1-A13 item. Deferred to iteration 5.
- **2-A9 and 2-I5, nit, low-contrast labels.** "Disabled" is 1.1:1 against its
  fill, which is the documented 38%. "Sending…" is an active state and reads
  about 1.2:1, which is a problem. Deferred to iteration 6 (contrast).
- **2-A10, minor, cards.** `.card--filled` has no visible surface. Card widths
  vary, and none fill the page width. Deferred to iteration 5.
- **2-A11, minor, right gutter.** The right gutter is 6 to 7 pt against 16 pt on
  the left. Inputs and paragraphs run past the bar's right edge at 1296 px.
  Deferred to iteration 5.
- **1-A6 and 2-A12, minor, Brand single column.** Columns are narrow, and the
  right third is empty. Deferred to iteration 5.
- **2-A13, nit, glass specimens.** The primary glass tile runs off its edge, and
  shapes spill outside their tiles. Deferred to iteration 5.
- **2-I8, minor, bottom clip on iOS.** Content is hard-clipped at about 816 pt,
  leaving a blank strip, and the snackbar's corners look notched. This is the
  safe-area clip from the iteration-1 mapping. Deferred to iteration 9.
- **2-I9, minor, bar edge and gap on iOS.** The bar's bottom edge fuses with the
  full-width divider, and about 55 pt sits empty between the bar and the tab
  labels. This is the 1-I5 chrome item. Deferred to iteration 9.
- **Not a defect.** The "Start a project" button in Components gives no visible
  response to a tap. It is a demo button with no action, and its pressed look is
  the feedback. The state tiles in Foundations are painted specimens by design.
- **Residual from iteration 1.** On Brand, the caption "After-hours debug ·
  Gothenburg HQ" ends one line with a trailing "·". The fix is to join the middle
  dot to its neighbours with non-breaking spaces. Deferred to iteration 5 (copy
  and layout).

Fix batch for iteration 2: tasks 5 to 10 (findings 2-I1, 2-A1, 2-A2, 2-A3,
1-A4 and 1-A5), run one at a time under `subagent-driven-development`, with a
spec review and then a quality review for each.

Progress on the batch (each commit verified green by the coordinator):

- **Task 5, snackbar (2-I1): `d64bb06`, test follow-up `a4a986b`.** The
  mechanism was not the hypothesised missing request. A timed request existed.
  The frame that crosses the deadline paints the snackbar, the tick then drops
  it, and the timed request after that is skipped, so nothing asks for the
  frame that clears the screen. The fix requests that frame, and a held clock
  asks for no timed repaint. The follow-up made the deadline guard able to fail,
  which was verified by mutation. **Open:** the iOS timed wake is unverified off
  the device. Re-check the snackbar on the simulator in the iteration-2
  verification.
- **Task 6, metric cards (2-A1): `688e280`.** The floor is 208 pt: 160 pt content
  plus 24 pt padding each side. Cards no longer squeeze, and the row scrolls.
  **Residual, deferred to iteration 5:** at 384 pt, card 1 is about 163 pt tall
  and cards 2 and 3 about 196 pt, so the heights differ. The reviewer accepted
  this as the unavoidable result of the floor. Quality nits are deferred to the
  test-quality sweep (see below).
- **Task 7, text fields (2-A2): `43bc8c8`.** The cause is confirmed in egui 0.36.2:
  an explicit `TextEdit::frame(..)` replaces the builder's margin. The margin
  now lives in `Frame::inner_margin`. Insets are 14 pt without a leading icon,
  44 pt with one, and 12 pt vertically for the textarea. The test-strength items
  are deferred to the sweep: assert the wrap width directly, and ADD the
  right-inset check that does not rely on word-wrap luck.
- **Task 8, select (2-A3): `0ac89f8`, test follow-up `c9988e6`.** The premise was
  corrected. This is **not** a touch bug. A mouse click outside the open list
  fails the same way, and the existing outside-click test only covers the pure
  state machine, so it could not catch the bug. The cause: the popover was built
  with `Popup::from_response` and no `open_memory`, so egui treats it as always
  open and never applies the close. The fix is `.open_memory(None)`, and the
  mouse test now opens and closes by mouse only, verified by mutation.
- **New finding 2-A15, major, Components select.** Tapping to open the Region
  select scrolls the page about 96 pt. The cause is unknown. A likely suspect is
  `scroll_to_rect(row, Center)` in the popup rows. Found by the implementer,
  pre-existing, and not part of the select fix. **Deferred to iteration 3**
  (chrome and popovers).

Test-quality backlog, to be taken in the iteration-10 sweep, all test-only and
none blocking:

- Metric cards: the `MIN_WIDTH` padding round-trip (add a comment or a
  `CONTENT_MIN_WIDTH` constant), the "148px" doc figure that no test pins, a
  `build` helper for the repeated card-building block, and the gallery test's
  "window's edge" wording (it means the page's content edge).
- Text fields: the wrap-width assertion, `painted_box` moved to `typography.rs`
  as `pub(crate)`, one helper for the three single-line blocks, named `EPS` and
  `NARROW_WIDTH` constants, and renaming the multiline test "top, left and right".
- Snackbar: none beyond what was fixed.

### Iteration 2 closed

Build for the device pass: `1b38fca` (Android installed, iOS simulator
installed from `373819f`; the simulator's wordmark and snackbar checks were
on the fixes as they then stood).

Fixed and verified:

- **2-I1 snackbar auto-dismiss.** `d64bb06`, follow-up `a4a986b`. Verified on
  Android (gone by about 7.5 s) and on the iOS simulator (on screen at 5.7 s,
  gone at 6.7 s, no touch). This closes the open iOS wake question.
- **2-A1 metric cards.** `688e280`. The floor is 208 pt. Verified on Android:
  whole words, the row scrolls. The uneven heights (about 163 against 196 pt)
  are accepted as the result of the floor, and they are a layout item for
  iteration 5.
- **2-A2 text inputs.** `43bc8c8`. Verified on Android: insets of 14 pt, and
  44 pt in the search field, with no overlap by the magnifier.
- **2-A3 select outside tap.** `0ac89f8`, follow-ups `c9988e6`. The premise was
  not touch-specific: a mouse click failed the same way. Verified on Android:
  the list closes on a tap outside, and the value is unchanged.
- **1-A4 swatch band, right edge.** `e47784b`, docs `cdc464d`. The band now
  fills the column. Verified on iOS and Android (the right edges match).
  **The hairlines are kept, as a documented design element.** See decision D2.
- **1-A5 top-bar wordmark.** `373819f`, follow-up `ec159c1`, focus fix `747dcfd`,
  naming `1b38fca`. The wordmark is on the centre line, verified on iOS (within
  1 px) and Android. **Trade-off, see decision D3.**
- **Keyboard focus ring on the Theme trigger (a regression I introduced).**
  The overhang had clipped it. `747dcfd` keeps 4 pt clear of the edge, and
  `1b38fca` names the constant once. Verified on Android: the ring is whole,
  with a 9 px gap.

Open decisions for you (the loop does not wait on them):

- **D1, relaunch freeze after Back (K3, blocker).** The gallery freezes on
  relaunch after Back at its root. The options are to end the process when the
  activity finishes at the root, which makes every launch fresh and changes the
  cached-process behaviour that the README describes; or to investigate the
  android-activity re-entry directly. Until you decide, the testers avoid Back.
- **D2, swatch hairlines.** Three surface-coloured rules cross the band. The
  module doc and a test assert them as the "ruled stock" design element, from the
  source brand preview. I kept them and closed the seam part of 1-A4 as by design.
  Say the word and the three `hline` calls go, in one small commit.
- **D3, the trailing gap.** The labelled Theme trigger's chevron now sits 28 pt
  from the pane edge, against 24 pt for the wordmark (box terms), and about 33 pt
  against 24 pt in ink terms. The 4 pt keeps its focus ring inside the bar. The
  equal-gap intent cannot hold for this control with the kit's 20-pt button
  padding. The options are to accept this, to give the bar's trigger its own
  smaller padding (a kit design change), or to let the ring paint outside the
  pane clip.
- **D4, the wordmark reads "Vnatix" at a glance.** The "y" tucks into the V
  monogram. It matches the large logo on the Brand tab, so I treated it as
  intentional. Confirm that, or flag it for the brand.
- **D5, a personal iPhone has the gallery installed.** My baseline run installed
  the Vynatix Gallery on your connected iPhone 17 Pro (`Räddad iPhone`,
  `07DBE8D0-DEAC-50B8-AE9B-492146EE6B11`). It was never launched. It is still
  there. Say the word and I will remove it with `xcrun devicectl device uninstall
  app`.

Moved to later iterations, with the reason:

- Iteration 3 (chrome, popovers and focus): the Brand tab's focus ring is clipped
  on its left edge by the tab strip (Android, verified). Tab order runs Theme,
  then Search, the reverse of the visual order. The Region select opening scrolls
  the page about 96 pt (2-A15). The bottom hairline on the top bar runs past its
  rounded corners. The snackbar floats over its own trigger row (cosmetic).
  Theme and search popover placement, and the theme-menu row spacing (1-I4).
- Iteration 5 (layout and touch targets): the Components right gutter (6 to 7 pt
  against 16 pt on the left, 2-A11). Controls below 44 pt (2-A6). Card surfaces
  and widths (2-A10). The Brand single column (1-A6). The keyboard-dismiss blank
  strip (2-A4). The glass specimen spill (2-A13). The Tab-to-me pill (2-A8). The
  metric heights (accepted, see above). The residual trailing "·" in a caption.
- Iteration 6 (rendering and contrast): wordmark anti-aliasing (2-I4). Disabled and
  "Sending…" labels (2-A9 and 2-I5).
- Iteration 9 (iOS pass): the bottom clip and notched snackbar corners (2-I8), and
  the bar's edge and gap on iOS (2-I9).
- Test-quality sweep (iteration 10): the items in the backlog above.

Harness notes, for future passes: `idb ui tap` needs `--duration 0.15` for the
Save trigger, because default-length taps were missed. Gestures do not reach the
iOS apps, so iOS passes can only check what is reachable by taps.

### Iteration 3: chrome, popovers and focus

Testers: Android (Feedback, Theme and search popovers, tab order and focus rings,
the select's scroll on open, the top bar's edge) and iOS (taps only). Screenshots
are under `build/usability-audit/iter-3/`. Build: `1b38fca` plus the ledger
commit `4daef56`.

Confirmed working: the Theme re-skins the page, and the popovers dismiss on a
bare-canvas tap. Both banners dismiss, and "Restore the banners" brings them
back. The snackbar works. The select's page does not scroll on open (Android,
0 px). That closes the headless-only finding 2-A15: it was a harness effect,
and it does not reproduce on the device.

Triage. Majors ship in this iteration:

- **3-A1 / 3-I1, major, Theme popover rows.** Radio rows are about 32 pt apart and
  toggle rows about 41 pt, under the 44-pt touch floor. The toggle labels also
  sit 28 pt right of the radio labels. **Fix: task 11.**
- **3-A2, major, top-bar tab order.** Tab goes Theme, then Search, while Search
  sits left of Theme on screen. **Fix: task 12.**
- **3-A3, major, Brand tab focus ring.** The ring is clipped on its left edge,
  at the content margin (x 48 px). The Foundations, Components, Feedback, Theme and
  Search rings are whole. **Fix: task 13.**

Minor, ship in this iteration:

- **3-A4 / 3-I2 / 3-I3, minor, popover anchoring.** The popover tops sit about
  11 pt inside the bar card, covering its bottom border. The Theme popover's left
  edge lines up with nothing, and its right edge sits 8 pt inside the bar's edge.
  The search popover is centred under its icon instead. **Fix: task 14.**
- **3-A5 / 3-I7, minor, search popover padding.** It is about 5 pt inside its edge,
  against 16 pt in the Theme popover. **Fix: task 14.**
- **3-A9, minor, stale over-scroll after a banner dismiss at the end of the page.**
  The page stays over-scrolled, with blank space below, until the next input. The
  same kind of missed repaint as the snackbar. **Fix: task 15.**
- **3-A7 / 3-A11, minor, top bar's bottom edge.** A full-width hairline runs under
  the card's border, so the edge reads as a double rule, and at the corners the
  curve lifts off the line. **Fix: task 16.**

Deferred, with the reason:

- **3-A6 / 3-I6, minor, search popover covers the tab strip.** A popover over
  content is normal. Whether it should cover the tabs, or close on a tab change, is
  a design decision. **Decision D6.**
- **3-I4 / 3-A12, nit, the Theme chevron does not flip when open.** The icon catalogue
  describes `expand_more` as "open a menu or section", so the design intent is
  unclear. **Decision D7.**
- **3-I8, nit, the snackbar covers its trigger row.** A snackbar floats over content
  by design. Left as it is.
- **3-I9 and 3-I10, nit, "Restore the banners" has no affordance, and the banner X is
  a bare 12-pt glyph.** The hit area is adequate (about 36 pt). Iteration 5
  (affordance and layout).
- **3-A10, nit, the snackbar and paragraph spacing.** Iteration 5.
- **3-A8, minor, form fields overhang the right gutter.** This is the 2-A11 item.
  Iteration 5.
- **Not reproduced.** An editorial-strip swipe that scrolled the strip rather than the
  page, once. It may be the horizontal-drag issue from PR #23. The seventh Tab stop
  shows no ring.

Fix batch for iteration 3: tasks 11 to 16, run one at a time under
`subagent-driven-development`.

### Iteration 3 closed

Build for the device re-check: `03858a2` (Android installed, iOS simulator
installed from the same head). Every fix below was spec-reviewed and
quality-reviewed, and each review's follow-ups landed as comment or test
commits.

Fixed and verified on the device:

- **3-A1 and 3-I1, Theme popover rows.** `ca1b391`. The rows are 44 pt, and the
  labels align. The kit gained an additive, opt-in row-height API, and its defaults
  are unchanged. Verified on both platforms: the rows are spaced generously.
- **3-A2, top-bar tab order.** `12c256e`, follow-up `0beed03`. egui hands Tab focus
  along in creation order, and the group had been built right-to-left. The layout
  is pixel-identical (checked across 288 bars). Verified on Android: Search comes
  first, then Theme.
- **3-A3, Brand tab focus ring.** `206daff`, follow-up `10bfbad`. The tab strip's
  scroll clip cut the first tab's ring. The clip is now widened along x around the
  ring's paint, and no tab moves. Verified on Android: the ring is whole.
- **3-A4, 3-I2, 3-I3 and 3-A5, 3-I7, popover anchoring and padding.** `aa6cb7f`,
  now `caa29e4`, with comment follow-ups `1b0066e`. Both popovers open 8 pt under
  the bar and share one rule: the right edge stands on the trigger's right edge.
  The search input is inset 17 pt (16 pt and the border). Verified on both
  platforms.
- **3-A9, banner dismiss at the page end.** `7bd2365`, follow-up `dbc72ff`. The
  first painted frame now shows the clamped page. A `request_discard` runs only
  when the content height changed and the offset is past the new maximum. A plain
  scroll takes one pass per frame. Verified on Android: the capture immediately
  after the tap matches the one two seconds later.
- **3-A7, 3-A11 and 3-I11, the top bar's bottom edge.** `ab9dfdd`, follow-up
  `03858a2`. The double rule was egui's default separator line on the top panel.
  The line is off, and its reserved room is given back as clearance, so nothing
  below the bar moves. Verified on both platforms: one clean rule.
- **The select.** The page does not scroll when the Region select opens. The
  headless finding 2-A15 did not reproduce on the device, so it is closed.

Known and accepted:

- **The Theme toggles' glyphs.** With the labels aligned, the toggle knobs sit about
  24 pt left of the radio circles, in the off state. This is the trade-off from
  `ca1b391`. Decision D8 below.
- **The trailing gap of the labelled trigger** (decision D3 above) is unchanged.
- **The search popover on a phone** is held against the bar's left edge, not hung
  from its trigger, because it is too wide to fit to the right of a trigger at 233 pt.
  This is the designed clamp, and the rule otherwise holds. Decision D9 below.
- **The Theme popover's right edge** sits 8 pt inside the bar's right edge, because
  the rule anchors on the trigger's right edge. This is by design.

Decisions for you, new in this iteration:

- **D6, the search popover covers the tab strip** on a phone. A popover over content
  is normal, but this one hides the tab labels. Whether it should close on a tab
  change, or leave the strip uncovered, is a design choice.
- **D7, the Theme chevron does not flip** when the menu opens. The icon catalogue
  describes `expand_more` as "open a menu or section", so the intent is unclear.
- **D8, toggle glyphs versus labels** (above). Accept the trade-off, or give the
  toggles and radios their own label inset.
- **D9, the held search popover** (above). Accept it, or anchor the search popover
  on the bar's right edge instead.
- **The "Banners" label at the page's end** sits half under the tab strip, because
  the page scrolls to its limit after the "Restore the banners" link takes back the
  freed height. This is the scroll limit, not the fix. Decide whether the label
  should stay visible at the limit.

Moved to later iterations, with the reason:

- Iteration 4 (landscape and orientation): the Theme popover is about 111 pt taller
  than it was, and in landscape on a phone, about 360 to 412 pt of height, it
  overflows with no scroll area. This came from the `ca1b391` concern. Landscape
  is Android-only for the device pass, because the iOS simulator cannot rotate
  from here.
- Iteration 5 (layout and touch targets): the Components right gutter, the
  controls below 44 pt, the card surfaces, the Brand single column, the
  keyboard-dismiss blank strip, the glass specimen spill, the Tab-to-me pill, the
  form fields' overhang of the right gutter, "Restore the banners" with no
  affordance, the banner dismiss glyph, and the snackbar-and-paragraph spacing.
- Iteration 9 (iOS pass): the bottom clip on both the Feedback page and the
  Components page (about 814 to 816 pt), and the bar's edge and gap on iOS.
- Test-quality sweep (iteration 10): the backlog above, and the new items from the
  iteration-3 reviews. These are the golden tables and their re-recording notes,
  the shape walkers that repeat across the test files, and the minor doc wording
  in the reviews.

Not reproduced: an editorial-strip swipe that scrolled the strip rather than the
page, once. It may be the horizontal-drag issue from PR #23. The seventh Tab stop
shows no ring.

Still open from earlier: the relaunch freeze after Back (D1, the blocker), the
swatch hairlines as a design element (D2), the personal iPhone install (D5), and
the wordmark's legibility (D4).
