//! `logs/bounce.log`, kept for the last few runs. RecompLauncher
//! attaches the newest `.log` in `logs/` to crash reports.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

static FILE: Mutex<Option<File>> = Mutex::new(None);
const KEEP: usize = 3;

/// Rotate old logs and open a new one. Logging before this only goes to stderr.
pub fn init() {
    let dir = crate::paths::logs_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    // bounce.log -> bounce.1.log -> bounce.2.log
    for i in (1..KEEP).rev() {
        let from = if i == 1 { dir.join("bounce.log") } else { dir.join(format!("bounce.{}.log", i - 1)) };
        let _ = fs::rename(from, dir.join(format!("bounce.{i}.log")));
    }
    if let Ok(f) = OpenOptions::new().create(true).write(true).truncate(true).open(dir.join("bounce.log")) {
        *FILE.lock().unwrap() = Some(f);
    }
    std::panic::set_hook(Box::new(|info| {
        write_line(&format!("crash: {info}"));
    }));
}

pub fn write_line(s: &str) {
    eprintln!("{s}");
    if let Ok(mut g) = FILE.lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "{s}");
            let _ = f.flush();
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::log::write_line(&format!($($t)*)) };
}
