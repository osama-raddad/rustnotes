# Fork triggers — when the eframe layer stops being a layer

Status: baseline record, 2026-10-13, after the seam (#14), second-consumer
(#15), packaging (#16) and GameActivity-opt-in (#17) waves. This file
exists because the extraction plan's Q1 said so: *"stay a layer while the
holes stay enumerable, push the generic ones upstream, and write down the
fork triggers before the list doubles again."*

The layer's bet is that every non-standard touchpoint below has an
upstream exit. A fork wins when an exit closes and the workaround becomes
permanent. Re-count the inventory at each wave that touches the host; the
discipline is the count, not the wording.

## The hole inventory (baseline: nine)

Where the layer reaches past eframe's supported surface, why each hole
exists, and its upstream exit:

| # | Hole | Where | Why it exists | Upstream exit |
|---|---|---|---|---|
| 1 | Creation-time font injection (`platform_font` through `App::new`) | `crates/rustnotes/src/lib.rs` (`platform_font`, `App::new`) | eframe has no font hook at creation; iOS devices have no readable font at the core's candidate paths | an eframe creation-time font hook |
| 2 | iOS lifecycle observers via a main-thread pointer (`HOST_PTR`, re-armed every frame) | `lib.rs` + `ios.rs` (`set_lifecycle_save_hook`) | winit emits nothing on iOS backgrounding and `on_exit` is dead under `UIApplicationMain`; saves must not depend on destroy-time callbacks | eframe lifecycle hooks (pause/resume/backgrounding) |
| 3 | Exit by `ViewportCommand::Close` → `run_native` returns | `lib.rs` (`leave_app`) | the only deadlock-free exit under NativeActivity (`Activity.finish()` deadlocks the glue) | none needed — sanctioned eframe behaviour; becomes a trigger only if the semantics change (T5) |
| 4 | Panic routing (logcat + forced backtrace / NSLog) | `android.rs`, `ios.rs` | stderr is unreadable on device | eframe mobile panic-hook conveniences (nice-to-have, not blocking) |
| 5 | `raw_input_hook` as the platform-injection point: JNI window insets incl. IME, night mode, fontScale→zoom | `android.rs` | egui-winit populates `safe_area_insets` only on iOS; no OS theme or text scale on Android | winit `Window::safe_area` (0.31) consumed by egui-winit cross-platform |
| 6 | iOS appearance / Dynamic Type / keyboard-frame reads through objc2 | `ios.rs` | same gaps on iOS (theme, text scale) | winit/egui-winit reporting OS theme + text scale on mobile |
| 7 | iOS wgpu limits (adapter-supported, not WebGPU defaults) | `ios.rs` (`wgpu_options`) | eframe exits at startup on Metal with default limits | an eframe mobile renderer preset |
| 8 | NativeActivity statics reset on `android_main` re-entry | `android.rs` (`set_app`) | NativeActivity re-enters `android_main` in one process; stale statics blind the new activity | upstream android-activity awareness is partial; this one is ours regardless of fork |
| 9 | The GA packaging boundary: dex + the appcompat resource-merge wall | `justfile` (`android-stage` game flavor), plan addendum #3 | games-activity is a Java library whose pom lies about its dependencies; a no-Gradle GA path needs a mini-AGP | an upstream packaging story for GameActivity apps, or gradle delegation as product (T3) |

Not holes, for the record: the seam itself (#14 — an ordinary trait), the
per-app identity table (#16 — repo tooling), and `android_main`-as-feature
(#15/#17 — packaging correctness, fork-irrelevant).

## The triggers

- **T1 — insets/theme/scale:** winit ships `Window::safe_area` (0.31,
  egui-winit's own iOS module names it as its exit plan) **and** egui-winit
  declines to consume it for Android, or it cannot express the
  IME-inclusion rule (bottom = max(bars, ime) under enforced edge-to-edge).
  The JNI block then has no exit and is permanent.
- **T2 — lifecycle:** eframe declines lifecycle hooks after an upstream
  issue, or a second between-frame event kind joins backgrounding (today:
  one save hook, one pointer, one SAFETY argument — at two-plus, the
  pointer machinery is a de-facto fork already).
- **T3 — IME/packaging:** a real app needs CJK (Q4 taken) **and** no
  upstream packaging route exists — then either gradle delegation becomes
  the product (xbuild's `android.gradle` mode, the probe route recorded in
  addendum #3) or the fork grows the mini-AGP. Either way "thin layer" is
  no longer the honest description; re-run Q1.
- **T4 — the count:** the inventory above doubles from its nine-item
  2026-10-13 baseline without the corresponding upstream exits landing.
  Which holes doubled does not matter; the layer's premise was
  enumerability.
- **T5 — semantics break:** an eframe/winit change breaks
  `raw_input_hook`, `run_native`-returns (the exit path), or
  `NativeOptions` construction as this layer uses them, with no
  replacement. The seam and the exit both hang off these; that is an
  immediate trigger, not a drift one.

## The upstream push list

Generic holes worth upstream PRs/issues, in value order (none filed yet —
deliberate: external feedback loops, and each needs a standalone minimal
repro this repo's modules are not):

1. egui-winit: consume winit 0.31's `Window::safe_area` on Android (kills
   the JNI half of hole 5 if the IME rule fits).
2. eframe: lifecycle hooks — pause/resume/low-memory/backgrounding
   callbacks (kills hole 2 wholesale; the plan's services matrix has had
   the row open since the pivot).
3. winit/egui-winit: OS theme + text scale on mobile (hole 6; font scale
   riding `zoom_factor` is a workaround, documented as such in `android.rs`).
4. eframe: creation-time font hook (hole 1) and a mobile wgpu-limits
   preset (hole 7) — small, mechanical asks.

Filing these is framework work with this repo's modules as the repro; do
it when Q5 settles where the framework lives, so the issues carry the
final crate names.
