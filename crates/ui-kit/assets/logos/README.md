# Logo sources

The V mark and wordmark artwork, byte for byte from the design
system's `assets/Logos/source/`. The kit never parses these at run
time: `tools/prepare_icons.py` triangulates them once into
`src/logo/paths.rs`, and the widget colours the triangles with role
tokens. The per-ink SVGs stay in the design system as reference.
