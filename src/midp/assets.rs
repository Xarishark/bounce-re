//! `getResourceAsStream` for the game: the files extracted from the
//! player's JAR into `data/` (see `setup`), read into memory once at start.

use crate::midp::gfx::{Image, Img};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::OnceLock;

static FILES: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();

fn walk(dir: &Path, root: &Path, out: &mut HashMap<String, Vec<u8>>) -> std::io::Result<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, root, out)?;
        } else if let Ok(rel) = p.strip_prefix(root) {
            let key = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
            if !key.starts_with("/.") {
                out.insert(key, std::fs::read(&p)?);
            }
        }
    }
    Ok(())
}

/// Read every file under `dir` (the prepared `data/` folder). Call once.
pub fn load(dir: &Path) -> Result<usize, String> {
    let mut files = HashMap::new();
    walk(dir, dir, &mut files).map_err(|e| format!("Can't read the game data in {}: {e}", dir.display()))?;
    let n = files.len();
    let _ = FILES.set(files);
    Ok(n)
}

/// `getResourceAsStream(path)`
pub fn get(path: &str) -> Option<&'static [u8]> {
    FILES.get()?.get(path).map(|v| v.as_slice())
}

/// `Image.createImage(path)`; None stands for the IOException / null result.
pub fn image(path: &str) -> Option<Img> {
    let img = get(path).and_then(Image::from_png).map(Rc::new);
    if img.is_none() {
        crate::log!("missing image {path}");
    }
    img
}
