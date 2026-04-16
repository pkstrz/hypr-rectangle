use anyhow::{Context, Result};
use std::process::Command as ProcessCommand;

/// Gap values for all four edges (top, right, bottom, left).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Gaps {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Gaps {
    /// Parse gap values from Hyprland's format. Follows CSS shorthand:
    /// - 1 value: all four edges
    /// - 2 values: vertical, horizontal
    /// - 3 values: top, horizontal, bottom
    /// - 4 values: top, right, bottom, left
    ///
    /// Unparseable input falls back to zero gaps with a warning on stderr.
    pub fn parse(value: &str) -> Self {
        let parts: Vec<i32> = value
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        match parts.len() {
            1 => Self {
                top: parts[0],
                right: parts[0],
                bottom: parts[0],
                left: parts[0],
            },
            2 => Self {
                top: parts[0],
                right: parts[1],
                bottom: parts[0],
                left: parts[1],
            },
            3 => Self {
                top: parts[0],
                right: parts[1],
                bottom: parts[2],
                left: parts[1],
            },
            4 => Self {
                top: parts[0],
                right: parts[1],
                bottom: parts[2],
                left: parts[3],
            },
            _ => {
                if !value.trim().is_empty() {
                    eprintln!(
                        "hypr-rectangle: cannot parse gap value {:?}; treating as 0",
                        value
                    );
                }
                Self::default()
            }
        }
    }
}

/// Fetch a Hyprland option via `hyprctl getoption -j`.
///
/// Needed because `hyprctl getoption general:gaps_*` returns values in the
/// `custom` field of the JSON, which hyprland-rs 0.4-beta.3 does not expose
/// through its typed config API (see hyprwm/Hyprland#4974).
fn get_hyprctl_option(option: &str) -> Result<String> {
    let output = ProcessCommand::new("hyprctl")
        .args(["getoption", option, "-j"])
        .output()
        .context("Failed to run hyprctl")?;

    if !output.status.success() {
        anyhow::bail!("hyprctl getoption {} failed", option);
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&json_str).context("Failed to parse hyprctl JSON output")?;

    // Order matters: `custom` holds vector values (gap TRBL), typed fields hold scalars.
    if let Some(custom) = json.get("custom").and_then(|v| v.as_str()) {
        return Ok(custom.to_string());
    }
    if let Some(int_val) = json.get("int").and_then(|v| v.as_i64()) {
        return Ok(int_val.to_string());
    }
    if let Some(float_val) = json.get("float").and_then(|v| v.as_f64()) {
        return Ok(float_val.to_string());
    }
    if let Some(str_val) = json.get("str").and_then(|v| v.as_str()) {
        return Ok(str_val.to_string());
    }

    Ok("0".to_string())
}

/// Read outer and inner gap settings from Hyprland.
pub fn get_gaps() -> Result<(Gaps, Gaps)> {
    let outer_raw = get_hyprctl_option("general:gaps_out").context("Failed to get gaps_out")?;
    let inner_raw = get_hyprctl_option("general:gaps_in").context("Failed to get gaps_in")?;
    Ok((Gaps::parse(&outer_raw), Gaps::parse(&inner_raw)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        assert_eq!(Gaps::parse(""), Gaps::default());
    }

    #[test]
    fn parse_single_value() {
        assert_eq!(
            Gaps::parse("10"),
            Gaps {
                top: 10,
                right: 10,
                bottom: 10,
                left: 10,
            }
        );
    }

    #[test]
    fn parse_two_values_vertical_horizontal() {
        assert_eq!(
            Gaps::parse("5 20"),
            Gaps {
                top: 5,
                right: 20,
                bottom: 5,
                left: 20,
            }
        );
    }

    #[test]
    fn parse_three_values_top_horiz_bottom() {
        assert_eq!(
            Gaps::parse("5 20 8"),
            Gaps {
                top: 5,
                right: 20,
                bottom: 8,
                left: 20,
            }
        );
    }

    #[test]
    fn parse_four_values_trbl() {
        assert_eq!(
            Gaps::parse("1 2 3 4"),
            Gaps {
                top: 1,
                right: 2,
                bottom: 3,
                left: 4,
            }
        );
    }

    #[test]
    fn parse_five_values_falls_back_to_zero() {
        assert_eq!(Gaps::parse("1 2 3 4 5"), Gaps::default());
    }

    #[test]
    fn parse_garbage_falls_back_to_zero() {
        assert_eq!(Gaps::parse("abc"), Gaps::default());
    }
}
