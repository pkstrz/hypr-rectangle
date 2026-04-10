//! hypr-rectangle - Rectangle-like window management for Hyprland
//!
//! This tool provides macOS Rectangle-style window snapping for Hyprland compositor.
//! It reads monitor geometry, layer surfaces (like Waybar), and gap settings
//! to calculate precise window positions.

// ============================================================================
// RUST BASICS: USE STATEMENTS
// ============================================================================
// `use` imports items from other modules/crates into scope.
// `::` is the path separator (like `/` in file paths).
// `{ A, B }` imports multiple items from the same path.

use anyhow::{Context, Result};
// `anyhow` provides ergonomic error handling:
// - `Result<T>` is shorthand for `Result<T, anyhow::Error>`
// - `Context` trait adds `.context("message")` to add context to errors
// - Works with `?` operator for automatic error propagation

use clap::Parser;
// `clap` is the most popular CLI argument parser for Rust.
// The `Parser` derive macro automatically generates argument parsing code
// from struct/enum definitions.

use hyprland::data::{Layers, Monitor};
use hyprland::dispatch::{Dispatch, DispatchType, Position};
use hyprland::shared::{HyprData, HyprDataActive};
// `hyprland` crate provides native IPC communication with Hyprland.
// - `HyprData` trait: for fetching collections (all monitors, all windows)
// - `HyprDataActive` trait: for fetching currently active items

use std::process::Command as ProcessCommand;
use std::thread::sleep;
use std::time::Duration;
// Standard library imports for process execution and delays.

// ============================================================================
// CLI ARGUMENT PARSING WITH CLAP
// ============================================================================

/// Rectangle-like window management for Hyprland
// The `///` is a doc comment - it becomes part of the generated --help text.

#[derive(Parser)]
// `#[derive(...)]` is an attribute that auto-generates trait implementations.
// `Parser` derive macro from clap generates all the CLI parsing logic.

// `#[command(...)]` configures the CLI application metadata.
// - `author`: reads from Cargo.toml
// - `version`: reads from Cargo.toml
// - `about`: uses the doc comment above
#[command(author, version, about, long_about = None)]
struct Cli {
    // In Rust, a `struct` is a custom data type that groups related values.
    // Fields are defined as `name: Type`.
    #[command(subcommand)]
    // This attribute tells clap that this field holds a subcommand enum.
    command: Command,
}

#[derive(Parser, Clone, Copy, Debug)]
// Multiple traits can be derived at once:
// - `Parser`: CLI parsing
// - `Clone`: allows `.clone()` to create copies
// - `Copy`: allows implicit copying (for simple types)
// - `Debug`: allows `{:?}` formatting for debugging

enum Command {
    // An `enum` in Rust is a type that can be one of several variants.
    // Unlike C enums, Rust enums can hold data (algebraic data types).
    /// Snap window to left half
    Left,
    /// Snap window to right half
    Right,
    /// Snap window to top half
    Up,
    /// Snap window to bottom half
    Down,
    /// Snap window to left third
    LeftThird,
    /// Snap window to center third
    CenterThird,
    /// Snap window to right third
    RightThird,
    /// Snap window to left two-thirds
    LeftTwoThird,
    /// Snap window to right two-thirds
    RightTwoThird,
    /// Maximize window (respecting gaps)
    Maximize,
    /// Center window at 75% size
    Center,
}

// ============================================================================
// GAP CONFIGURATION
// ============================================================================

/// Represents gap values for all four edges (top, right, bottom, left)
#[derive(Debug, Clone, Copy, Default)]
// `Default` trait allows creating a default instance with `Gaps::default()`.
struct Gaps {
    top: i32,
    right: i32,
    bottom: i32,
    left: i32,
}

impl Gaps {
    // `impl` block defines methods for a type.
    // Methods are functions associated with a type.

    /// Parse gap values from Hyprland's format.
    /// Hyprland can return gaps as:
    /// - Single value: "10" -> all sides equal
    /// - Four values: "10 20 10 20" -> top right bottom left
    fn parse(value: &str) -> Self {
        // `&str` is a string slice (borrowed reference to string data).
        // `Self` is an alias for the type we're implementing (Gaps).

        let parts: Vec<i32> = value
            // Method chaining: each method returns a value the next method uses.
            .split_whitespace()
            // `split_whitespace()` returns an iterator over words
            .filter_map(|s| s.parse().ok())
            // `filter_map` combines filter and map:
            // - `|s|` is a closure (anonymous function), `s` is the parameter
            // - `s.parse()` tries to parse string to i32, returns Result
            // - `.ok()` converts Result to Option (Some if Ok, None if Err)
            // - filter_map keeps only Some values and unwraps them
            .collect();
        // `collect()` consumes the iterator and creates a collection.
        // The type `Vec<i32>` is inferred from the annotation.

        match parts.len() {
            // `match` is Rust's powerful pattern matching (like switch on steroids).
            // It MUST be exhaustive - all cases must be handled.
            0 => Self::default(),
            // If no values parsed, return default (all zeros)
            1 => Self {
                top: parts[0],
                right: parts[0],
                bottom: parts[0],
                left: parts[0],
            },
            // Single value applies to all sides
            4 => Self {
                top: parts[0],
                right: parts[1],
                bottom: parts[2],
                left: parts[3],
            },
            // Four values: TRBL order (like CSS)
            _ => Self::default(),
            // `_` is a wildcard pattern matching anything else
        }
    }
}

/// Fetch a Hyprland option value using hyprctl
/// This is needed because hyprland-rs doesn't yet support the "custom" field
/// that Hyprland uses for gap settings.
fn get_hyprctl_option(option: &str) -> Result<String> {
    // `std::process::Command` runs external processes.
    // We use `ProcessCommand` to avoid name conflict with our CLI Command enum.
    let output = ProcessCommand::new("hyprctl")
        .args(["getoption", option, "-j"])
        // `.args()` takes an array/slice of arguments
        .output()
        // `.output()` runs the command and captures stdout/stderr
        .context("Failed to run hyprctl")?;

    if !output.status.success() {
        // `anyhow::bail!` is a macro that returns an error immediately
        anyhow::bail!("hyprctl getoption {} failed", option);
    }

    // Parse the JSON output to extract the value
    // `String::from_utf8_lossy` converts bytes to string, replacing invalid UTF-8
    let json_str = String::from_utf8_lossy(&output.stdout);

    // Use serde_json to parse the response
    // `serde_json::Value` is a dynamic JSON type (like Python dict/JavaScript object)
    let json: serde_json::Value =
        serde_json::from_str(&json_str).context("Failed to parse hyprctl JSON output")?;

    // Hyprland options can be in different fields depending on type:
    // - "custom" for vector values like gaps
    // - "int" for integer values
    // - "float" for float values
    // - "str" for string values
    // We try "custom" first (for gaps), then fall back to others

    // Try "custom" field first (used for gap vectors like "20 20 20 20")
    if let Some(custom) = json.get("custom").and_then(|v| v.as_str()) {
        return Ok(custom.to_string());
    }

    // Try "int" field (for integer options)
    if let Some(int_val) = json.get("int").and_then(|v| v.as_i64()) {
        return Ok(int_val.to_string());
    }

    // Try "float" field (for floating point options)
    if let Some(float_val) = json.get("float").and_then(|v| v.as_f64()) {
        return Ok(float_val.to_string());
    }

    // Try "str" field (for string options)
    if let Some(str_val) = json.get("str").and_then(|v| v.as_str()) {
        return Ok(str_val.to_string());
    }

    // Default to "0" if no value found
    Ok("0".to_string())
}

/// Read gap settings from Hyprland configuration
fn get_gaps() -> Result<(Gaps, Gaps)> {
    // Return type `Result<(Gaps, Gaps)>` means:
    // - On success: Ok((outer_gaps, inner_gaps)) - a tuple of two Gaps
    // - On failure: Err(anyhow::Error)

    // Read outer gaps (space between windows and screen edge)
    let outer_raw = get_hyprctl_option("general:gaps_out").context("Failed to get gaps_out")?;
    // `?` operator: if Result is Err, return early with that error.
    // `.context()` adds human-readable context to the error.

    // Read inner gaps (space between adjacent windows)
    let inner_raw = get_hyprctl_option("general:gaps_in").context("Failed to get gaps_in")?;

    // Parse the string values into our Gaps struct
    let outer = Gaps::parse(&outer_raw);
    let inner = Gaps::parse(&inner_raw);

    Ok((outer, inner))
    // `Ok(...)` wraps the success value in the Result type.
    // No semicolon = this is the return value (expression, not statement).
}

// ============================================================================
// MONITOR AND LAYER INFORMATION
// ============================================================================

/// Information about the usable area on a monitor
#[derive(Debug)]
struct UsableArea {
    /// X offset from absolute screen coordinates
    x: i32,
    /// Y offset from absolute screen coordinates
    y: i32,
    /// Usable width after subtracting reserved areas and gaps
    width: i32,
    /// Usable height after subtracting reserved areas and gaps
    height: i32,
}

/// Calculate usable area by accounting for layer surfaces (Waybar, etc.)
fn calculate_usable_area(outer_gaps: &Gaps) -> Result<UsableArea> {
    // `&Gaps` is a reference (borrow) - we read but don't take ownership.
    // This is Rust's core safety feature: the borrow checker ensures
    // references are always valid and prevents data races.

    const EDGE_TOLERANCE: i32 = 100;
    // `const` defines a compile-time constant. SCREAMING_CASE by convention.

    // Get the currently focused monitor
    let monitor = Monitor::get_active().context("Failed to get active monitor")?;

    let mon_width = monitor.width as i32;
    let mon_height = monitor.height as i32;
    let mon_x = monitor.x;
    let mon_y = monitor.y;
    // `as i32` is a type cast. Monitor dimensions are u16, we need i32
    // for calculations that might involve negative numbers.

    // Initialize offsets for reserved areas (layer surfaces like Waybar)
    let mut left_offset = 0;
    let mut right_offset = 0;
    let mut top_offset = 0;
    let mut bottom_offset = 0;
    // `mut` makes variables mutable. Rust variables are immutable by default!

    // Get layer surface information for all monitors
    let layers = Layers::get().context("Failed to get layers")?;

    // Find layers for our monitor
    // `Layers` is a newtype wrapper: `struct Layers(HashMap<String, LayerDisplay>)`
    // Access the inner HashMap with `.0` and then use `.get()` on it.
    if let Some(monitor_layers) = layers.iter().find(|(name, _)| *name == &monitor.name) {
        // `if let` is a pattern match that only handles one case.
        // `Some(x)` matches if the Option contains a value, binding it to `x`.
        // If it's `None`, the else branch runs (or nothing if no else).

        // Iterate through all layer levels (background, bottom, top, overlay)
        // `monitor_layers.1` is the `LayerDisplay`, which has a `levels` HashMap
        for (_level, layer_list) in monitor_layers.1.iter() {
            // `_level` prefix means we intentionally ignore this variable.
            // Rust warns about unused variables; underscore prefix suppresses this.

            for layer in layer_list {
                let lx = layer.x;
                let ly = layer.y;
                let lw = layer.w as i32;
                let lh = layer.h as i32;

                // Skip fullscreen background layers (like hyprpaper wallpaper)
                if lw >= mon_width - 1 && lh >= mon_height - 1 {
                    continue;
                    // `continue` skips to the next loop iteration
                }

                // Check if this is a horizontal bar (spans most of width)
                if lw >= mon_width - EDGE_TOLERANCE {
                    // Calculate distance from top and bottom edges
                    let top_dist = (ly - mon_y).max(0);
                    let bottom_dist = ((mon_y + mon_height) - (ly + lh)).max(0);
                    // `.max(0)` ensures we don't get negative distances

                    if top_dist <= EDGE_TOLERANCE {
                        // Layer is near the top edge
                        let offset = top_dist + lh;
                        top_offset = top_offset.max(offset);
                        // `.max()` returns the larger of two values
                    } else if bottom_dist <= EDGE_TOLERANCE {
                        // Layer is near the bottom edge
                        let offset = bottom_dist + lh;
                        bottom_offset = bottom_offset.max(offset);
                    }
                    continue;
                }

                // Check if this is a vertical bar (spans most of height)
                if lh >= mon_height - EDGE_TOLERANCE {
                    let left_dist = (lx - mon_x).max(0);
                    let right_dist = ((mon_x + mon_width) - (lx + lw)).max(0);

                    if left_dist <= EDGE_TOLERANCE {
                        let offset = left_dist + lw;
                        left_offset = left_offset.max(offset);
                    } else if right_dist <= EDGE_TOLERANCE {
                        let offset = right_dist + lw;
                        right_offset = right_offset.max(offset);
                    }
                }
            }
        }
    }

    // Calculate usable area after subtracting reserved space
    let mut usable_width = mon_width - left_offset - right_offset;
    let mut usable_height = mon_height - top_offset - bottom_offset;
    let mut offset_x = mon_x + left_offset;
    let mut offset_y = mon_y + top_offset;

    // Apply outer gaps (shrink usable area further)
    if usable_width > outer_gaps.left + outer_gaps.right {
        offset_x += outer_gaps.left;
        usable_width -= outer_gaps.left + outer_gaps.right;
    }

    if usable_height > outer_gaps.top + outer_gaps.bottom {
        offset_y += outer_gaps.top;
        usable_height -= outer_gaps.top + outer_gaps.bottom;
    }

    // Ensure minimum dimensions of 1 pixel
    usable_width = usable_width.max(1);
    usable_height = usable_height.max(1);

    Ok(UsableArea {
        x: offset_x,
        y: offset_y,
        width: usable_width,
        height: usable_height,
    })
    // Struct literal syntax: create instance with named fields
}

// ============================================================================
// WINDOW POSITIONING CALCULATIONS
// ============================================================================

/// Calculated dimensions for various window positions
struct Dimensions {
    /// Width for half-screen positions (accounting for inner gap)
    half_width: i32,
    /// Height for half-screen positions (accounting for inner gap)
    half_height: i32,
    /// Width for third-screen positions (accounting for inner gaps)
    third_width: i32,
    /// Width for two-thirds positions
    two_third_width: i32,
}

fn calculate_dimensions(area: &UsableArea, inner_gaps: &Gaps) -> Dimensions {
    // Use horizontal inner gap for width calculations
    let gap_h = inner_gaps.left;
    // Use vertical inner gap for height calculations
    let gap_v = inner_gaps.top;

    // Half dimensions: (total - gap) / 2
    // The gap goes between the two halves
    let half_width = if area.width > gap_h {
        (area.width - gap_h) / 2
    } else {
        area.width / 2
    }
    .max(1);
    // `.max(1)` chained after the if expression ensures minimum of 1

    let half_height = if area.height > gap_v {
        (area.height - gap_v) / 2
    } else {
        area.height / 2
    }
    .max(1);

    // Third dimensions: (total - 2*gap) / 3
    // Two gaps between three sections
    let double_gap = gap_h * 2;
    let third_width = if area.width > double_gap {
        (area.width - double_gap) / 3
    } else {
        area.width / 3
    }
    .max(1);

    // Two-thirds = 2 * third_width + gap
    let two_third_width = third_width * 2 + gap_h;

    Dimensions {
        half_width,
        half_height,
        third_width,
        two_third_width,
    }
    // Field shorthand: if variable name matches field name,
    // you can write `half_width` instead of `half_width: half_width`
}

// ============================================================================
// WINDOW DISPATCHING
// ============================================================================

/// Move and resize the active window to specified position and size
fn dispatch_window(x: i32, y: i32, width: i32, height: i32) -> Result<()> {
    // `Result<()>` means: returns Ok(()) on success, or an error.
    // `()` is the "unit type" - like void but it's an actual value.

    // Convert i32 to i16 for hyprland API
    // `try_into()` attempts conversion and returns Result
    // `context()` adds error message if conversion fails
    let x_i16: i16 = x.try_into().context("X coordinate out of i16 range")?;
    let y_i16: i16 = y.try_into().context("Y coordinate out of i16 range")?;
    let w_i16: i16 = width.try_into().context("Width out of i16 range")?;
    let h_i16: i16 = height.try_into().context("Height out of i16 range")?;

    // Move window to exact coordinates
    Dispatch::call(DispatchType::MoveActive(Position::Exact(x_i16, y_i16)))
        .context("Failed to move window")?;

    // Small delay between move and resize for window manager sync
    sleep(Duration::from_millis(10));

    // Resize window to exact dimensions
    Dispatch::call(DispatchType::ResizeActive(Position::Exact(w_i16, h_i16)))
        .context("Failed to resize window")?;

    Ok(())
    // Return success (unit value wrapped in Ok)
}

/// Execute the specified window management command
fn execute_command(cmd: Command, area: &UsableArea, inner_gaps: &Gaps) -> Result<()> {
    let dims = calculate_dimensions(area, inner_gaps);
    let gap_h = inner_gaps.left;
    let gap_v = inner_gaps.top;

    // Pattern match on the command to determine window position
    match cmd {
        Command::Left => {
            dispatch_window(area.x, area.y, dims.half_width, area.height)?;
        }

        Command::Right => {
            let x = area.x + dims.half_width + gap_h;
            dispatch_window(x, area.y, dims.half_width, area.height)?;
        }

        Command::Up => {
            dispatch_window(area.x, area.y, area.width, dims.half_height)?;
        }

        Command::Down => {
            let y = area.y + dims.half_height + gap_v;
            dispatch_window(area.x, y, area.width, dims.half_height)?;
        }

        Command::LeftThird => {
            dispatch_window(area.x, area.y, dims.third_width, area.height)?;
        }

        Command::CenterThird => {
            let x = area.x + dims.third_width + gap_h;
            dispatch_window(x, area.y, dims.third_width, area.height)?;
        }

        Command::RightThird => {
            let x = area.x + dims.third_width * 2 + gap_h * 2;
            dispatch_window(x, area.y, dims.third_width, area.height)?;
        }

        Command::LeftTwoThird => {
            dispatch_window(area.x, area.y, dims.two_third_width, area.height)?;
        }

        Command::RightTwoThird => {
            let x = area.x + dims.third_width + gap_h;
            dispatch_window(x, area.y, dims.two_third_width, area.height)?;
        }

        Command::Maximize => {
            dispatch_window(area.x, area.y, area.width, area.height)?;
        }

        Command::Center => {
            // Center at 75% of usable area
            let w = area.width * 75 / 100;
            let h = area.height * 75 / 100;
            let x = area.x + (area.width - w) / 2;
            let y = area.y + (area.height - h) / 2;
            dispatch_window(x, y, w, h)?;
        }
    }
    // Note: no semicolon after the match, and each arm ends with `?;`
    // The match returns () from each arm, which is our return value

    Ok(())
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

fn main() -> Result<()> {
    // `main` can return `Result` for automatic error printing.
    // If we return `Err`, Rust will print the error and exit with code 1.

    // Parse command line arguments
    let cli = Cli::parse();
    // `Cli::parse()` reads std::env::args() and parses into our struct.
    // If parsing fails (invalid args), clap prints help and exits.

    // Get gap configuration from Hyprland
    let (outer_gaps, inner_gaps) = get_gaps()?;
    // Destructuring: unpack the tuple into two variables.
    // `?` propagates any error from get_gaps().

    // Calculate usable area (accounting for Waybar, gaps, etc.)
    let area = calculate_usable_area(&outer_gaps)?;
    // `&outer_gaps` passes a reference (borrow) - we don't give up ownership.

    // Execute the requested command
    execute_command(cli.command, &area, &inner_gaps)?;

    Ok(())
}

// ============================================================================
// RUST CONCEPTS SUMMARY
// ============================================================================
//
// OWNERSHIP & BORROWING:
// - Each value has exactly one owner
// - When owner goes out of scope, value is dropped (freed)
// - `&T` is an immutable borrow - read access, original owner keeps it
// - `&mut T` is a mutable borrow - write access, exclusive access
// - Borrow checker ensures references are always valid
//
// OPTION & RESULT:
// - `Option<T>` = Some(value) or None - for optional values
// - `Result<T, E>` = Ok(value) or Err(error) - for operations that can fail
// - `?` operator: if Err, return early; if Ok, unwrap the value
//
// PATTERN MATCHING:
// - `match` must be exhaustive (handle all cases)
// - `if let` for single-case matching
// - `_` is wildcard (matches anything)
//
// TRAITS:
// - Like interfaces in other languages
// - `derive` macro auto-implements common traits
// - Common traits: Debug, Clone, Copy, Default, Parser
//
// ERROR HANDLING:
// - Use `Result` and `?` for recoverable errors
// - `anyhow` crate simplifies error handling with `.context()`
// - `panic!` for unrecoverable errors (bugs)
