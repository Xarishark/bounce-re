//! First-run setup: the game ships without any of the original files. The
//! player supplies their own `Bounce.jar`; this checks it
//! and extracts the art, levels, sounds and text into `data/`.
//!
//! The same code serves three entry points: `--prepare --game <jar>`
//! (RecompLauncher's headless setup step), `--game <jar>` (prepare on
//! start if needed), and the in-window setup screen (drop or browse).

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The supported release: Bounce 1.9 for Nokia Series 40 (128x128).
pub const JAR_NAME: &str = "Bounce.jar";
pub const JAR_SIZE: u64 = 56_375;
pub const JAR_SHA256: &str = "3fad7bc2b82e7ddeaffbbb205a239897b1380829abc557c7ae6909a0245179c6";

/// Bump when the extracted layout changes, so old `data/` folders are redone.
const LAYOUT: u32 = 1;
const MARKER: &str = ".prepared";

/// Process exit codes (also listed in recomplauncher.json `exit_codes`).
pub mod exit {
    /// the JAR couldn't be found or read
    pub const UNREADABLE: i32 = 2;
    /// a file that isn't the supported JAR
    pub const WRONG_FILE: i32 = 3;
    /// the game's folder can't be written
    pub const NOT_WRITABLE: i32 = 4;
    /// no prepared data and no JAR given (headless / launcher start)
    pub const NOT_SET_UP: i32 = 5;
}

#[derive(Debug)]
pub struct SetupError {
    pub code: i32,
    pub message: String,
}

fn err(code: i32, message: impl Into<String>) -> SetupError {
    SetupError { code, message: message.into() }
}

/// Is `data/` prepared from the supported JAR with the current layout?
pub fn prepared(data: &Path) -> bool {
    fs::read_to_string(data.join(MARKER)).map(|m| m.trim() == format!("{JAR_SHA256} {LAYOUT}")).unwrap_or(false)
}

/// Read and check the player's JAR.
fn read_jar(jar: &Path) -> Result<Vec<u8>, SetupError> {
    let bytes = fs::read(jar).map_err(|e| err(exit::UNREADABLE, format!("Couldn't read {}: {e}", jar.display())))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if bytes.len() as u64 != JAR_SIZE || hash != JAR_SHA256 {
        return Err(err(
            exit::WRONG_FILE,
            format!(
                "{} isn't the supported Bounce.\nIt needs the Nokia 128x128 release of Bounce 1.9, {JAR_NAME} ({JAR_SIZE} bytes).",
                jar.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            ),
        ));
    }
    Ok(bytes)
}

/// Check `jar` and extract its resources into `data`, replacing what's
/// there. `progress(fraction, text)` reports each step.
pub fn prepare(jar: &Path, data: &Path, mut progress: impl FnMut(f32, &str)) -> Result<usize, SetupError> {
    progress(0.0, "Checking your Bounce JAR");
    let bytes = read_jar(jar)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| err(exit::WRONG_FILE, format!("Couldn't open the JAR: {e}")))?;

    // Write into a fresh sibling folder, then swap it in.
    let parent = data.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let tmp = parent.join("data.new");
    let not_writable = |e: std::io::Error| err(exit::NOT_WRITABLE, format!("Couldn't write the game data to {}: {e}", parent.display()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(not_writable)?;

    let total = zip.len().max(1);
    let mut written = 0;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| err(exit::WRONG_FILE, format!("Damaged JAR: {e}")))?;
        let name = f.name().map_err(|e| err(exit::WRONG_FILE, format!("Damaged JAR: {e}")))?.to_string();
        // only resources: no code, no manifest, no folders, nothing outside the root
        let skip = f.is_dir()
            || name.ends_with(".class")
            || name.starts_with("META-INF/")
            || name.contains("..")
            || name.starts_with('/');
        if skip {
            continue;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| err(exit::WRONG_FILE, format!("Damaged JAR entry {name}: {e}")))?;
        let out = tmp.join(&name);
        if let Some(d) = out.parent() {
            fs::create_dir_all(d).map_err(not_writable)?;
        }
        fs::write(&out, &buf).map_err(not_writable)?;
        written += 1;
        let what = if name.starts_with("levels/") {
            "Unpacking levels"
        } else if name.starts_with("sounds/") {
            "Unpacking sounds"
        } else if name.starts_with("lang.") {
            "Unpacking text"
        } else {
            "Unpacking art"
        };
        progress(0.1 + 0.8 * (i + 1) as f32 / total as f32, what);
    }

    progress(0.95, "Finishing");
    fs::write(tmp.join(MARKER), format!("{JAR_SHA256} {LAYOUT}\n")).map_err(not_writable)?;
    let _ = fs::remove_dir_all(data);
    fs::rename(&tmp, data).map_err(not_writable)?;
    progress(1.0, "Ready");
    Ok(written)
}

/// Is `jar` the supported file? (quick check for auto-detection)
pub fn is_supported(jar: &Path) -> bool {
    fs::metadata(jar).map(|m| m.len() == JAR_SIZE).unwrap_or(false) && read_jar(jar).is_ok()
}

/// A supported JAR sitting next to the game, if any.
pub fn find_nearby(root: &Path) -> Option<PathBuf> {
    let mut found = fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jar")))
        .collect::<Vec<_>>();
    found.sort();
    found.into_iter().find(|p| is_supported(p))
}
