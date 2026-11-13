//! `settings.toml` next to the game: a small, commented file a person (or
//! RecompLauncher, which edits it line by line) can change. Written with
//! the defaults on first start; never shipped in a release, so updates
//! don't overwrite it. Command-line options win over it.

use std::fs;

pub struct Settings {
    /// window size at start, in multiples of 128x128
    pub scale: u32,
    pub fullscreen: bool,
    /// "sharp": whole-pixel scaling; "fill": as large as fits, same shape
    pub scaling: String,
    /// sound volume 0..=100
    pub volume: u32,
    pub sound: bool,
    pub rumble: bool,
    /// text language: "xx" (English) or "tl-PH" (Filipino)
    pub language: String,
    /// read controllers while the window isn't in front
    pub background_input: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scale: 5,
            fullscreen: false,
            scaling: "sharp".into(),
            volume: 100,
            sound: true,
            rumble: true,
            language: "xx".into(),
            background_input: false,
        }
    }
}

const TEMPLATE: &str = r#"# Bounce RE settings. Command-line options override these.

# Window size at start, in multiples of the phone's 128x128 screen (1-16).
scale = 5

# Start in full screen (F11 or Alt+Enter switches while playing).
fullscreen = false

# "sharp": whole-pixel scaling, crisp but with borders.
# "fill": as large as the window allows, keeping the shape.
scaling = "sharp"

# Sound effect volume, 0-100.
volume = 100

# Sound effects on or off (M toggles while playing).
sound = true

# Gamepad rumble when the ball pops.
rumble = true

# Text language: "xx" for English, "tl-PH" for Filipino.
language = "xx"

# Keep reading controllers while the window isn't in front.
background_input = false
"#;

fn unquote(v: &str) -> &str {
    let v = v.trim();
    v.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(v)
}

fn parse(text: &str, s: &mut Settings) {
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        let b = |v: &str| matches!(unquote(v), "true" | "1" | "yes" | "on");
        match k {
            "scale" => s.scale = unquote(v).parse::<u32>().unwrap_or(s.scale).clamp(1, 16),
            "fullscreen" => s.fullscreen = b(v),
            "scaling" => s.scaling = unquote(v).to_string(),
            "volume" => s.volume = unquote(v).parse::<u32>().unwrap_or(s.volume).min(100),
            "sound" | "music" => s.sound = b(v),
            "rumble" => s.rumble = b(v),
            "language" => s.language = unquote(v).to_string(),
            "background_input" => s.background_input = b(v),
            _ => {}
        }
    }
}

/// Read `settings.toml`, creating it with the defaults if it's missing.
pub fn load() -> Settings {
    let path = crate::paths::settings_file();
    let mut s = Settings::default();
    match fs::read_to_string(&path) {
        Ok(text) => parse(&text, &mut s),
        Err(_) => {
            if let Err(e) = fs::write(&path, TEMPLATE) {
                crate::log!("couldn't write {}: {e}", path.display());
            }
        }
    }
    s
}
