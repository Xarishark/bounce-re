//! Where everything lives. The game is portable: one root folder (the one
//! holding the executable, or `BOUNCE_HOME`) contains
//!
//! - `save/`          the RecordStore files (`bounceRMS.rms`)
//! - `data/`          art, music and text extracted from the player's JAR
//! - `logs/`          `bounce.log` from the last few runs
//! - `settings.toml`  the player's settings
//!
//! There is no fallback to a per-user folder: if the root can't be written
//! the game stops with an error.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Pick the root folder and check that it can be written. Call once at start.
pub fn init() -> Result<PathBuf, String> {
    let root = match std::env::var_os("BOUNCE_HOME") {
        Some(d) => PathBuf::from(d),
        None => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .ok_or_else(|| "Can't find the game's folder.".to_string())?,
    };
    let save = root.join("save");
    let probe = save.join(".write-test");
    let ok = fs::create_dir_all(&save).and_then(|_| fs::write(&probe, b""));
    let _ = fs::remove_file(&probe);
    if let Err(e) = ok {
        return Err(format!(
            "Can't write to the game's folder:\n{}\n\n{e}\n\nMove the game to a folder you can write to (not Program Files).",
            root.display()
        ));
    }
    let _ = ROOT.set(root.clone());
    Ok(root)
}

pub fn root() -> PathBuf {
    ROOT.get().cloned().expect("paths::init not called")
}
pub fn save_dir() -> PathBuf {
    root().join("save")
}
pub fn data_dir() -> PathBuf {
    root().join("data")
}
pub fn logs_dir() -> PathBuf {
    root().join("logs")
}
pub fn settings_file() -> PathBuf {
    root().join("settings.toml")
}
