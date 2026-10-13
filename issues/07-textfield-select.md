---
id: 07
title: Components: TextField and Select with gallery section
tag: AFK
priority: high
blocked-by: [03, 04]
blocks: [09]
estimate: medium
status: done
completed: 2026-10-14T01:55:12+02:00
commit: d5cc48a955b8ba9e2c33e48b3442e8a1b66e6c1e
modules:
  - crates/ui-kit/src/components/text_field.rs
  - crates/ui-kit/src/components/select.rs
  - crates/ui-kit/examples/gallery.rs
---

# Components: TextField and Select with gallery section

## What changes for the user

A "Components: text inputs" gallery section: labelled text fields (filled style per
spec, hint line, leading icon where spec'd, error state with the four-part
reassuring message), a multiline variant if the spec has one, and a Select that
opens a popover list over the gallery and commits a choice. All states
(hover/focus/disabled/error) visible and focus-ring correct in every theme.

## Acceptance criteria

- TextField per `components/TextField/README.md`: always-visible label, box per
  spec (height, radius-md, outline colours from roles), optional leading icon
  (icon-size-md), hint below, error state using the error roles + a status icon +
  word, focus ring per 04, disabled 38/12. Wraps egui's text editing (`TextEdit`)
  restyled — do not reimplement text input; own the chrome around it.
- Select per `components/Select/README.md`: same box as text field at rest with a
  chevron icon, opens a popover (egui popup) with the option list, selected option
  marked per spec, keyboard navigable, commits on click/Enter, closes on Esc or
  outside click. Long lists scroll. Typed selection API.
- Unit tests: select state machine (open→hover→select→commit, Esc dismissal,
  outside-click dismissal where testable headlessly as pure logic), field/hint
  visibility rules, error-state colour resolution per theme.
- Gallery section: a small believable form (e.g. "Project details": name field
  with hint, email field that shows the error state when malformed, region select
  per the radio spec's example domain), voice-clean copy, error copy four-part
  (what, why, what next, help) per the voice guidelines.
- `just check/lint/test` green.

## Out of scope for this issue

- Form validation framework; multi-select; combobox-with-typing.

## Implementation notes

- egui `TextEdit` supports `hint_text`, `desired_width`, custom frame = false;
  style text colours via the theme's visuals.
- Popover: `egui::Popup` or `ui.menu_button`-style layer; keep chrome on-token.

## Definition of done

- [x] All acceptance tests pass (red → green observed).
- [x] `just check`, `just lint`, `just test` green.
- [x] One commit, one issue. Commit message references this file by id.
- [x] Issue file updated with `status: done` in frontmatter.
