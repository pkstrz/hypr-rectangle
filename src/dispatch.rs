use anyhow::{Context, Result};
use hyprland::dispatch::{Dispatch, DispatchType, Position, WindowIdentifier};
use hyprland::shared::Address;
use std::process::Command as ProcessCommand;
use std::thread::sleep;
use std::time::Duration;

use crate::area::UsableArea;
use crate::cli::Command;
use crate::dims::calculate_dimensions;
use crate::gaps::Gaps;

/// Delay between ResizeActive and MoveActive dispatch calls.
/// hyprland-rs 0.4-beta.3 has no atomic move+resize; empirically 10 ms
/// is enough for the compositor to apply the first op before the second.
///
/// Order is resize-then-move because Hyprland ≥ 0.54 resizes a floating
/// window around its current center (not from its top-left corner). Doing
/// move first would put the window at the right spot, then resize would
/// shift it back toward the center; doing resize first lets the center-pull
/// happen, and then move places the final top-left corner exactly.
const MOVE_RESIZE_DELAY: Duration = Duration::from_millis(10);

/// Percentage of usable area used by the `center` command (Rectangle.app parity).
const CENTER_SIZE_PERCENT: i32 = 75;

/// Run a single dispatcher through `hyprctl dispatch <expr>`.
///
/// Hyprland ≥ 0.56 (Lua config) evaluates the argument as
/// `hl.dispatch(<expr>)`, so `expr` must be a Lua dispatcher expression such
/// as `hl.dsp.window.move({ x = 1, y = 2 })`. hyprland-rs 0.4-beta.3 only
/// knows the legacy `resizeactive exact W H` syntax, hence the direct call.
///
/// Success is `ok` on stdout; older Hyprland versions reject Lua expressions
/// with a non-`ok` message (sometimes with exit code 0), so we check both.
fn hyprctl_dispatch(expr: &str) -> Result<()> {
    let output = ProcessCommand::new("hyprctl")
        .args(["dispatch", expr])
        .output()
        .context("Failed to run hyprctl")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() && stdout.trim() == "ok" {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!(
        "hyprctl dispatch {:?} failed: {}{}",
        expr,
        stdout.trim(),
        stderr.trim()
    )
}

/// Lua selector argument for a window target: empty table field for the
/// active window, `window = 'address:0x…'` for a specific one.
fn lua_window_field(target: Option<&Address>) -> String {
    match target {
        None => String::new(),
        Some(addr) => format!(", window = '{}'", WindowIdentifier::Address(addr.clone())),
    }
}

fn lua_resize_expr(width: i32, height: i32, target: Option<&Address>) -> String {
    format!(
        "hl.dsp.window.resize({{ x = {}, y = {}{} }})",
        width,
        height,
        lua_window_field(target)
    )
}

fn lua_move_expr(x: i32, y: i32, target: Option<&Address>) -> String {
    format!(
        "hl.dsp.window.move({{ x = {}, y = {}{} }})",
        x,
        y,
        lua_window_field(target)
    )
}

/// Resize-then-move via the Lua dispatchers (Hyprland ≥ 0.56).
fn lua_place(x: i32, y: i32, width: i32, height: i32, target: Option<&Address>) -> Result<()> {
    hyprctl_dispatch(&lua_resize_expr(width, height, target))?;
    sleep(MOVE_RESIZE_DELAY);
    hyprctl_dispatch(&lua_move_expr(x, y, target))
}

/// Move and resize the active window to exact coordinates.
///
/// Tries the Lua dispatchers first (Hyprland ≥ 0.56) and falls back to the
/// legacy hyprland-rs dispatchers for older Hyprland versions.
pub fn dispatch_active(x: i32, y: i32, width: i32, height: i32) -> Result<()> {
    match lua_place(x, y, width, height, None) {
        Ok(()) => Ok(()),
        Err(lua_err) => legacy_dispatch_active(x, y, width, height).with_context(|| {
            format!("Lua dispatch failed ({lua_err:#}); legacy dispatch also failed")
        }),
    }
}

/// Legacy (Hyprland < 0.56) move+resize of the active window.
fn legacy_dispatch_active(x: i32, y: i32, width: i32, height: i32) -> Result<()> {
    let (x, y, w, h) = to_i16_tuple(x, y, width, height)?;

    Dispatch::call(DispatchType::ResizeActive(Position::Exact(w, h)))
        .context("Failed to resize window")?;
    sleep(MOVE_RESIZE_DELAY);
    Dispatch::call(DispatchType::MoveActive(Position::Exact(x, y)))
        .context("Failed to move window")?;

    Ok(())
}

/// Place a window so its VISIBLE frame occupies `(vx, vy, vw, vh)`. Hyprland
/// draws `general:border_size` outside the reported `at`/`size` rect, so to
/// make the visible frame match the intended rectangle we inset the reported
/// coords by `border` on each side.
fn dispatch_visible(vx: i32, vy: i32, vw: i32, vh: i32, border: i32) -> Result<()> {
    let at_x = vx + border;
    let at_y = vy + border;
    let size_w = (vw - 2 * border).max(1);
    let size_h = (vh - 2 * border).max(1);
    dispatch_active(at_x, at_y, size_w, size_h)
}

/// Move and resize a specific window (by address) to exact coordinates.
///
/// Tries the Lua dispatchers first (Hyprland ≥ 0.56) and falls back to the
/// legacy hyprland-rs dispatchers for older Hyprland versions.
pub fn dispatch_by_address(
    address: &Address,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<()> {
    match lua_place(x, y, width, height, Some(address)) {
        Ok(()) => Ok(()),
        Err(lua_err) => {
            legacy_dispatch_by_address(address, x, y, width, height).with_context(|| {
                format!("Lua dispatch failed ({lua_err:#}); legacy dispatch also failed")
            })
        }
    }
}

/// Legacy (Hyprland < 0.56) move+resize of a specific window.
fn legacy_dispatch_by_address(
    address: &Address,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<()> {
    let (x, y, w, h) = to_i16_tuple(x, y, width, height)?;
    let id = WindowIdentifier::Address(address.clone());

    Dispatch::call(DispatchType::ResizeWindowPixel(
        Position::Exact(w, h),
        id.clone(),
    ))
    .context("Failed to resize window")?;
    sleep(MOVE_RESIZE_DELAY);
    Dispatch::call(DispatchType::MoveWindowPixel(Position::Exact(x, y), id))
        .context("Failed to move window")?;

    Ok(())
}

fn to_i16_tuple(x: i32, y: i32, w: i32, h: i32) -> Result<(i16, i16, i16, i16)> {
    let x = i16_or_overflow(x, "x")?;
    let y = i16_or_overflow(y, "y")?;
    let w = i16_or_overflow(w, "width")?;
    let h = i16_or_overflow(h, "height")?;
    Ok((x, y, w, h))
}

fn i16_or_overflow(value: i32, what: &str) -> Result<i16> {
    value.try_into().with_context(|| {
        format!(
            "{} = {} is outside the i16 range required by hyprland-rs; \
             possible cause: very large monitor or multi-monitor offset",
            what, value
        )
    })
}

/// Execute a snap command against the given usable area. Does not cover
/// `Restore` — that path is handled separately by the state module.
pub fn execute(cmd: Command, area: &UsableArea, inner_gaps: &Gaps, border: i32) -> Result<()> {
    let d = calculate_dimensions(area, inner_gaps);

    match cmd {
        Command::Left => dispatch_visible(area.x, area.y, d.half_width, area.height, border),
        Command::Right => dispatch_visible(
            area.x + d.half_width + d.gap_h,
            area.y,
            d.half_width,
            area.height,
            border,
        ),
        Command::Up => dispatch_visible(area.x, area.y, area.width, d.half_height, border),
        Command::Down => dispatch_visible(
            area.x,
            area.y + d.half_height + d.gap_v,
            area.width,
            d.half_height,
            border,
        ),

        Command::TopLeft => dispatch_visible(area.x, area.y, d.half_width, d.half_height, border),
        Command::TopRight => dispatch_visible(
            area.x + d.half_width + d.gap_h,
            area.y,
            d.half_width,
            d.half_height,
            border,
        ),
        Command::BottomLeft => dispatch_visible(
            area.x,
            area.y + d.half_height + d.gap_v,
            d.half_width,
            d.half_height,
            border,
        ),
        Command::BottomRight => dispatch_visible(
            area.x + d.half_width + d.gap_h,
            area.y + d.half_height + d.gap_v,
            d.half_width,
            d.half_height,
            border,
        ),

        Command::LeftThird => dispatch_visible(area.x, area.y, d.third_width, area.height, border),
        Command::CenterThird => dispatch_visible(
            area.x + d.third_width + d.gap_h,
            area.y,
            d.third_width,
            area.height,
            border,
        ),
        Command::RightThird => dispatch_visible(
            area.x + d.third_width * 2 + d.gap_h * 2,
            area.y,
            d.third_width,
            area.height,
            border,
        ),
        Command::LeftTwoThird => {
            dispatch_visible(area.x, area.y, d.two_third_width, area.height, border)
        }
        Command::RightTwoThird => dispatch_visible(
            area.x + d.third_width + d.gap_h,
            area.y,
            d.two_third_width,
            area.height,
            border,
        ),

        Command::Maximize => dispatch_visible(area.x, area.y, area.width, area.height, border),
        Command::Center => {
            let vw = area.width * CENTER_SIZE_PERCENT / 100;
            let vh = area.height * CENTER_SIZE_PERCENT / 100;
            let vx = area.x + (area.width - vw) / 2;
            let vy = area.y + (area.height - vh) / 2;
            dispatch_visible(vx, vy, vw, vh, border)
        }

        Command::Restore => {
            // Restore is handled upstream in main(); reaching here is a bug.
            anyhow::bail!("Restore should not reach dispatch::execute")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_expr_for_active_window() {
        assert_eq!(
            lua_resize_expr(800, 600, None),
            "hl.dsp.window.resize({ x = 800, y = 600 })"
        );
    }

    #[test]
    fn move_expr_for_active_window() {
        assert_eq!(
            lua_move_expr(10, 20, None),
            "hl.dsp.window.move({ x = 10, y = 20 })"
        );
    }

    #[test]
    fn exprs_target_window_by_address() {
        let addr = Address::new("0x5da1c3c3ccd0");
        assert_eq!(
            lua_resize_expr(800, 600, Some(&addr)),
            "hl.dsp.window.resize({ x = 800, y = 600, window = 'address:0x5da1c3c3ccd0' })"
        );
        assert_eq!(
            lua_move_expr(10, 20, Some(&addr)),
            "hl.dsp.window.move({ x = 10, y = 20, window = 'address:0x5da1c3c3ccd0' })"
        );
    }

    #[test]
    fn negative_coordinates_are_emitted_verbatim() {
        assert_eq!(
            lua_move_expr(-1920, -5, None),
            "hl.dsp.window.move({ x = -1920, y = -5 })"
        );
    }
}
