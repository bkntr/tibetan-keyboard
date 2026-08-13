//! Persistent user configuration and hotkey parsing.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fmt, fs, io, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    /// Human-editable global toggle chord or repeated stroke, for example
    /// `Ctrl+Alt+T` or `Shift+Space+Space`.
    pub hotkey: String,
    /// Start in Tibetan mode rather than disabled mode.
    pub enabled_on_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: "Shift+Space+Space".into(),
            enabled_on_start: false,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        if let Some(path) = std::env::var_os("TIBETAN_EWTS_CONFIG") {
            return PathBuf::from(path);
        }
        ProjectDirs::from("org", "Tibetan Keyboard", "Tibetan EWTS Keyboard")
            .map(|dirs| dirs.config_dir().join("settings.toml"))
            .unwrap_or_else(|| PathBuf::from("settings.toml"))
    }

    pub fn load_or_create() -> io::Result<Self> {
        let path = Self::path();
        if path.exists() {
            let text = fs::read_to_string(path)?;
            toml::from_str(&text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }

    pub fn parsed_hotkey(&self) -> Result<Hotkey, HotkeyParseError> {
        self.hotkey.parse()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    /// One or more Windows virtual-key values. Multi-stroke hotkeys repeat the
    /// same key, for example `Shift+Space+Space`.
    pub keys: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyMatch {
    Pass,
    Suppress,
    Trigger,
    /// Replay the suppressed prefix, then pass the current key event through.
    CancelAndPass(Vec<u16>),
    /// Replay the suppressed prefix, then process the current key as normal.
    CancelAndReprocess(Vec<u16>),
}

/// Stateful recognizer for single- and repeated-stroke hotkeys.
#[derive(Debug, Clone)]
pub struct HotkeyMatcher {
    hotkey: Hotkey,
    progress: usize,
    active_key_down: bool,
    suppress_release: Option<u16>,
}

impl HotkeyMatcher {
    pub fn new(hotkey: Hotkey) -> Self {
        Self {
            hotkey,
            progress: 0,
            active_key_down: false,
            suppress_release: None,
        }
    }

    pub fn hotkey(&self) -> &Hotkey {
        &self.hotkey
    }

    pub fn reset(&mut self) {
        self.progress = 0;
        self.active_key_down = false;
        self.suppress_release = None;
    }

    /// Examine one low-level key event. `modifiers` is the modifier state just
    /// before the event, matching the semantics of `WH_KEYBOARD_LL`.
    pub fn handle(&mut self, key: u16, is_down: bool, modifiers: Modifiers) -> HotkeyMatch {
        if self.suppress_release == Some(key) {
            if !is_down {
                self.suppress_release = None;
            }
            return HotkeyMatch::Suppress;
        }

        if self.progress > 0 {
            if !is_down && required_modifier_key(key, self.hotkey.modifiers) {
                return self.cancel(false);
            }

            let sequence_key = self.hotkey.keys[0];
            if !is_down && key == sequence_key && self.active_key_down {
                self.active_key_down = false;
                return HotkeyMatch::Suppress;
            }

            if is_down {
                if key == sequence_key && self.active_key_down {
                    // Key auto-repeat is not a second stroke. An actual key-up
                    // must separate activation strokes.
                    return HotkeyMatch::Suppress;
                }

                let expected = self.hotkey.keys[self.progress];
                if key == expected && modifiers == self.hotkey.modifiers {
                    self.progress += 1;
                    self.active_key_down = true;
                    if self.progress == self.hotkey.keys.len() {
                        self.progress = 0;
                        self.active_key_down = false;
                        self.suppress_release = Some(key);
                        return HotkeyMatch::Trigger;
                    }
                    return HotkeyMatch::Suppress;
                }

                return self.cancel(true);
            }

            return HotkeyMatch::Pass;
        }

        if is_down && key == self.hotkey.keys[0] && modifiers == self.hotkey.modifiers {
            if self.hotkey.keys.len() == 1 {
                self.suppress_release = Some(key);
                return HotkeyMatch::Trigger;
            }
            self.progress = 1;
            self.active_key_down = true;
            return HotkeyMatch::Suppress;
        }

        HotkeyMatch::Pass
    }

    fn cancel(&mut self, reprocess: bool) -> HotkeyMatch {
        let replay = self.hotkey.keys[..self.progress].to_vec();
        if self.active_key_down {
            self.suppress_release = Some(self.hotkey.keys[0]);
        }
        self.progress = 0;
        self.active_key_down = false;
        if reprocess {
            HotkeyMatch::CancelAndReprocess(replay)
        } else {
            HotkeyMatch::CancelAndPass(replay)
        }
    }
}

fn required_modifier_key(key: u16, modifiers: Modifiers) -> bool {
    match key {
        0x10 | 0xA0 | 0xA1 => modifiers.shift,
        0x11 | 0xA2 | 0xA3 => modifiers.ctrl,
        0x12 | 0xA4 | 0xA5 => modifiers.alt,
        0x5B | 0x5C => modifiers.win,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyParseError(pub String);

impl fmt::Display for HotkeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HotkeyParseError {}

impl FromStr for Hotkey {
    type Err = HotkeyParseError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut modifiers = Modifiers::default();
        let mut keys = Vec::new();

        for raw in source.split('+') {
            let token = raw.trim();
            if token.is_empty() {
                return Err(HotkeyParseError("empty hotkey component".into()));
            }
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                "win" | "windows" | "super" => modifiers.win = true,
                _ => keys.push(parse_key(token)?),
            }
        }

        let first_key = keys
            .first()
            .copied()
            .ok_or_else(|| HotkeyParseError("hotkey needs a non-modifier key".into()))?;
        if keys.iter().any(|key| *key != first_key) {
            return Err(HotkeyParseError(
                "multi-stroke hotkeys must repeat the same main key".into(),
            ));
        }
        if !modifiers.ctrl && !modifiers.alt && !modifiers.shift && !modifiers.win {
            return Err(HotkeyParseError(
                "hotkey needs at least one modifier".into(),
            ));
        }
        Ok(Self { modifiers, keys })
    }
}

fn parse_key(token: &str) -> Result<u16, HotkeyParseError> {
    let upper = token.to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Ok(byte as u16);
        }
    }
    if let Some(number) = upper.strip_prefix('F').and_then(|n| n.parse::<u16>().ok())
        && (1..=24).contains(&number)
    {
        return Ok(0x70 + number - 1);
    }
    match upper.as_str() {
        "SPACE" => Ok(0x20),
        "TAB" => Ok(0x09),
        "ENTER" | "RETURN" => Ok(0x0D),
        "ESC" | "ESCAPE" => Ok(0x1B),
        _ => Err(HotkeyParseError(format!("unsupported hotkey key: {token}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_configurable_hotkeys() {
        let hotkey: Hotkey = "Ctrl+Alt+T".parse().unwrap();
        assert_eq!(hotkey.keys, vec![b'T' as u16]);
        assert_eq!(
            hotkey.modifiers,
            Modifiers {
                ctrl: true,
                alt: true,
                shift: false,
                win: false
            }
        );

        let hotkey: Hotkey = "Win+Shift+F12".parse().unwrap();
        assert_eq!(hotkey.keys, vec![0x7B]);
        assert!(hotkey.modifiers.win && hotkey.modifiers.shift);

        let hotkey: Hotkey = "Shift+Space+Space".parse().unwrap();
        assert_eq!(hotkey.keys, vec![0x20, 0x20]);
        assert_eq!(
            hotkey.modifiers,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            }
        );
    }

    #[test]
    fn rejects_unsafe_bare_keys() {
        assert!("T".parse::<Hotkey>().is_err());
        assert!("Ctrl+Alt".parse::<Hotkey>().is_err());
        assert!("Ctrl+T+Y".parse::<Hotkey>().is_err());
    }

    #[test]
    fn config_roundtrips_toml() {
        let config = Config {
            hotkey: "Ctrl+Shift+Space".into(),
            enabled_on_start: true,
        };
        let text = toml::to_string(&config).unwrap();
        let decoded: Config = toml::from_str(&text).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn default_config_has_valid_hotkey() {
        let config = Config::default();
        assert_eq!(config.hotkey, "Shift+Space+Space");
        assert!(config.parsed_hotkey().is_ok());
    }

    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Modifiers::default()
        }
    }

    #[test]
    fn repeated_stroke_triggers_only_after_distinct_presses() {
        let hotkey: Hotkey = "Shift+Space+Space".parse().unwrap();
        let mut matcher = HotkeyMatcher::new(hotkey);

        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, false, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Trigger);
        assert_eq!(matcher.handle(0x20, false, shift()), HotkeyMatch::Suppress);
    }

    #[test]
    fn releasing_shift_cancels_and_replays_prefix() {
        let hotkey: Hotkey = "Shift+Space+Space".parse().unwrap();
        let mut matcher = HotkeyMatcher::new(hotkey);

        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, false, shift()), HotkeyMatch::Suppress);
        assert_eq!(
            matcher.handle(0xA0, false, shift()),
            HotkeyMatch::CancelAndPass(vec![0x20])
        );
        assert_eq!(
            matcher.handle(0x20, true, Modifiers::default()),
            HotkeyMatch::Pass
        );
    }

    #[test]
    fn another_key_cancels_and_requires_a_fresh_sequence() {
        let hotkey: Hotkey = "Shift+Space+Space".parse().unwrap();
        let mut matcher = HotkeyMatcher::new(hotkey);

        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, false, shift()), HotkeyMatch::Suppress);
        assert_eq!(
            matcher.handle(b'A' as u16, true, shift()),
            HotkeyMatch::CancelAndReprocess(vec![0x20])
        );
        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, false, shift()), HotkeyMatch::Suppress);
        assert_eq!(matcher.handle(0x20, true, shift()), HotkeyMatch::Trigger);
    }
}
