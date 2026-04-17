# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release              # Build release binary at target/release/hypr-rectangle
cargo run -- <subcommand>          # Run locally (e.g. `cargo run -- left`)
cargo test                         # Run unit tests for pure functions
cargo clippy -- -D warnings        # Lint (CI gate)
cargo fmt                          # Format
nix build                          # Build via flake -> ./result/bin/hypr-rectangle
nix develop                        # Enter dev shell (cargo, rustc, clippy, rustfmt, rust-analyzer)
```

Unit tests cover the pure functions only (`gaps::Gaps::parse`, `dims::calculate_dimensions`, `area::classify_layer`, `state::State`). End-to-end verification (actual window snapping) requires a running Hyprland session and is manual.

## Architecture

Each invocation is a one-shot pipeline in `src/main.rs::main`:

1. **Parse CLI** (`cli.rs`) — subcommand + optional `--monitor <name>` global flag.
2. **Read gaps** (`gaps.rs`) — shells out to `hyprctl getoption general:gaps_{out,in} -j` because hyprland-rs 0.4-beta.3 does not expose the `custom` JSON field that Hyprland uses for gap vectors (see hyprwm/Hyprland#4974, closed not-planned). `Gaps::parse` follows CSS 1/2/3/4-value shorthand; garbage input falls back to zero with a stderr warning.
3. **Resolve monitor** (`area.rs::resolve_monitor`) — active monitor, or named monitor for `--monitor`.
4. **Compute usable area** (`area.rs`) — iterates `Layers` via hyprland-rs IPC and classifies each layer with the pure `classify_layer` helper (`EDGE_TOLERANCE_PX = 100`). Fullscreen layers (e.g. hyprpaper backgrounds) are skipped. Outer gaps are subtracted afterward.
5. **Compute tile dimensions** (`dims.rs::calculate_dimensions`) — halves/thirds/two-thirds, accounting for inner gaps. For asymmetric inner gaps it uses `.max()` of the horizontal/vertical edges so adjacent tiles never overlap.
6. **Snapshot geometry** (`state.rs`) — before dispatching a snap, the active window's current `{x, y, w, h}` is persisted keyed by its address to `$XDG_CACHE_HOME/hypr-rectangle/state.json`. Ring buffer capped at 50 entries. `Restore` reads and re-applies this geometry.
7. **Dispatch** (`dispatch.rs`) — issues `MoveActive` (or `MoveWindowPixel` for restore), sleeps `MOVE_RESIZE_DELAY = 10ms`, then issues the resize. The delay is load-bearing: hyprland-rs 0.4-beta.3 has no atomic `MoveResize` dispatch, and without the sleep the compositor races the two ops.

### Module layout

```
src/
  main.rs       orchestration + restore path
  cli.rs        Cli / Command (all snap variants + Restore)
  gaps.rs       Gaps, Gaps::parse, get_gaps (shells out to hyprctl)
  area.rs       UsableArea, Rect, classify_layer, resolve_monitor, calculate_usable_area
  dims.rs       Dimensions, calculate_dimensions
  dispatch.rs   MOVE_RESIZE_DELAY, CENTER_SIZE_PERCENT, dispatch_active, dispatch_by_address, execute
  state.rs      Geometry, State (load/save/record/take)
```

## Things to know when editing

- The `hyprland` crate is pinned to `0.4.0-beta.3`; there is no stable 0.4 upstream. Any upgrade will likely break `Layers`/`Monitor`/`Dispatch` imports. Before changing the version, recheck whether the `custom` JSON field has first-class support — if so, the `hyprctl` shell-out in `gaps.rs` can be removed.
- `Monitor.reserved: (u16, u16, u16, u16)` is available in hyprland-rs and exposes Hyprland's own computed reserved edges. We deliberately do **not** use it, because it only reflects layers with an explicit exclusive zone; the layer-iteration heuristic in `area.rs` also catches non-exclusive bars. If a future rewrite wants to simplify, this tradeoff is the thing to evaluate.
- Dispatch coordinates/sizes are cast `i32 → i16` (hyprland-rs API constraint). Very large multi-monitor offsets or 8K+ resolutions can overflow; errors carry a hint pointing at this.
- Inner-gap semantics: for asymmetric gaps (e.g. `gaps_in = 5 20 5 20`), `calculate_dimensions` uses `max(left, right)` as the horizontal gap and `max(top, bottom)` as the vertical. Overshoot is preferred over overlap.
- Dispatch order is **resize then move** (not move then resize). Hyprland ≥ 0.54 resizes a floating window around its current *center*, not from its top-left corner. Doing `MoveActive` first would put the window at the right spot, then `ResizeActive` would shift it back toward the center and the first snap would land wrong; doing resize first lets the center-pull happen, then move places the final top-left exactly. This was the root cause of the "first click wrong, second click correct" regression after upgrading to Hyprland 0.54.
- `cargo clippy -- -D warnings` is expected to pass at all times.
