# hypr-rectangle

Rectangle-like window management for [Hyprland](https://hyprland.org/).

Snap windows to halves, thirds, or center with keyboard shortcuts - just like [Rectangle](https://rectangleapp.com/) on macOS.

## Features

- **Half snapping**: left, right, top, bottom
- **Third snapping**: left-third, center-third, right-third
- **Two-thirds**: left-two-third, right-two-third
- **Maximize**: fill usable area (respecting gaps)
- **Center**: 75% centered window

Automatically accounts for:
- Layer surfaces (Waybar, panels, docks)
- Outer gaps (between windows and screen edges)
- Inner gaps (between adjacent windows)
- Multi-monitor setups

## Installation

### Nix (recommended)

Run directly:
```bash
nix run github:pkstrz/hypr-rectangle -- left
```

Or add to your flake:
```nix
{
  inputs.hypr-rectangle.url = "github:pkstrz/hypr-rectangle";
}
```

Then use `inputs.hypr-rectangle.packages.${system}.default`.

### From source

```bash
git clone https://github.com/pkstrz/hypr-rectangle
cd hypr-rectangle
cargo build --release
# Binary at ./target/release/hypr-rectangle
```

## Usage

```bash
hypr-rectangle <COMMAND>

Commands:
  left             Snap window to left half
  right            Snap window to right half
  up               Snap window to top half
  down             Snap window to bottom half
  top-left         Snap window to top-left quarter
  top-right        Snap window to top-right quarter
  bottom-left      Snap window to bottom-left quarter
  bottom-right     Snap window to bottom-right quarter
  left-third       Snap window to left third
  center-third     Snap window to center third
  right-third      Snap window to right third
  left-two-third   Snap window to left two-thirds
  right-two-third  Snap window to right two-thirds
  maximize         Maximize window (respecting gaps)
  center           Center window at 75% size
  restore          Restore active window to its previous geometry
  help             Print this message

Options:
      --monitor <NAME>  Target a specific monitor by name instead of the active one
  -h, --help            Print help
  -V, --version         Print version
```

## Configuration

These bindings are **identical to Rectangle.app "Alternate" defaults on macOS**,
with modifiers translated for Linux: `⌃` → `CTRL`, `⌥` → `ALT`, `⌫` → `BackSpace`.

Add to your `~/.config/hypr/hyprland.conf`:

```conf
# Halves                                   # Rectangle default
bind = CTRL ALT, Left,      exec, hypr-rectangle left              # ⌃⌥ ←
bind = CTRL ALT, Right,     exec, hypr-rectangle right             # ⌃⌥ →
bind = CTRL ALT, Up,        exec, hypr-rectangle up                # ⌃⌥ ↑
bind = CTRL ALT, Down,      exec, hypr-rectangle down              # ⌃⌥ ↓

# Quarters
bind = CTRL ALT, U,         exec, hypr-rectangle top-left          # ⌃⌥ U
bind = CTRL ALT, I,         exec, hypr-rectangle top-right         # ⌃⌥ I
bind = CTRL ALT, J,         exec, hypr-rectangle bottom-left       # ⌃⌥ J
bind = CTRL ALT, K,         exec, hypr-rectangle bottom-right      # ⌃⌥ K

# Thirds
bind = CTRL ALT, D,         exec, hypr-rectangle left-third        # ⌃⌥ D
bind = CTRL ALT, F,         exec, hypr-rectangle center-third      # ⌃⌥ F
bind = CTRL ALT, G,         exec, hypr-rectangle right-third       # ⌃⌥ G

# Two-thirds
bind = CTRL ALT, E,         exec, hypr-rectangle left-two-third    # ⌃⌥ E
bind = CTRL ALT, T,         exec, hypr-rectangle right-two-third   # ⌃⌥ T

# Maximize, center, restore
bind = CTRL ALT, Return,    exec, hypr-rectangle maximize          # ⌃⌥ ⏎
bind = CTRL ALT, C,         exec, hypr-rectangle center            # ⌃⌥ C
bind = CTRL ALT, BackSpace, exec, hypr-rectangle restore           # ⌃⌥ ⌫
```

See [examples/hyprland.conf](examples/hyprland.conf) for the full binding file.

## Rectangle.app feature parity

Compared against [Rectangle.app](https://github.com/rxhanson/Rectangle) on macOS
(actions from `WindowAction.swift`). Default binds shown are Rectangle's
Alternate set, matched 1:1 above.

### Implemented

| Rectangle action | Default bind | `hypr-rectangle` command |
|---|---|---|
| Left Half | ⌃⌥ ← | `left` |
| Right Half | ⌃⌥ → | `right` |
| Top Half | ⌃⌥ ↑ | `up` |
| Bottom Half | ⌃⌥ ↓ | `down` |
| Top Left | ⌃⌥ U | `top-left` |
| Top Right | ⌃⌥ I | `top-right` |
| Bottom Left | ⌃⌥ J | `bottom-left` |
| Bottom Right | ⌃⌥ K | `bottom-right` |
| First Third | ⌃⌥ D | `left-third` |
| Center Third | ⌃⌥ F | `center-third` |
| Last Third | ⌃⌥ G | `right-third` |
| First Two Thirds | ⌃⌥ E | `left-two-third` |
| Last Two Thirds | ⌃⌥ T | `right-two-third` |
| Maximize | ⌃⌥ ⏎ | `maximize` |
| Center | ⌃⌥ C | `center` |
| Restore | ⌃⌥ ⌫ | `restore` |

Plus one feature Rectangle doesn't have: `--monitor <NAME>` to target a named
monitor instead of the active one.

### Not implemented (yet)

| Category | Rectangle actions |
|---|---|
| Resize steps | Larger / Smaller (both axes or per-axis), Double/Halve Width/Height |
| Fine movement | Move Left / Right / Up / Down (shift without resize) |
| Extra fits | Almost Maximize, Maximize Height, Center Prominently, Center Half, Center Two-Thirds |
| Fourths | First / Second / Third / Last Fourth, Three-Fourths variants |
| Sixths | All 6 corner/edge sixths |
| Ninths | 3×3 grid |
| Eighths / Twelfths / Sixteenths | Finer grid splits |
| Display moves | Next Display, Previous Display, Display 1..9 (direct) |
| Orientation-aware thirds | Vertical thirds/two-thirds, corner-thirds |
| Batch | Tile All, Cascade All, Tile / Cascade Active App |
| Drag | Snap areas (drag window to edge/corner to snap) |
| Custom | "Specified" (user-defined size), Reverse All |
| Cycling | Halves → Two-Thirds → Thirds on repeated press |

Snap areas require Hyprland-side support for drag tracking and are outside the
scope of a one-shot CLI. The rest are candidates for future commands.

## How it works

1. Reads Hyprland's gap settings (`general:gaps_out`, `general:gaps_in`) via `hyprctl getoption`
2. Resolves the target monitor (`--monitor <name>` or the active one)
3. Detects edge-hugging layer surfaces (Waybar, docks, etc.) to calculate reserved space
4. Computes the usable area after gaps and reserved space
5. Before moving, snapshots the window's current geometry to `$XDG_CACHE_HOME/hypr-rectangle/state.json` so `restore` can undo
6. Moves and resizes the active (or `--monitor`-targeted) window to the requested position

## Requirements

- Hyprland (tested with 0.40+)
- Rust 1.70+ (for building)

## License

MIT License - see [LICENSE](LICENSE)
