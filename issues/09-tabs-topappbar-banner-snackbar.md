---
id: 09
title: Components: Tabs, TopAppBar, Banner, Snackbar with gallery section
tag: AFK
priority: high
blocked-by: [05, 07, 08]
blocks: [11]
estimate: medium
modules:
  - crates/ui-kit/src/components/tabs.rs
  - crates/ui-kit/src/components/top_app_bar.rs
  - crates/ui-kit/src/components/banner.rs
  - crates/ui-kit/src/components/snackbar.rs
  - crates/ui-kit/examples/gallery.rs
---

# Components: Tabs, TopAppBar, Banner, Snackbar with gallery section

## What changes for the user

The gallery gains chrome: a working TopAppBar (logo, section links, search affordance,
one CTA) on Liquid Glass at the top of the window; a Tabs strip that actually switches
between gallery sub-panels (used by the section itself as its demo); Banner demos
(info/warning/error, dismissible) anchored at a panel top; and a Snackbar host that
shows brief operational messages ("Saved.") triggered by a gallery action button.

## Acceptance criteria

- Tabs per `components/Tabs/README.md`: peer panels, accent `indicator` underline on
  the active tab, focus rings, keyboard arrow navigation if egui sense allows,
  typed active-index state. Used for real in the gallery (panel switcher).
- TopAppBar per `components/TopAppBar/README.md`: glass chrome (08 module, chrome
  or regular thickness per spec), logo (03) + primary nav + search affordance +
  ONE call-to-action button (05); height/sticky behaviour per spec; on scroll it
  stays put (egui: paint in a top panel).
- Banner per `components/Banner/README.md`: leading icon, bold lead, message,
  optional dismiss; tones mapped to status roles (info/warning/error + neutral);
  focus ring on the dismiss control; dismissible with typed event.
- Snackbar per `components/Snackbar/README.md`: inverse surface near the bottom,
  optional title/line, at most one action + dismiss, success/error status tile
  variants; a small host/state-machine with timed auto-dismiss (spec duration;
  reduced-motion flag affects entrance animation only, never the auto-dismiss
  timing); "at most one visible" rule enforced by the host (unit test).
- Unit tests: snackbar machine (enqueue → show → timeout → gone; action click →
  event + dismiss; second enqueue while visible replaces/queues per spec), tabs
  index bounds + indicator geometry, banner dismiss event mapping, top app bar
  layout constants.
- Gallery copy voice-clean: snackbar messages operational ("Saved.", "Link copied."),
  banner copy four-part where error-toned.
- `just check/lint/test` green.

## Out of scope for this issue

- Real navigation routing; responsive collapse of the app bar (note as future work
  if the spec mentions mobile behaviour).

## Implementation notes

- The gallery grows its final shape here: top app bar + tabbed sections. Coordinate
  with the existing section registry so 10/11 slot in cleanly.
- egui top panel: `TopBottomPanel::top`; glass pane paints inside it.

## Definition of done

- [ ] All acceptance tests pass (red → green observed).
- [ ] `just check`, `just lint`, `just test` green.
- [ ] One commit, one issue. Commit message references this file by id.
- [ ] Issue file updated with `status: done` in frontmatter.
