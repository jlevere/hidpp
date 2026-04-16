use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const SAMPLE_CONFIG: &str = r#"# hidppd config — maps diverted buttons and gestures to actions.
#
# Button CIDs for MX Master 3S:
#   82  = Middle Click
#   83  = Back
#   86  = Forward
#   195 = Gesture Button (thumb)
#   196 = Mode Shift (scroll wheel click)
#
# Keystroke format: "modifier+modifier+key"
#   Modifiers: ctrl, alt, shift, cmd (or meta/super/win)
#   Keys: a-z, 0-9, f1-f20, left/right/up/down, tab, return,
#         space, escape, home, end, pageup, pagedown, delete,
#         playpause, next, prev, volumeup, volumedown, mute
#
# Simple button → keystroke:
#   [buttons]
#   83 = "alt+left"          # Back button → browser back
#   86 = "alt+right"         # Forward → browser forward
#
# Gesture button — hold + swipe for directional actions:
#   [gestures.195]
#   up = "ctrl+up"           # Swipe up → Mission Control
#   down = "ctrl+down"       # Swipe down → App Exposé
#   left = "ctrl+left"       # Swipe left → prev desktop
#   right = "ctrl+right"     # Swipe right → next desktop
#   tap = "playpause"        # Quick tap → play/pause
#   threshold = 50           # Min displacement (default: 50)
#
# Command actions:
#   83 = { type = "command", run = "open -a Safari" }

[buttons]
83 = "alt+left"
86 = "alt+right"

[gestures.195]
up = "ctrl+up"
down = "ctrl+down"
left = "ctrl+left"
right = "ctrl+right"
"#;

/// Raw config as deserialized from TOML (string keys).
#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    buttons: HashMap<String, Action>,
    #[serde(default)]
    gestures: HashMap<String, GestureConfig>,
}

/// Top-level daemon config with parsed CID keys.
#[derive(Debug, Default)]
pub struct Config {
    /// Map of CID → action for simple button diversion.
    pub buttons: HashMap<u16, Action>,
    /// Map of CID → gesture config for gesture buttons.
    pub gestures: HashMap<u16, GestureConfig>,
}

impl Config {
    /// All CIDs that need to be diverted (buttons + gestures).
    pub fn all_diverted_cids(&self) -> impl Iterator<Item = u16> + '_ {
        self.buttons
            .keys()
            .copied()
            .chain(self.gestures.keys().copied())
    }

    /// Whether this CID is a gesture button (needs rawXY).
    pub fn is_gesture_cid(&self, cid: u16) -> bool {
        self.gestures.contains_key(&cid)
    }
}

/// Gesture configuration for a single button.
#[derive(Debug, Deserialize, Clone)]
pub struct GestureConfig {
    pub up: Option<Action>,
    pub down: Option<Action>,
    pub left: Option<Action>,
    pub right: Option<Action>,
    pub tap: Option<Action>,
    /// Minimum accumulated displacement to trigger a directional gesture.
    #[serde(default = "default_threshold")]
    pub threshold: i32,
}

fn default_threshold() -> i32 {
    50
}

/// An action triggered by a diverted button or gesture.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Action {
    /// Shorthand: just a keystroke string like "ctrl+left".
    Keystroke(String),
    /// Explicit action with type.
    Explicit(ExplicitAction),
}

/// Explicit action definition.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ExplicitAction {
    /// Send a keystroke combination.
    #[serde(rename = "keystroke")]
    Keystroke { keys: String },
    /// Run a shell command.
    #[serde(rename = "command")]
    Command { run: String },
}

impl Action {
    /// Get the keystroke string if this is a keystroke action.
    pub fn keystroke(&self) -> Option<&str> {
        match self {
            Action::Keystroke(s) => Some(s),
            Action::Explicit(ExplicitAction::Keystroke { keys }) => Some(keys),
            Action::Explicit(ExplicitAction::Command { .. }) => None,
        }
    }

    /// Get the command string if this is a command action.
    pub fn command(&self) -> Option<&str> {
        match self {
            Action::Explicit(ExplicitAction::Command { run }) => Some(run),
            _ => None,
        }
    }
}

/// Default config file path: `~/.config/hidpp/config.toml`.
pub fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    Path::new(&home)
        .join(".config")
        .join("hidpp")
        .join("config.toml")
}

/// Validate that a TOML string is a valid daemon config.
pub fn validate(content: &str) -> anyhow::Result<()> {
    parse(content)?;
    Ok(())
}

/// Parse a TOML string into a Config.
fn parse(content: &str) -> anyhow::Result<Config> {
    let raw: RawConfig = toml::from_str(content)?;

    let buttons: HashMap<u16, Action> = raw
        .buttons
        .into_iter()
        .map(|(k, v)| {
            let cid: u16 = k
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid button CID: {k}"))?;
            Ok((cid, v))
        })
        .collect::<anyhow::Result<_>>()?;

    let gestures: HashMap<u16, GestureConfig> = raw
        .gestures
        .into_iter()
        .map(|(k, v)| {
            let cid: u16 = k
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid gesture CID: {k}"))?;
            Ok((cid, v))
        })
        .collect::<anyhow::Result<_>>()?;

    // Validate: no CID in both buttons and gestures.
    for cid in buttons.keys() {
        if gestures.contains_key(cid) {
            anyhow::bail!(
                "CID {cid} appears in both [buttons] and [gestures] — use one or the other"
            );
        }
    }

    Ok(Config { buttons, gestures })
}

/// Load config from a TOML file. Returns default config if file doesn't exist.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    if !path.exists() {
        tracing::info!("no config at {}, using defaults", path.display());
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(path)?;
    let config = parse(&content)?;

    tracing::debug!(
        "loaded config from {} ({} buttons, {} gestures)",
        path.display(),
        config.buttons.len(),
        config.gestures.len(),
    );
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_buttons_and_gestures() {
        let config = parse(
            r#"
[buttons]
83 = "alt+left"
86 = "alt+right"

[gestures.195]
up = "ctrl+up"
down = "ctrl+down"
left = "ctrl+left"
right = "ctrl+right"
tap = "playpause"
threshold = 75
"#,
        )
        .unwrap();
        assert_eq!(config.buttons.len(), 2);
        assert_eq!(config.gestures.len(), 1);
        assert!(config.is_gesture_cid(195));
        assert!(!config.is_gesture_cid(83));

        let g = &config.gestures[&195];
        assert_eq!(g.threshold, 75);
        assert!(g.up.is_some());
        assert!(g.tap.is_some());
    }

    #[test]
    fn default_threshold() {
        let config = parse(
            r#"
[gestures.195]
up = "ctrl+up"
"#,
        )
        .unwrap();
        assert_eq!(config.gestures[&195].threshold, 50);
    }

    #[test]
    fn all_diverted_cids_combines() {
        let config = parse(
            r#"
[buttons]
83 = "alt+left"

[gestures.195]
up = "ctrl+up"
"#,
        )
        .unwrap();
        let cids: Vec<u16> = config.all_diverted_cids().collect();
        assert!(cids.contains(&83));
        assert!(cids.contains(&195));
    }
}
