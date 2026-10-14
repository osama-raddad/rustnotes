---
id: 14
title: ui-kit: voice lint moves to a foundation module
tag: human-in-loop
priority: low
blocked-by: []
blocks: []
estimate: small
modules:
  - crates/ui-kit/src/voice.rs
  - crates/ui-kit/src/gallery_copy.rs
---

# ui-kit: voice lint moves to a foundation module

From the architecture audit (candidate 2). The brand-voice lint is kit-grade
(em dash, ellipsis allowlist, exclamation, emoji ranges, forbidden vocabulary,
title-case heuristic, canary-tested) but lives inside `gallery_copy`, seeded
from `brand_showcase`. Hosts shipping Vynatix-branded copy should not import
gallery guts to run voice checks on their own strings.
