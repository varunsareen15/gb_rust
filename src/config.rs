use minifb::Key;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub controls: Controls,
    pub display: Display,
    pub speed: Speed,
}

#[derive(Serialize, Deserialize)]
pub struct Controls {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub select: String,
    pub start: String,
}

#[derive(Serialize, Deserialize)]
pub struct Display {
    pub scale: String,
    pub palette: String,
    pub scanlines: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Speed {
    /// 0 = uncapped, 2 = 2x, 4 = 4x, etc.
    pub fast_forward_multiplier: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            controls: Controls {
                up: "Up".into(),
                down: "Down".into(),
                left: "Left".into(),
                right: "Right".into(),
                a: "Z".into(),
                b: "X".into(),
                select: "Backspace".into(),
                start: "Enter".into(),
            },
            display: Display {
                scale: "4x".into(),
                palette: "Classic".into(),
                scanlines: false,
            },
            speed: Speed {
                fast_forward_multiplier: 0,
            },
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("gb_rust");
        path.push("config.toml");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => eprintln!("Error parsing {}: {}; using defaults", path.display(), e),
                },
                Err(e) => eprintln!("Error reading {}: {}; using defaults", path.display(), e),
            }
        } else {
            let config = Config::default();
            config.write_defaults();
            return config;
        }
        Config::default()
    }

    fn write_defaults(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Error creating config directory: {}", e);
                return;
            }
        }
        let contents = toml::to_string_pretty(self).expect("Failed to serialize config");
        if let Err(e) = std::fs::write(&path, contents) {
            eprintln!("Error writing {}: {}", path.display(), e);
        } else {
            eprintln!("Wrote default config to {}", path.display());
        }
    }

    pub fn scale_index(&self) -> usize {
        match self.display.scale.as_str() {
            "2x" => 0,
            "4x" => 1,
            "8x" => 2,
            _ => 1,
        }
    }

    pub fn palette_index(&self) -> usize {
        match self.display.palette.as_str() {
            "Classic" => 0,
            "DMG Green" => 1,
            "Grayscale" => 2,
            "Pocket" => 3,
            _ => 0,
        }
    }

    pub fn joypad_key_map(&self) -> Vec<(Key, crate::joypad::JoypadKey)> {
        use crate::joypad::JoypadKey;
        let pairs = [
            (&self.controls.right, JoypadKey::Right),
            (&self.controls.left, JoypadKey::Left),
            (&self.controls.up, JoypadKey::Up),
            (&self.controls.down, JoypadKey::Down),
            (&self.controls.a, JoypadKey::A),
            (&self.controls.b, JoypadKey::B),
            (&self.controls.select, JoypadKey::Select),
            (&self.controls.start, JoypadKey::Start),
        ];
        pairs.iter().filter_map(|(name, jk)| {
            key_name_to_minifb(name).map(|k| (k, *jk))
        }).collect()
    }
}

pub fn key_name_to_minifb(name: &str) -> Option<Key> {
    match name {
        "A" => Some(Key::A), "B" => Some(Key::B), "C" => Some(Key::C),
        "D" => Some(Key::D), "E" => Some(Key::E), "F" => Some(Key::F),
        "G" => Some(Key::G), "H" => Some(Key::H), "I" => Some(Key::I),
        "J" => Some(Key::J), "K" => Some(Key::K), "L" => Some(Key::L),
        "M" => Some(Key::M), "N" => Some(Key::N), "O" => Some(Key::O),
        "P" => Some(Key::P), "Q" => Some(Key::Q), "R" => Some(Key::R),
        "S" => Some(Key::S), "T" => Some(Key::T), "U" => Some(Key::U),
        "V" => Some(Key::V), "W" => Some(Key::W), "X" => Some(Key::X),
        "Y" => Some(Key::Y), "Z" => Some(Key::Z),
        "0" => Some(Key::Key0), "1" => Some(Key::Key1), "2" => Some(Key::Key2),
        "3" => Some(Key::Key3), "4" => Some(Key::Key4), "5" => Some(Key::Key5),
        "6" => Some(Key::Key6), "7" => Some(Key::Key7), "8" => Some(Key::Key8),
        "9" => Some(Key::Key9),
        "Up" => Some(Key::Up), "Down" => Some(Key::Down),
        "Left" => Some(Key::Left), "Right" => Some(Key::Right),
        "Enter" | "Return" => Some(Key::Enter),
        "Space" => Some(Key::Space),
        "Backspace" => Some(Key::Backspace),
        "Tab" => Some(Key::Tab),
        "LeftShift" => Some(Key::LeftShift),
        "RightShift" => Some(Key::RightShift),
        "LeftCtrl" => Some(Key::LeftCtrl),
        "RightCtrl" => Some(Key::RightCtrl),
        "Escape" | "Esc" => Some(Key::Escape),
        "Comma" | "," => Some(Key::Comma),
        "Period" | "." => Some(Key::Period),
        "Slash" | "/" => Some(Key::Slash),
        "Semicolon" | ";" => Some(Key::Semicolon),
        "Apostrophe" | "'" => Some(Key::Apostrophe),
        "LeftBracket" | "[" => Some(Key::LeftBracket),
        "RightBracket" | "]" => Some(Key::RightBracket),
        "Backslash" | "\\" => Some(Key::Backslash),
        "Minus" | "-" => Some(Key::Minus),
        "Equal" | "=" => Some(Key::Equal),
        _ => {
            eprintln!("Unknown key name in config: '{}'", name);
            None
        }
    }
}
