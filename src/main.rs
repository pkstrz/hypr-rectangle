//! hypr-rectangle - Rectangle-like window management for Hyprland.

use anyhow::{Context, Result};
use clap::Parser;
use hyprland::data::Client;
use hyprland::shared::HyprDataActiveOptional;

mod area;
mod cli;
mod dims;
mod dispatch;
mod gaps;
mod state;

use crate::cli::{Cli, Command};
use crate::state::{Geometry, State};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (outer_gaps, inner_gaps) = gaps::get_gaps()?;
    let monitor = area::resolve_monitor(cli.monitor.as_deref())?;
    let area = area::calculate_usable_area(&monitor, &outer_gaps)?;

    if cli.command == Command::Restore {
        return restore_active_window();
    }

    snapshot_active_window()?;
    dispatch::execute(cli.command, &area, &inner_gaps)?;
    Ok(())
}

fn active_client() -> Result<Client> {
    Client::get_active()
        .context("Failed to get active window")?
        .context("No active window")
}

/// Record the active window's current geometry before a snap, so `restore`
/// can return it later. Failures to persist are logged but non-fatal — we
/// never want a state-write issue to block the snap itself.
fn snapshot_active_window() -> Result<()> {
    let client = active_client()?;
    let geometry = Geometry {
        x: client.at.0 as i32,
        y: client.at.1 as i32,
        width: client.size.0 as i32,
        height: client.size.1 as i32,
    };
    let mut state = State::load();
    state.record(&client.address.to_string(), geometry);
    if let Err(e) = state.save() {
        eprintln!("hypr-rectangle: failed to save restore state: {e:#}");
    }
    Ok(())
}

fn restore_active_window() -> Result<()> {
    let client = active_client()?;
    let addr = client.address.to_string();
    let mut state = State::load();
    let Some(geom) = state.take(&addr) else {
        anyhow::bail!("No saved geometry for window {}", addr);
    };
    dispatch::dispatch_by_address(&client.address, geom.x, geom.y, geom.width, geom.height)?;
    if let Err(e) = state.save() {
        eprintln!("hypr-rectangle: failed to update restore state: {e:#}");
    }
    Ok(())
}
