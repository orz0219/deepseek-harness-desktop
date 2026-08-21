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

pub mod dsh_launch;
pub mod locate_dsh;

pub use dsh_launch::{
    AppSettings, LaunchPlan, DEFAULT_PORT, build_env_snapshot, build_launch_plan, gui_url,
    load_settings, resolve_node, save_settings, settings_dir, settings_path,
};
pub use locate_dsh::locate;

use std::path::PathBuf;

/// Lifecycle state of the supervised `dsh web` child (PLAN §6, supervisor).
///
/// Rust owns only this coarse state machine — business errors are surfaced by
/// dsh's own error page; Rust never re-implements a second status layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Child spawned, readiness probe not yet satisfied.
    Starting,
    /// Readiness probe satisfied; webview navigated to dsh.
    Ready,
    /// Child exited unexpectedly; one restart scheduled.
    Restarting,
    /// Child exited (or failed after restart); show error page.
    Exited,
    /// App is tearing down; sending SIGTERM to the child.
    Stopping,
}

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
