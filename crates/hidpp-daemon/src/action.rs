use std::sync::Mutex;

use enigo::{Direction, Enigo, Key, Keyboard as _, Settings};
use tracing::{error, info};

use crate::config::{Action, ExplicitAction};

/// Global enigo instance. Must be created once and reused (platform resources).
static ENIGO: Mutex<Option<Enigo>> = Mutex::new(None);

/// Initialize the input injection backend. Call once at startup.
pub fn init() -> anyhow::Result<()> {
    let enigo = Enigo::new(&Settings {
        release_keys_when_dropped: true,
        ..Settings::default()
    })
    .map_err(|e| anyhow::anyhow!("failed to initialize input backend: {e}"))?;

    *ENIGO.lock().unwrap() = Some(enigo);
    info!("input injection initialized");
    Ok(())
}

/// Execute an action.
pub fn execute(action: &Action) {
    match action {
        Action::Keystroke(keys) => execute_keystroke(keys),
        Action::Explicit(ExplicitAction::Keystroke { keys }) => execute_keystroke(keys),
        Action::Explicit(ExplicitAction::Command { run }) => execute_command(run),
    }
}

/// Parse and execute a keystroke string like "ctrl+shift+left".
fn execute_keystroke(keystroke: &str) {
    let parts: Vec<&str> = keystroke.split('+').map(str::trim).collect();
    if parts.is_empty() {
        return;
    }

    // Everything except the last part is a modifier, last part is the main key.
    let (modifier_strs, main_str) = parts.split_at(parts.len() - 1);

    let modifiers: Vec<Key> = modifier_strs.iter().filter_map(|s| parse_key(s)).collect();

    let Some(main_key) = main_str.first().and_then(|s| parse_key(s)) else {
        error!("unknown key in keystroke: {keystroke}");
        return;
    };

    let mut guard = ENIGO.lock().unwrap();
    let Some(enigo) = guard.as_mut() else {
        error!("input backend not initialized");
        return;
    };

    // Press modifiers, click main key, release modifiers in reverse.
    for m in &modifiers {
        if let Err(e) = enigo.key(*m, Direction::Press) {
            error!("key press failed: {e}");
            return;
        }
    }

    if let Err(e) = enigo.key(main_key, Direction::Click) {
        error!("key click failed: {e}");
    }

    for m in modifiers.iter().rev() {
        if let Err(e) = enigo.key(*m, Direction::Release) {
            error!("key release failed: {e}");
        }
    }

    info!("keystroke: {keystroke}");
}

/// Run a shell command in the background. Reaps the child on a separate thread.
fn execute_command(cmd: &str) {
    info!("command: {cmd}");

    #[cfg(unix)]
    let result = std::process::Command::new("sh").args(["-c", cmd]).spawn();

    #[cfg(windows)]
    let result = std::process::Command::new("cmd").args(["/C", cmd]).spawn();

    match result {
        Ok(mut child) => {
            // Reap the child process to avoid zombies.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => error!("command failed: {e}"),
    }
}

/// Map a key name string to an enigo Key.
fn parse_key(name: &str) -> Option<Key> {
    let lower = name.to_lowercase();
    let key = match lower.as_str() {
        // Modifiers.
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "meta" | "cmd" | "command" | "super" | "win" => Key::Meta,

        // Navigation.
        "left" | "leftarrow" => Key::LeftArrow,
        "right" | "rightarrow" => Key::RightArrow,
        "up" | "uparrow" => Key::UpArrow,
        "down" | "downarrow" => Key::DownArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,

        // Editing.
        "return" | "enter" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "capslock" => Key::CapsLock,

        // Function keys.
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "f13" => Key::F13,
        "f14" => Key::F14,
        "f15" => Key::F15,
        "f16" => Key::F16,
        "f17" => Key::F17,
        "f18" => Key::F18,
        "f19" => Key::F19,
        "f20" => Key::F20,

        // Media.
        "playpause" | "play" => Key::MediaPlayPause,
        "next" | "nexttrack" => Key::MediaNextTrack,
        "prev" | "prevtrack" => Key::MediaPrevTrack,
        "volumeup" | "volup" => Key::VolumeUp,
        "volumedown" | "voldown" => Key::VolumeDown,
        "mute" | "volumemute" => Key::VolumeMute,

        // Single character → Unicode key.
        s if s.len() == 1 => {
            let ch = s.chars().next()?;
            Key::Unicode(ch)
        }

        _ => {
            error!("unknown key name: {name}");
            return None;
        }
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modifiers() {
        assert!(matches!(parse_key("ctrl"), Some(Key::Control)));
        assert!(matches!(parse_key("Ctrl"), Some(Key::Control)));
        assert!(matches!(parse_key("CMD"), Some(Key::Meta)));
        assert!(matches!(parse_key("shift"), Some(Key::Shift)));
        assert!(matches!(parse_key("alt"), Some(Key::Alt)));
    }

    #[test]
    fn parse_arrows() {
        assert!(matches!(parse_key("left"), Some(Key::LeftArrow)));
        assert!(matches!(parse_key("Right"), Some(Key::RightArrow)));
        assert!(matches!(parse_key("UP"), Some(Key::UpArrow)));
    }

    #[test]
    fn parse_single_char() {
        assert!(matches!(parse_key("a"), Some(Key::Unicode('a'))));
        assert!(matches!(parse_key("z"), Some(Key::Unicode('z'))));
        assert!(matches!(parse_key("1"), Some(Key::Unicode('1'))));
    }

    #[test]
    fn parse_function_keys() {
        assert!(matches!(parse_key("f1"), Some(Key::F1)));
        assert!(matches!(parse_key("F12"), Some(Key::F12)));
    }

    #[test]
    fn parse_unknown() {
        assert!(parse_key("nonexistent").is_none());
    }
}
