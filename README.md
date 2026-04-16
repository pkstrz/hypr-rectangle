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

Add keybindings to your `~/.config/hypr/hyprland.conf`:

```conf
# Rectangle-like window management
# Halves
bind = SUPER CTRL, Left, exec, hypr-rectangle left
bind = SUPER CTRL, Right, exec, hypr-rectangle right
bind = SUPER CTRL, Up, exec, hypr-rectangle up
bind = SUPER CTRL, Down, exec, hypr-rectangle down

# Thirds
bind = SUPER CTRL, D, exec, hypr-rectangle left-third
bind = SUPER CTRL, F, exec, hypr-rectangle center-third
bind = SUPER CTRL, G, exec, hypr-rectangle right-third

# Two-thirds
bind = SUPER CTRL, E, exec, hypr-rectangle left-two-third
bind = SUPER CTRL, T, exec, hypr-rectangle right-two-third

# Quarters
bind = SUPER CTRL, Y, exec, hypr-rectangle top-left
bind = SUPER CTRL, U, exec, hypr-rectangle top-right
bind = SUPER CTRL, B, exec, hypr-rectangle bottom-left
bind = SUPER CTRL, N, exec, hypr-rectangle bottom-right

# Maximize, center, restore
bind = SUPER CTRL, Return, exec, hypr-rectangle maximize
bind = SUPER CTRL, C, exec, hypr-rectangle center
bind = SUPER CTRL, Z, exec, hypr-rectangle restore
```

See [examples/hyprland.conf](examples/hyprland.conf) for more keybinding options.

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
