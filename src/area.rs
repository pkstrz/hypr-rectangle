use anyhow::{Context, Result};
use hyprland::data::{LayerClient, Layers, Monitor, Monitors};
use hyprland::shared::{HyprData, HyprDataActive};

use crate::gaps::Gaps;

/// Tolerance (in pixels) for classifying a layer surface as edge-hugging.
/// Covers the case where panels are positioned a few pixels off the edge
/// or span almost (but not quite) the full monitor dimension.
const EDGE_TOLERANCE_PX: i32 = 100;

/// Usable area on a monitor after reserving layer surfaces and outer gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsableArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Rectangle in screen coordinates. Used by the pure classifier below.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Which edge a panel hugs. `Fullscreen` means the layer covers the whole
/// monitor (likely a background, e.g. hyprpaper) and must be ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeReservation {
    Top(i32),
    Bottom(i32),
    Left(i32),
    Right(i32),
    Fullscreen,
    None,
}

/// Classify a layer surface against its monitor's rectangle. Pure function
/// so it can be tested without a running Hyprland.
pub fn classify_layer(mon: Rect, layer: Rect) -> EdgeReservation {
    if layer.w >= mon.w - 1 && layer.h >= mon.h - 1 {
        return EdgeReservation::Fullscreen;
    }

    if layer.w >= mon.w - EDGE_TOLERANCE_PX {
        let top_dist = (layer.y - mon.y).max(0);
        let bottom_dist = ((mon.y + mon.h) - (layer.y + layer.h)).max(0);
        if top_dist <= EDGE_TOLERANCE_PX {
            return EdgeReservation::Top(top_dist + layer.h);
        }
        if bottom_dist <= EDGE_TOLERANCE_PX {
            return EdgeReservation::Bottom(bottom_dist + layer.h);
        }
        return EdgeReservation::None;
    }

    if layer.h >= mon.h - EDGE_TOLERANCE_PX {
        let left_dist = (layer.x - mon.x).max(0);
        let right_dist = ((mon.x + mon.w) - (layer.x + layer.w)).max(0);
        if left_dist <= EDGE_TOLERANCE_PX {
            return EdgeReservation::Left(left_dist + layer.w);
        }
        if right_dist <= EDGE_TOLERANCE_PX {
            return EdgeReservation::Right(right_dist + layer.w);
        }
    }

    EdgeReservation::None
}

/// Resolve the target monitor: the one whose `name` matches `requested`,
/// or the active monitor when `requested` is `None`.
pub fn resolve_monitor(requested: Option<&str>) -> Result<Monitor> {
    match requested {
        None => Monitor::get_active().context("Failed to get active monitor"),
        Some(name) => {
            let monitors = Monitors::get().context("Failed to get monitors")?;
            monitors
                .into_iter()
                .find(|m| m.name == name)
                .with_context(|| format!("Monitor {:?} not found", name))
        }
    }
}

/// Compute the usable area on `monitor`, subtracting edge-hugging layer
/// surfaces and the requested outer gaps.
pub fn calculate_usable_area(monitor: &Monitor, outer_gaps: &Gaps) -> Result<UsableArea> {
    let mon_rect = Rect {
        x: monitor.x,
        y: monitor.y,
        w: monitor.width as i32,
        h: monitor.height as i32,
    };

    let mut top = 0_i32;
    let mut bottom = 0_i32;
    let mut left = 0_i32;
    let mut right = 0_i32;

    let layers = Layers::get().context("Failed to get layers")?;
    if let Some((_, layer_display)) = layers
        .iter()
        .find(|(name, _)| name.as_str() == monitor.name)
    {
        for (_level, layer_list) in layer_display.iter() {
            for layer in layer_list {
                let layer_rect = layer_to_rect(layer);
                match classify_layer(mon_rect, layer_rect) {
                    EdgeReservation::Top(o) => top = top.max(o),
                    EdgeReservation::Bottom(o) => bottom = bottom.max(o),
                    EdgeReservation::Left(o) => left = left.max(o),
                    EdgeReservation::Right(o) => right = right.max(o),
                    EdgeReservation::Fullscreen | EdgeReservation::None => {}
                }
            }
        }
    }

    let mut usable_width = mon_rect.w - left - right;
    let mut usable_height = mon_rect.h - top - bottom;
    let mut offset_x = mon_rect.x + left;
    let mut offset_y = mon_rect.y + top;

    if usable_width > outer_gaps.left + outer_gaps.right {
        offset_x += outer_gaps.left;
        usable_width -= outer_gaps.left + outer_gaps.right;
    }
    if usable_height > outer_gaps.top + outer_gaps.bottom {
        offset_y += outer_gaps.top;
        usable_height -= outer_gaps.top + outer_gaps.bottom;
    }

    Ok(UsableArea {
        x: offset_x,
        y: offset_y,
        width: usable_width.max(1),
        height: usable_height.max(1),
    })
}

fn layer_to_rect(layer: &LayerClient) -> Rect {
    Rect {
        x: layer.x,
        y: layer.y,
        w: layer.w as i32,
        h: layer.h as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: Rect = Rect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    };

    #[test]
    fn fullscreen_layer_is_ignored() {
        let layer = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        assert_eq!(classify_layer(MON, layer), EdgeReservation::Fullscreen);
    }

    #[test]
    fn top_bar_reserves_top() {
        let waybar = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 30,
        };
        assert_eq!(classify_layer(MON, waybar), EdgeReservation::Top(30));
    }

    #[test]
    fn top_bar_with_small_offset_still_counts_as_top() {
        let waybar = Rect {
            x: 0,
            y: 4,
            w: 1920,
            h: 30,
        };
        assert_eq!(classify_layer(MON, waybar), EdgeReservation::Top(34));
    }

    #[test]
    fn bottom_bar_reserves_bottom() {
        let dock = Rect {
            x: 0,
            y: 1040,
            w: 1920,
            h: 40,
        };
        assert_eq!(classify_layer(MON, dock), EdgeReservation::Bottom(40));
    }

    #[test]
    fn left_vertical_bar_reserves_left() {
        let bar = Rect {
            x: 0,
            y: 0,
            w: 40,
            h: 1080,
        };
        assert_eq!(classify_layer(MON, bar), EdgeReservation::Left(40));
    }

    #[test]
    fn floating_widget_reserves_nothing() {
        let notification = Rect {
            x: 500,
            y: 500,
            w: 300,
            h: 100,
        };
        assert_eq!(classify_layer(MON, notification), EdgeReservation::None);
    }

    #[test]
    fn monitor_with_offset_classifies_correctly() {
        let mon = Rect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 1440,
        };
        let waybar = Rect {
            x: 1920,
            y: 0,
            w: 2560,
            h: 36,
        };
        assert_eq!(classify_layer(mon, waybar), EdgeReservation::Top(36));
    }
}
