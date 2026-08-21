//! Pure, Tauri-free core of the DeepSeek Harness desktop launcher.
//!
//! This crate holds the logic that does not depend on the Tauri runtime so it
//! can be unit-tested with `cargo test --lib` without pulling the heavy Tauri
//! dependency tree. The Tauri glue lives in `main.rs` (the binary) and calls
//! into this crate.
//!
//! Design contract is kept deliberately minimal and follows dsh's *current*
//! behavior (see `FEASIBILITY.md`):
//!   - we never import dsh internals;
//!   - we only use dsh's CLI (`dsh web --host 127.0.0.1 --port <p> --no-open`)
//!     plus an HTTP readiness probe (`GET /` -> 200) and the loopback-only
//!     shutdown route dsh serves itself.

pub mod archive_ops;
pub mod dsh_launch;
pub mod locate_dsh;
pub mod logging;

pub use archive_ops::{
    delete_archived, dsh_home, restore_archived, sessions_root, workspace_storage_path,
    ArchiveOpsReport,
};
pub use dsh_launch::{
    build_env_snapshot, build_launch_plan, format_env_snapshot, gui_url, load_settings,
    resolve_node, save_settings, settings_dir, settings_path, AppSettings, LaunchPlan,
    DEFAULT_PORT,
};
pub use locate_dsh::locate;

use std::path::PathBuf;

/// Where the dsh binary was found and how trustworthy that finding is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Tier 0: user explicitly chose this path in settings (only fully reliable).
    UserSpecified,
    /// Tier 1: discovered by scanning candidate bin directories.
    Scanned,
    /// Tier 2: resolved from parsed PATH files (/etc/paths, ~/.zprofile, ...).
    PathFile,
    /// Tier 3: last-resort `zsh -lic 'command -v dsh'`.
    ZshLogin,
}

impl Source {
    /// Numeric priority; lower means more trustworthy (used for ordering).
    pub fn priority(&self) -> u8 {
        match self {
            Source::UserSpecified => 0,
            Source::Scanned => 1,
            Source::PathFile => 2,
            Source::ZshLogin => 3,
        }
    }

    /// Stable lowercase label for logs and the setup UI.
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::UserSpecified => "user",
            Source::Scanned => "scanned",
            Source::PathFile => "path-file",
            Source::ZshLogin => "zsh-login",
        }
    }
}

/// A resolved dsh installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DshCandidate {
    /// Absolute path to the `dsh` executable (a node script or binary).
    pub executable: PathBuf,
    /// The node runtime that should run `dsh`. Resolved from the dsh shebang /
    /// settings / PATH. `None` means "let the kernel use the shebang".
    pub node: Option<PathBuf>,
    /// dsh version string from `dsh --version` (e.g. "0.1.0-rc.7").
    pub version: String,
    /// How this candidate was discovered.
    pub source: Source,
}

/// Result of the location pass: the chosen candidate plus all discovered
/// candidates (for diagnostics / user confirmation).
#[derive(Debug, Clone)]
pub struct LocateOutcome {
    /// The candidate the launcher will use (best by source priority then version).
    pub primary: Option<DshCandidate>,
    /// Every candidate found, highest priority / version first.
    pub candidates: Vec<DshCandidate>,
}

/// Extract the HTTP status code from the head of an HTTP response
/// (everything up to the first `\r\n\r\n`; only the status line is needed).
///
/// Returns e.g. `Some(200)` for `"HTTP/1.1 200 OK\r\n..."`, `None` when the
/// payload does not start with a well-formed status line.
pub fn parse_status_code(head: &str) -> Option<u16> {
    let line = head.lines().next()?;
    let mut parts = line.split_whitespace();
    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    parts.next()?.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_code_extracts_code() {
        assert_eq!(
            parse_status_code("HTTP/1.1 200 OK\r\ncontent-type: text/html\r\n\r\n"),
            Some(200)
        );
        assert_eq!(
            parse_status_code("HTTP/1.0 404 Not Found\r\n\r\n"),
            Some(404)
        );
        assert_eq!(parse_status_code("<html>not http</html>"), None);
        assert_eq!(parse_status_code(""), None);
    }
}
