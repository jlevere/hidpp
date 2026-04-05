use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level daemon config.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Map of CID → action string.
    /// e.g. `83 = "alt+left"` means Back button sends Alt+Left.
    #[serde(default)]
    pub buttons: HashMap<u16, Action>,
}

/// An action triggered by a diverted button.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Action {
    /// Shorthand: just a keystroke string like "ctrl+left".
    Keystroke(String),
    /// Explicit action with type.
    Explicit(ExplicitAction),
}

/// Explicit action definition.
#[derive(Debug, Deserialize)]
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
    Path::new(&home).join(".config").join("hidpp").join("config.toml")
}

/// Load config from a TOML file. Returns default config if file doesn't exist.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    if !path.exists() {
        tracing::info!("no config at {}, using defaults", path.display());
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    tracing::info!(
        "loaded config from {} ({} button mappings)",
        path.display(),
        config.buttons.len(),
    );
    Ok(config)
}
