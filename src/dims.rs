use crate::area::UsableArea;
use crate::gaps::Gaps;

/// Tile dimensions in VISIBLE-frame space (border drawn outside these rects).
///
/// `gap_h` / `gap_v` are the visible space between adjacent tile frames.
/// Per Hyprland's model, `gaps_in` is applied per-tile-side, so the visible
/// gap between two neighbours is `2 * gaps_in`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub half_width: i32,
    pub half_height: i32,
    pub third_width: i32,
    pub two_third_width: i32,
    pub gap_h: i32,
    pub gap_v: i32,
}

/// Compute tile dimensions for the given visible-frame usable area.
///
/// For asymmetric inner gaps, the larger edge wins so adjacent tiles never
/// overlap — overshoot by a few pixels is less noticeable than overlap.
pub fn calculate_dimensions(area: &UsableArea, inner_gaps: &Gaps) -> Dimensions {
    let gap_h = inner_gaps.left.max(inner_gaps.right) * 2;
    let gap_v = inner_gaps.top.max(inner_gaps.bottom) * 2;

    let half_width = if area.width > gap_h {
        (area.width - gap_h) / 2
    } else {
        area.width / 2
    }
    .max(1);

    let half_height = if area.height > gap_v {
        (area.height - gap_v) / 2
    } else {
        area.height / 2
    }
    .max(1);

    let double_gap_h = gap_h * 2;
    let third_width = if area.width > double_gap_h {
        (area.width - double_gap_h) / 3
    } else {
        area.width / 3
    }
    .max(1);

    let two_third_width = third_width * 2 + gap_h;

    Dimensions {
        half_width,
        half_height,
        third_width,
        two_third_width,
        gap_h,
        gap_v,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(w: i32, h: i32) -> UsableArea {
        UsableArea {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    }

    fn sym_gap(v: i32) -> Gaps {
        Gaps {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    #[test]
    fn visible_gap_is_twice_gaps_in() {
        let d = calculate_dimensions(&area(1000, 800), &sym_gap(5));
        assert_eq!(d.gap_h, 10);
        assert_eq!(d.gap_v, 10);
    }

    #[test]
    fn halves_sum_plus_gap_equals_width() {
        let d = calculate_dimensions(&area(1000, 800), &sym_gap(10));
        assert_eq!(d.half_width * 2 + d.gap_h, 1000);
    }

    #[test]
    fn thirds_sum_plus_gaps_equals_width() {
        let d = calculate_dimensions(&area(999, 800), &sym_gap(3));
        assert_eq!(d.third_width * 3 + d.gap_h * 2, 999);
    }

    #[test]
    fn two_third_is_two_thirds_plus_gap() {
        let d = calculate_dimensions(&area(900, 600), &sym_gap(5));
        assert_eq!(d.two_third_width, d.third_width * 2 + d.gap_h);
    }

    #[test]
    fn gap_larger_than_area_falls_back_to_half() {
        let d = calculate_dimensions(&area(10, 10), &sym_gap(20));
        assert_eq!(d.half_width, 5);
        assert_eq!(d.half_height, 5);
    }

    #[test]
    fn asymmetric_inner_gap_uses_max() {
        let gaps = Gaps {
            top: 2,
            right: 20,
            bottom: 2,
            left: 5,
        };
        let d = calculate_dimensions(&area(1000, 800), &gaps);
        assert_eq!(d.gap_h, 40);
        assert_eq!(d.gap_v, 4);
    }

    #[test]
    fn zero_gap_halves_area_exactly() {
        let d = calculate_dimensions(&area(1000, 800), &sym_gap(0));
        assert_eq!(d.half_width, 500);
        assert_eq!(d.half_height, 400);
    }

    #[test]
    fn minimum_one_pixel_dimensions() {
        let d = calculate_dimensions(&area(1, 1), &sym_gap(50));
        assert!(d.half_width >= 1);
        assert!(d.half_height >= 1);
        assert!(d.third_width >= 1);
    }
}
