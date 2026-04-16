use clap::Parser;

/// Rectangle-like window management for Hyprland
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Target a specific monitor by name instead of the active one
    #[arg(long, global = true, value_name = "NAME")]
    pub monitor: Option<String>,
}

#[derive(Parser, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Snap window to left half
    Left,
    /// Snap window to right half
    Right,
    /// Snap window to top half
    Up,
    /// Snap window to bottom half
    Down,
    /// Snap window to top-left quarter
    TopLeft,
    /// Snap window to top-right quarter
    TopRight,
    /// Snap window to bottom-left quarter
    BottomLeft,
    /// Snap window to bottom-right quarter
    BottomRight,
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
    /// Restore previous geometry of the active window
    Restore,
}
