use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;

use serde::Serialize;

/// Serializable view of a discovered dsh candidate (for the setup UI).
#[derive(Debug, Clone, Serialize)]
pub struct CandidateView {
    pub(crate) executable: String,
    pub(crate) version: String,
    pub(crate) source: String,
}

/// State pushed to the webview's status page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub(crate) state: String,
    pub(crate) message: Option<String>,
    pub(crate) port: u16,
    pub(crate) url: String,
    pub(crate) candidates: Vec<CandidateView>,
}

/// Shared launcher state (managed by Tauri).
pub struct LauncherState {
    pub(crate) status: Mutex<StatusPayload>,
    /// Process group id of the supervised dsh child (for tree teardown).
    pub(crate) pgid: Mutex<Option<i32>>,
    /// Set when the app is tearing down, so supervisors stop watching/retrying.
    pub(crate) stopping: AtomicBool,
    /// Bumped whenever a (re)start is requested externally (save/restart
    /// command); running supervisors observe it and abort quietly. This closes
    /// the save-settings race where an old supervisor could spawn a competing
    /// dsh child after its teardown.
    pub(crate) generation: AtomicU64,
    /// App-exit teardown runs once (ExitRequested / Exit may both fire).
    pub(crate) torn_down: AtomicBool,
    pub(crate) settings_path: PathBuf,
    /// Origin of the bundled UI page ("http://localhost:<port>" in dev,
    /// "tauri://localhost" in prod). Error pages live here; after dsh took
    /// over the view we navigate BACK here to surface any problem.
    pub(crate) launcher_origin: Mutex<String>,
}

/// Outcome of one supervised lifecycle attempt.
pub enum Lifecycle {
    /// Everything healthy until an external stop / newer generation took over.
    Aborted,
    /// Could not reach READY (spawn error / timeout / port clash). The error
    /// page has already been shown; per PLAN 门禁 E these never auto-restart.
    StartFailed,
    /// Our child exited with this status (clean or not).
    Exited(std::process::ExitStatus),
    /// A previously-healthy dsh (ours or pre-existing) stopped answering.
    ConnectionLost(String),
}

/// Current state kind strings (kept in sync with ui/index.html).
pub const ST_STARTING: &str = "starting";
pub const ST_READY: &str = "ready";
pub const ST_MISSING_DSH: &str = "missing-dsh";
pub const ST_ERROR: &str = "error";

