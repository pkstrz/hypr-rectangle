use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_ENTRIES: usize = 50;
const STATE_FILENAME: &str = "state.json";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct State {
    /// Parallel vectors to preserve insertion order (oldest → newest).
    /// `serde_json` preserves JSON object order but a simple `Vec` of
    /// `(address, geometry)` is clearer and lets us cap the ring trivially.
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
    address: String,
    geometry: Geometry,
}

impl State {
    pub fn load() -> Self {
        state_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = state_path().context("Cannot determine state file path")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create state directory {}", parent.display())
            })?;
        }
        let json = serde_json::to_string(self).context("Failed to serialize state")?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write state file {}", path.display()))?;
        Ok(())
    }

    pub fn record(&mut self, address: &str, geometry: Geometry) {
        self.entries.retain(|e| e.address != address);
        self.entries.push(Entry {
            address: address.to_string(),
            geometry,
        });
        while self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
    }

    pub fn take(&mut self, address: &str) -> Option<Geometry> {
        let idx = self.entries.iter().position(|e| e.address == address)?;
        Some(self.entries.remove(idx).geometry)
    }
}

fn state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("hypr-rectangle").join(STATE_FILENAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(x: i32) -> Geometry {
        Geometry {
            x,
            y: 0,
            width: 100,
            height: 100,
        }
    }

    #[test]
    fn record_then_take_returns_geometry() {
        let mut s = State::default();
        s.record("0xdead", geom(1));
        assert_eq!(s.take("0xdead"), Some(geom(1)));
        assert_eq!(s.take("0xdead"), None);
    }

    #[test]
    fn record_twice_keeps_latest() {
        let mut s = State::default();
        s.record("0xdead", geom(1));
        s.record("0xdead", geom(2));
        assert_eq!(s.take("0xdead"), Some(geom(2)));
    }

    #[test]
    fn ring_buffer_drops_oldest() {
        let mut s = State::default();
        for i in 0..(MAX_ENTRIES as i32 + 5) {
            s.record(&format!("0x{}", i), geom(i));
        }
        assert_eq!(s.entries.len(), MAX_ENTRIES);
        assert_eq!(s.take("0x0"), None);
        assert_eq!(
            s.take(&format!("0x{}", MAX_ENTRIES as i32 + 4)),
            Some(geom(MAX_ENTRIES as i32 + 4))
        );
    }
}
