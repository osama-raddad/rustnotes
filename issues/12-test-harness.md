---
id: 12
title: ui-kit: shared headless-paint test harness
tag: human-in-loop
priority: medium
blocked-by: []
blocks: []
estimate: small
modules:
  - crates/ui-kit/src/testutil.rs
---

# ui-kit: shared headless-paint test harness

From the architecture audit (`.afk/architecture-audit.md`, candidate 1). The kit's
real test surface — warm-up pass installing fonts, paint passes via `ctx.run_ui`,
`FullOutput::drop_without_applying_deltas()`, assert on public responses — is
copy-pasted across ~20 test modules (`install_faces` ×6, warm-ups ×14, string-keyed
`state(flag)` ×7, 76 `run_ui` sites). Name it as one `#[cfg(test)]` module.

Design note for the grilling pass: the harness should own the pass structure, not
hide it — tests must still be able to measure previous-frame vs current-frame
behaviour explicitly.
