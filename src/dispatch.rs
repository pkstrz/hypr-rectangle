use anyhow::{Context, Result};
use hyprland::dispatch::{Dispatch, DispatchType, Position, WindowIdentifier};
use hyprland::shared::Address;
use std::thread::sleep;
use std::time::Duration;

use crate::area::UsableArea;
use crate::cli::Command;
use crate::dims::calculate_dimensions;
use crate::gaps::Gaps;

/// Delay between MoveActive and ResizeActive dispatch calls.
/// hyprland-rs 0.4-beta.3 has no atomic move+resize; empirically 10 ms
/// is enough for the compositor to apply the first op before the second.
const MOVE_RESIZE_DELAY: Duration = Duration::from_millis(10);

/// Percentage of usable area used by the `center` command (Rectangle.app parity).
const CENTER_SIZE_PERCENT: i32 = 75;

/// Move and resize the active window to exact coordinates.
pub fn dispatch_active(x: i32, y: i32, width: i32, height: i32) -> Result<()> {
    let (x, y, w, h) = to_i16_tuple(x, y, width, height)?;

    Dispatch::call(DispatchType::MoveActive(Position::Exact(x, y)))
        .context("Failed to move window")?;
    sleep(MOVE_RESIZE_DELAY);
    Dispatch::call(DispatchType::ResizeActive(Position::Exact(w, h)))
        .context("Failed to resize window")?;

    Ok(())
}

/// Move and resize a specific window (by address) to exact coordinates.
pub fn dispatch_by_address(
    address: &Address,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<()> {
    let (x, y, w, h) = to_i16_tuple(x, y, width, height)?;
    let id = WindowIdentifier::Address(address.clone());

    Dispatch::call(DispatchType::MoveWindowPixel(
        Position::Exact(x, y),
        id.clone(),
    ))
    .context("Failed to move window")?;
    sleep(MOVE_RESIZE_DELAY);
    Dispatch::call(DispatchType::ResizeWindowPixel(Position::Exact(w, h), id))
        .context("Failed to resize window")?;

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
pub fn execute(cmd: Command, area: &UsableArea, inner_gaps: &Gaps) -> Result<()> {
    let d = calculate_dimensions(area, inner_gaps);

    match cmd {
        Command::Left => dispatch_active(area.x, area.y, d.half_width, area.height),
        Command::Right => dispatch_active(
            area.x + d.half_width + d.gap_h,
            area.y,
            d.half_width,
            area.height,
        ),
        Command::Up => dispatch_active(area.x, area.y, area.width, d.half_height),
        Command::Down => dispatch_active(
            area.x,
            area.y + d.half_height + d.gap_v,
            area.width,
            d.half_height,
        ),

        Command::TopLeft => dispatch_active(area.x, area.y, d.half_width, d.half_height),
        Command::TopRight => dispatch_active(
            area.x + d.half_width + d.gap_h,
            area.y,
            d.half_width,
            d.half_height,
        ),
        Command::BottomLeft => dispatch_active(
            area.x,
            area.y + d.half_height + d.gap_v,
            d.half_width,
            d.half_height,
        ),
        Command::BottomRight => dispatch_active(
            area.x + d.half_width + d.gap_h,
            area.y + d.half_height + d.gap_v,
            d.half_width,
            d.half_height,
        ),

        Command::LeftThird => dispatch_active(area.x, area.y, d.third_width, area.height),
        Command::CenterThird => dispatch_active(
            area.x + d.third_width + d.gap_h,
            area.y,
            d.third_width,
            area.height,
        ),
        Command::RightThird => dispatch_active(
            area.x + d.third_width * 2 + d.gap_h * 2,
            area.y,
            d.third_width,
            area.height,
        ),
        Command::LeftTwoThird => dispatch_active(area.x, area.y, d.two_third_width, area.height),
        Command::RightTwoThird => dispatch_active(
            area.x + d.third_width + d.gap_h,
            area.y,
            d.two_third_width,
            area.height,
        ),

        Command::Maximize => dispatch_active(area.x, area.y, area.width, area.height),
        Command::Center => {
            let w = area.width * CENTER_SIZE_PERCENT / 100;
            let h = area.height * CENTER_SIZE_PERCENT / 100;
            let x = area.x + (area.width - w) / 2;
            let y = area.y + (area.height - h) / 2;
            dispatch_active(x, y, w, h)
        }

        Command::Restore => {
            // Restore is handled upstream in main(); reaching here is a bug.
            anyhow::bail!("Restore should not reach dispatch::execute")
        }
    }
}
