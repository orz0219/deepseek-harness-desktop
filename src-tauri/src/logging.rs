//! Minimal file + stderr logger (no external dependencies).
//!
//! PLAN 门禁 D: launcher logs belong in `~/Library/Logs/<identifier>/`. A GUI
//! app launched from Finder has no visible stdout, so without a file every
//! diagnostic is lost. This module writes timestamped lines to
//! `<logs_dir>/launcher.log` and mirrors them to stderr for `cargo tauri dev`.

use std::fs::{create_dir_all, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_FILE: OnceLock<Option<Mutex<File>>> = OnceLock::new();

/// Logs directory for the given bundle identifier:
/// `~/Library/Logs/<identifier>`.
pub fn logs_dir(identifier: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    PathBuf::from(home)
        .join("Library")
        .join("Logs")
        .join(identifier)
}

/// Open (append) `<logs_dir>/launcher.log`. Returns false when the file could
/// not be opened; logging then degrades to stderr only.
pub fn init(identifier: &str) -> bool {
    let dir = logs_dir(identifier);
    let ok = create_dir_all(&dir).is_ok();
    let file = if ok {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("launcher.log"))
            .ok()
    } else {
        None
    };
    let enabled = file.is_some();
    let _ = LOG_FILE.set(file.map(Mutex::new));
    enabled
}

/// RFC3339-ish local-free timestamp (`YYYY-MM-DDTHH:MM:SS.mmmZ`, UTC).
/// Implemented by hand to stay dependency-free.
fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let millis = now.subsec_millis();
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (h, m, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

/// Days-since-epoch → (year, month, day). Howard Hinnant's `civil_from_days`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Write one line to the log file and mirror it to stderr.
pub fn log(msg: &str) {
    let line = format!("[{}] {}", timestamp(), msg);
    eprintln!("{line}");
    if let Some(Some(file)) = LOG_FILE.get().map(|f| f.as_ref()) {
        if let Ok(mut f) = file.lock() {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Convenience wrapper with `info`-ish plain formatting.
pub fn info(msg: &str) {
    log(msg);
}

/// Convenience wrapper that prefixes errors.
pub fn error(msg: &str) {
    log(&format!("ERROR: {msg}"));
}
