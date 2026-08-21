//! Constructing the `dsh web` launch command and managing launcher settings
//! (PLAN §3 / §5 门禁 B-D / §6 Slice 1A, 1B).
//!
//! The command follows dsh's *current* CLI (see `FEASIBILITY.md`):
//!   `<node> <dsh> web --host 127.0.0.1 --port <port> --no-open [--profile <p>]`
//! `--no-open` is required so dsh does not spawn its own system browser; the
//! Tauri webview is our browser.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::DshCandidate;

/// Default loopback port (matches dsh-desktop-launcher's default `url`).
pub const DEFAULT_PORT: u16 = 3080;

/// Default bind host (loopback-only trust fence, PLAN §6).
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// Launcher-owned settings (NOT dsh's runtime data — PLAN 门禁 D).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    /// Tier 0: user-chosen dsh executable (absolute or ~/...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsh_path: Option<PathBuf>,
    /// Optional explicit node runtime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_path: Option<PathBuf>,
    /// Listen port dsh binds (known value; we poll it, no discovery).
    #[serde(default = "default_port")]
    pub port: u16,
    /// Optional `--profile` passed to `dsh web`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Where the active dsh_path came from (diagnostics).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            dsh_path: None,
            node_path: None,
            port: DEFAULT_PORT,
            profile: None,
            source: String::new(),
        }
    }
}

/// A fully-resolved command to spawn (program + args + env + cwd).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: PathBuf,
}

/// The node runtime the launcher will use: explicit setting wins, then the
/// candidate's resolved node (from the dsh shebang), then bare `node`.
pub fn resolve_node(settings: &AppSettings, candidate: &DshCandidate) -> Option<PathBuf> {
    settings
        .node_path
        .clone()
        .or_else(|| candidate.node.clone())
}

/// Build the launch plan for a resolved candidate.
pub fn build_launch_plan(
    candidate: &DshCandidate,
    settings: &AppSettings,
    home: &Path,
) -> LaunchPlan {
    let node = resolve_node(settings, candidate);

    let (program, mut args) = match &node {
        Some(node) => (node.clone(), vec![candidate.executable.to_string_lossy().into_owned()]),
        None => (
            candidate.executable.clone(),
            Vec::new(),
        ),
    };
    args.push("web".into());
    args.push("--host".into());
    args.push(DEFAULT_HOST.into());
    args.push("--port".into());
    args.push(settings.port.to_string());
    args.push("--no-open".into());
    if let Some(profile) = &settings.profile {
        args.push("--profile".into());
        args.push(profile.clone());
    }

    LaunchPlan {
        program,
        args,
        env: build_env_snapshot(home),
        cwd: home.to_path_buf(),
    }
}

/// Build the spawn environment snapshot (PLAN 验收点 2).
///
/// dsh may spawn git/python/bash as children, so a correct PATH (and the
/// locations parsed from PATH files) is essential — especially in a GUI context
/// where the app does not inherit the shell PATH. We record exactly what we pass
/// so the launcher can log it for diagnostics.
pub fn build_env_snapshot(home: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // HOME is mandatory: dsh stores data under ~/.dsh (PLAN 门禁 D).
    env.insert("HOME".into(), home.to_string_lossy().into_owned());

    // PATH: current process PATH augmented with directories parsed from
    // /etc/paths, /etc/paths.d/*, ~/.zprofile (Tier 2 discovery).
    let augmented = crate::locate_dsh::augmented_path(home);
    env.insert("PATH".into(), augmented);

    // SHELL: dsh and its children sometimes consult it.
    env.insert(
        "SHELL".into(),
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()),
    );

    // LANG: locale for node/dsh output; default to UTF-8.
    env.insert(
        "LANG".into(),
        std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".into()),
    );

    // NODE_PATH / NVM_DIR: only forwarded when already present, so we do not
    // invent a broken module path.
    if let Ok(v) = std::env::var("NODE_PATH") {
        if !v.is_empty() {
            env.insert("NODE_PATH".into(), v);
        }
    }
    if let Ok(v) = std::env::var("NVM_DIR") {
        if !v.is_empty() {
            env.insert("NVM_DIR".into(), v);
        }
    }

    env.insert("TERM".into(), "xterm-256color".into());
    env
}

/// GUI URL the webview navigates to once the readiness probe succeeds.
pub fn gui_url(port: u16) -> String {
    format!("http://{}:{}", DEFAULT_HOST, port)
}

/// Settings directory: `~/Library/Application Support/<identifier>`.
pub fn settings_dir(identifier: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join(identifier)
}

/// Settings file path inside the settings directory.
pub fn settings_path(identifier: &str) -> PathBuf {
    settings_dir(identifier).join("settings.json")
}

/// Load settings, merging onto defaults. Missing file -> defaults.
pub fn load_settings(path: &Path) -> AppSettings {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

/// Persist settings (creating the directory if needed).
pub fn save_settings(path: &Path, settings: &AppSettings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    #[test]
    fn default_settings_use_port_3080() {
        let s = AppSettings::default();
        assert_eq!(s.port, DEFAULT_PORT);
        assert_eq!(s.port, 3080);
    }

    #[test]
    fn launch_plan_includes_no_open_and_loopback() {
        let home = Path::new("/Users/tester");
        let candidate = DshCandidate {
            executable: PathBuf::from("/opt/homebrew/bin/dsh"),
            node: Some(PathBuf::from("/opt/homebrew/bin/node")),
            version: "0.1.0-rc.7".into(),
            source: Source::UserSpecified,
        };
        let settings = AppSettings {
            dsh_path: Some(PathBuf::from("/opt/homebrew/bin/dsh")),
            node_path: None,
            port: 3080,
            profile: None,
            source: "test".into(),
        };
        let plan = build_launch_plan(&candidate, &settings, home);
        assert_eq!(plan.program, PathBuf::from("/opt/homebrew/bin/node"));
        assert_eq!(plan.args[0], "/opt/homebrew/bin/dsh");
        assert!(plan.args.contains(&"web".to_string()));
        assert!(plan.args.contains(&"--host".to_string()));
        assert!(plan.args.contains(&"127.0.0.1".to_string()));
        assert!(plan.args.contains(&"--port".to_string()));
        assert!(plan.args.contains(&"3080".to_string()));
        assert!(plan.args.contains(&"--no-open".to_string()));
        assert_eq!(plan.cwd, home);
    }

    #[test]
    fn launch_plan_without_node_uses_shebang_executable() {
        let home = Path::new("/Users/tester");
        let candidate = DshCandidate {
            executable: PathBuf::from("/usr/local/bin/dsh"),
            node: None,
            version: "0.1.0".into(),
            source: Source::Scanned,
        };
        let settings = AppSettings::default();
        let plan = build_launch_plan(&candidate, &settings, home);
        assert_eq!(plan.program, PathBuf::from("/usr/local/bin/dsh"));
        assert!(!plan.args.contains(&"/usr/local/bin/dsh".to_string()));
    }

    #[test]
    fn launch_plan_honors_profile() {
        let home = Path::new("/Users/tester");
        let candidate = DshCandidate {
            executable: PathBuf::from("/x/dsh"),
            node: Some(PathBuf::from("/x/node")),
            version: "0.1.0".into(),
            source: Source::UserSpecified,
        };
        let settings = AppSettings {
            profile: Some("desktop".into()),
            ..Default::default()
        };
        let plan = build_launch_plan(&candidate, &settings, home);
        let i = plan.args.iter().position(|a| a == "--profile").unwrap();
        assert_eq!(plan.args[i + 1], "desktop");
    }

    #[test]
    fn env_snapshot_has_required_keys() {
        let home = Path::new("/Users/tester");
        let env = build_env_snapshot(home);
        assert_eq!(env.get("HOME").unwrap(), "/Users/tester");
        assert!(env.contains_key("PATH"));
        assert!(env.contains_key("SHELL"));
        assert!(env.contains_key("LANG"));
        assert!(env.contains_key("TERM"));
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = AppSettings {
            dsh_path: Some(PathBuf::from("/opt/homebrew/bin/dsh")),
            node_path: None,
            port: 4040,
            profile: Some("web".into()),
            source: "user".into(),
        };
        save_settings(&path, &s).unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded, s);
    }

    #[test]
    fn load_missing_settings_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.json");
        let loaded = load_settings(&path);
        assert_eq!(loaded.port, DEFAULT_PORT);
    }

    #[test]
    fn gui_url_uses_loopback() {
        assert_eq!(gui_url(3080), "http://127.0.0.1:3080");
    }
}
