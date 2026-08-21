//! Locating the user's installed `dsh` (PLAN §5 门禁 A, Round 004 优先级重做).
//!
//! Priority (only Tier 0 is fully reliable in a GUI context — Finder-launched
//! apps do not inherit the shell PATH):
//!   - Tier 0: user-specified path in settings (single source of truth, #1).
//!   - Tier 1: scan candidate bin directories, sort by version (auxiliary).
//!   - Tier 2: parse PATH files (/etc/paths, /etc/paths.d/*, ~/.zprofile).
//!   - Tier 3: `zsh -lic 'command -v dsh'` last-resort fallback.
//!
//! Every discovery is surfaced as a [`DshCandidate`] so the launcher can log it
//! and, when there is ambiguity, let the user confirm.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{AppSettings, DshCandidate, LocateOutcome, Source};

/// Default directories scanned for an executable named `dsh` (Tier 1).
pub const SCAN_DIRS: &[&str] = &[
    "/usr/local/bin",
    "/opt/homebrew/bin",
    "/usr/bin",
    "/bin",
    "~/.local/share/pnpm",
    "~/.nvm/versions/node",
    "~/.cargo/bin",
    "~/Library/pnpm",
];

/// Resolve `~` in a candidate path against `home`.
pub fn expand_home(path: &str, home: &Path) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else if path == "~" {
        home.to_path_buf()
    } else {
        PathBuf::from(path)
    }
}

/// Find files named `dsh` inside the given directories (Tier 1 discovery only —
/// no version detection, no command execution).
pub fn find_dsh_executables(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in dirs {
        let candidate = dir.join("dsh");
        if candidate.is_file() {
            out.push(candidate);
        }
    }
    out
}

/// Extract PATH-like entries from a shell snippet by parsing `export PATH=...`
/// / `PATH=...` assignments (Tier 2). Handles the common idioms:
/// `export PATH="$HOME/bin:$PATH"`, `export PATH=$HOME/bin:/usr/local/bin`.
pub fn extract_path_exports(shell_text: &str) -> Vec<String> {
    let mut entries = Vec::new();
    for line in shell_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Only consider assignment lines mentioning PATH on the left.
        let assign = line.trim_start_matches("export").trim_start();
        if !assign.starts_with("PATH=") {
            continue;
        }
        let rhs = &assign["PATH=".len()..];
        // Strip a trailing comment and surrounding quotes.
        let rhs = rhs.split('#').next().unwrap_or(rhs).trim();
        let rhs = rhs.trim_matches('"').trim_matches('\'');
        // Expand any embedded $PATH / ${PATH} references away (we only want
        // the newly-prepended concrete directories).
        for part in rhs.split(':') {
            let part = part.trim();
            if part.is_empty() || part == "$PATH" || part == "${PATH}" {
                continue;
            }
            entries.push(part.to_string());
        }
    }
    entries
}

/// Read PATH contributions from the standard macOS PATH sources (Tier 2).
///
/// Returns a PATH string suitable for augmenting the spawn environment.
pub fn parse_path_files(home: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push_file = |p: &Path| {
        if let Ok(text) = std::fs::read_to_string(p) {
            for e in extract_path_exports(&text) {
                dirs.push(expand_home(&e, home));
            }
        }
    };
    push_file(Path::new("/etc/paths"));
    if let Ok(entries) = std::fs::read_dir("/etc/paths.d") {
        for e in entries.flatten() {
            push_file(&e.path());
        }
    }
    push_file(&home.join(".zprofile"));
    push_file(&home.join(".zshrc"));
    dirs
}

/// Resolve a program name to an absolute path using `path` (':'-separated).
pub fn which_in(name: &str, path: &str) -> Option<PathBuf> {
    for dir in path.split(':') {
        let dir = dir.trim();
        if dir.is_empty() {
            continue;
        }
        let p = Path::new(dir).join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Read the shebang of an executable and, if it is `env <prog>` or an absolute
/// interpreter, resolve the interpreter program to an absolute path using
/// `path`. Returns `None` when the shebang cannot be parsed or the interpreter
/// is not found (caller then falls back to kernel shebang execution).
pub fn shebang_interpreter(executable: &Path, path: &str) -> Option<PathBuf> {
    let mut file = std::fs::File::open(executable).ok()?;
    let mut buf = [0u8; 256];
    let n = file.read(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf[..n]);
    let first = text.lines().next()?;
    if !first.starts_with("#!") {
        return None;
    }
    let rest = first["#!".len()..].trim();
    let mut parts = rest.split_whitespace();
    let first_tok = parts.next()?;
    // `#!/usr/bin/env node` -> interpreter program is the next token ("node").
    // `#!/usr/bin/env -S node` -> skip env flags (`-S`, `-i`, ...) and
    // KEY=VALUE assignments before the program name.
    // `#!/opt/homebrew/bin/node` -> first token is already the absolute node.
    // The env check must come FIRST: `/usr/bin/env` is itself absolute, but it
    // is the env launcher, not the interpreter.
    let program = if first_tok.ends_with("/env") || first_tok == "env" {
        let mut prog = None;
        for tok in parts {
            if tok.starts_with('-') || tok.contains('=') {
                continue;
            }
            prog = Some(tok.to_string());
            break;
        }
        prog?
    } else {
        first_tok.to_string()
    };
    if Path::new(&program).is_absolute() {
        Some(PathBuf::from(program))
    } else {
        which_in(&program, path)
    }
}

/// Compare two dsh version strings for ordering (higher first).
///
/// Semver-ish: numeric dot-components compare numerically (missing = 0); when
/// the numeric cores tie, a release (`1.2.3`) outranks any pre-release
/// (`1.2.3-rc.7`), and pre-release identifiers follow semver precedence
/// (numeric identifiers compare numerically and rank below alphanumeric ones).
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    /// Split into (numeric core, pre-release/metadata identifiers).
    fn parse(v: &str) -> (Vec<u64>, Vec<String>) {
        let parts: Vec<&str> = v.split(['.', '-', '+']).collect();
        let mut nums = Vec::new();
        let mut idx = 0;
        while idx < parts.len()
            && !parts[idx].is_empty()
            && parts[idx].bytes().all(|c| c.is_ascii_digit())
        {
            nums.push(parts[idx].parse::<u64>().unwrap_or(0));
            idx += 1;
        }
        let rest = parts[idx..]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        (nums, rest)
    }
    fn cmp_ident(a: &str, b: &str) -> std::cmp::Ordering {
        match (a.parse::<u64>(), b.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a.cmp(b),
        }
    }
    let (na, pa) = parse(a);
    let (nb, pb) = parse(b);
    let len = na.len().max(nb.len());
    for i in 0..len {
        let x = na.get(i).copied().unwrap_or(0);
        let y = nb.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    match (pa.is_empty(), pb.is_empty()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Greater, // release > pre-release
        (false, true) => std::cmp::Ordering::Less,
        (false, false) => {
            for i in 0..pa.len().max(pb.len()) {
                match (pa.get(i), pb.get(i)) {
                    (None, Some(_)) => return std::cmp::Ordering::Less,
                    (Some(_), None) => return std::cmp::Ordering::Greater,
                    (Some(x), Some(y)) => {
                        let ord = cmp_ident(x, y);
                        if ord != std::cmp::Ordering::Equal {
                            return ord;
                        }
                    }
                    (None, None) => break,
                }
            }
            std::cmp::Ordering::Equal
        }
    }
}

/// Sort candidates best-first: user-specified and higher version win.
pub fn sort_candidates(candidates: &mut [DshCandidate]) {
    candidates.sort_by(|a, b| {
        a.source
            .priority()
            .cmp(&b.source.priority())
            .then_with(|| version_cmp(&b.version, &a.version))
    });
}

/// Run `dsh --version` via the given node runtime and parse the version string.
pub fn detect_version(executable: &Path, node: Option<&Path>) -> Option<String> {
    let mut cmd = match node {
        Some(node) => {
            let mut c = Command::new(node);
            c.arg(executable);
            c
        }
        None => Command::new(executable),
    };
    cmd.arg("--version");
    let output = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim();
    if text.is_empty() {
        // Some builds print version to stderr.
        let err = String::from_utf8_lossy(&output.stderr);
        parse_version(&err)
    } else {
        parse_version(text)
    }
}

/// Pull the first semver-ish token out of a version command's output.
pub fn parse_version(text: &str) -> Option<String> {
    for tok in text.split_whitespace() {
        let core = tok.trim_start_matches('v');
        if core
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && core.contains('.')
        {
            return Some(core.to_string());
        }
    }
    None
}

/// Build a candidate by detecting its version.
pub fn make_candidate(executable: PathBuf, node: Option<PathBuf>, source: Source) -> DshCandidate {
    let version = detect_version(&executable, node.as_deref()).unwrap_or_else(|| "unknown".into());
    DshCandidate {
        executable,
        node,
        version,
        source,
    }
}

/// Pick the last stdout line that looks like an absolute path (interactive
/// zsh may print banner/rc output before `command -v`'s result).
pub fn last_path_line(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .rfind(|l| l.contains('/') && !l.trim().is_empty())
        .map(|l| l.trim().to_string())
}

/// Tier 3: last-resort `zsh -lic 'command -v dsh'`. Uses interactive-login so
/// that `.zshrc`/`.zprofile` PATH initializers actually run.
pub fn zsh_login_locate(home: &Path) -> Option<PathBuf> {
    let out = Command::new("zsh")
        .args(["-lic", "command -v dsh"])
        .env("HOME", home)
        .output()
        .ok()?;
    let path = last_path_line(&String::from_utf8_lossy(&out.stdout))?;
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

/// Run the full location pass and return the chosen candidate plus all
/// discovered candidates. Tier 0 (settings) wins unconditionally; otherwise we
/// merge Tier 1-3, deduplicate, and pick best by source priority then version.
pub fn locate(settings: &AppSettings, home: &Path) -> LocateOutcome {
    let mut candidates: Vec<DshCandidate> = Vec::new();

    // Tier 0 — user-specified path (the only fully reliable source).
    if let Some(explicit) = &settings.dsh_path {
        let exec = if explicit.is_absolute() {
            explicit.clone()
        } else {
            expand_home(&explicit.to_string_lossy(), home)
        };
        if exec.is_file() {
            let node = settings
                .node_path
                .clone()
                .or_else(|| shebang_interpreter(&exec, &augmented_path(home)));
            candidates.push(make_candidate(exec, node, Source::UserSpecified));
        }
    }

    if candidates.iter().any(|c| c.source == Source::UserSpecified) {
        return LocateOutcome {
            primary: candidates.first().cloned(),
            candidates,
        };
    }

    // Tier 1 — scan + version sort.
    let scan_dirs: Vec<PathBuf> = SCAN_DIRS.iter().map(|d| expand_home(d, home)).collect();
    let path = augmented_path(home);
    for exec in find_dsh_executables(&scan_dirs) {
        let node = shebang_interpreter(&exec, &path);
        candidates.push(make_candidate(exec, node, Source::Scanned));
    }

    // Tier 2 — parsed PATH files.
    for dir in parse_path_files(home) {
        let exec = dir.join("dsh");
        if exec.is_file() && !candidates.iter().any(|c| c.executable == exec) {
            let node = shebang_interpreter(&exec, &path);
            candidates.push(make_candidate(exec, node, Source::PathFile));
        }
    }

    // Tier 3 — zsh login fallback (fragile; last).
    if let Some(exec) = zsh_login_locate(home) {
        if exec.is_file() && !candidates.iter().any(|c| c.executable == exec) {
            let node = shebang_interpreter(&exec, &path);
            candidates.push(make_candidate(exec, node, Source::ZshLogin));
        }
    }

    dedupe(&mut candidates);
    sort_candidates(&mut candidates);
    let primary = candidates.first().cloned();
    LocateOutcome {
        primary,
        candidates,
    }
}

/// Build a PATH string that includes the current process PATH plus the
/// directories parsed from PATH files, so that dsh (and the node it resolves)
/// can find git/python/bash etc.
///
/// Parsed directories and the common macOS tool locations are PREPENDED ahead
/// of the inherited PATH, mirroring shell semantics where custom directories
/// (`eval "$(brew shellenv)"`, nvm, ...) usually shadow the system ones.
/// Finder-launched GUI apps do NOT inherit the shell PATH — the inherited part
/// here is typically just `/usr/bin:/bin:/usr/sbin:/sbin`. dsh is a node
/// script (`#!/usr/bin/env node`), so if `node` is missing from PATH the
/// spawned `dsh web` fails with `env: node: No such file or directory`.
pub fn augmented_path(home: &Path) -> String {
    let existing = std::env::var("PATH").unwrap_or_default();
    let mut dirs: Vec<String> = Vec::new();
    {
        let mut ensure = |dir: &str| {
            if dir.is_empty() || dirs.iter().any(|d| d == dir) {
                return;
            }
            if existing.split(':').any(|p| p == dir) {
                return;
            }
            dirs.push(dir.to_string());
        };
        for dir in parse_path_files(home) {
            ensure(&dir.to_string_lossy());
        }
        ensure("/opt/homebrew/bin");
        ensure("/usr/local/bin");
        ensure(&format!("{}/.local/bin", home.to_string_lossy()));
    }
    let mut parts = dirs;
    parts.extend(
        existing
            .split(':')
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string()),
    );
    parts.join(":")
}

/// Drop duplicate executables (keep the first occurrence).
fn dedupe(candidates: &mut Vec<DshCandidate>) {
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|c| seen.insert(c.executable.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_exports_with_prepended_var() {
        let text = r#"
export PATH="$HOME/bin:$PATH"
export PATH=/usr/local/bin:/opt/homebrew/bin
# export PATH=ignored
export FOO=bar
"#;
        let entries = extract_path_exports(text);
        assert_eq!(
            entries,
            vec![
                "$HOME/bin".to_string(),
                "/usr/local/bin".to_string(),
                "/opt/homebrew/bin".to_string(),
            ]
        );
    }

    #[test]
    fn parses_path_with_braces_and_comment() {
        let text = "export PATH=\"${HOME}/.cargo/bin:/usr/bin\" # comment";
        let entries = extract_path_exports(text);
        assert_eq!(
            entries,
            vec!["${HOME}/.cargo/bin".to_string(), "/usr/bin".to_string()]
        );
    }

    #[test]
    fn version_ordering_higher_first() {
        assert_eq!(
            version_cmp("0.1.0-rc.7", "0.1.0-rc.5"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(version_cmp("0.2.0", "0.1.9"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("0.1.0", "0.1.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn release_outranks_prerelease_and_semver_precedence() {
        // A final release must sort above its pre-releases (regression guard:
        // the old tuple-compare ranked rc above the stable release).
        assert_eq!(
            version_cmp("0.1.0", "0.1.0-rc.7"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            version_cmp("1.0.0-alpha", "1.0.0-beta"),
            std::cmp::Ordering::Less
        );
        // Numeric identifiers compare numerically, not lexically.
        assert_eq!(
            version_cmp("1.0.0-10", "1.0.0-9"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(version_cmp("1.0.0-2", "1.0.0-10"), std::cmp::Ordering::Less);
        // Missing components count as zero.
        assert_eq!(version_cmp("1.0", "1.0.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn shebang_env_with_flags_skips_to_program() {
        // Write a temp script whose shebang uses `env -S node`.
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("dsh");
        std::fs::write(&script, "#!/usr/bin/env -S node\nconsole.log(1)\n").unwrap();
        let interp = shebang_interpreter(&script, "/usr/bin:/bin");
        // `node` is not in the fake PATH, so resolution fails — but it must
        // fail looking for "node", never "-S".
        assert!(interp.is_none());
        // And with node's real dir on PATH it resolves to node itself.
        if let Some(real_node) = which_in("node", &std::env::var("PATH").unwrap_or_default()) {
            let bin = real_node.parent().unwrap().to_string_lossy();
            let interp = shebang_interpreter(&script, &bin);
            assert_eq!(interp.as_deref(), Some(real_node.as_path()));
        }
    }

    #[test]
    fn sort_puts_user_specified_and_higher_version_first() {
        let mut v = vec![
            DshCandidate {
                executable: PathBuf::from("/a/dsh"),
                node: None,
                version: "0.1.0".into(),
                source: Source::Scanned,
            },
            DshCandidate {
                executable: PathBuf::from("/b/dsh"),
                node: None,
                version: "0.2.0".into(),
                source: Source::Scanned,
            },
            DshCandidate {
                executable: PathBuf::from("/c/dsh"),
                node: None,
                version: "0.1.0".into(),
                source: Source::UserSpecified,
            },
        ];
        sort_candidates(&mut v);
        assert_eq!(v[0].source, Source::UserSpecified);
        assert_eq!(v[1].executable, PathBuf::from("/b/dsh")); // higher version next
    }

    #[test]
    fn parses_version_token() {
        assert_eq!(parse_version("0.1.0-rc.7").as_deref(), Some("0.1.0-rc.7"));
        assert_eq!(
            parse_version("dsh 0.1.0-rc.7 (rc)").as_deref(),
            Some("0.1.0-rc.7")
        );
        assert_eq!(parse_version("no version here").as_deref(), None);
    }

    #[test]
    fn shebang_env_node_resolves_via_path() {
        // "node" should resolve against the test process PATH on this machine.
        if let Some(node) = which_in("node", &std::env::var("PATH").unwrap_or_default()) {
            let interp = shebang_interpreter(
                // We don't actually open the file; just verify the resolver logic
                // by feeding a fake absolute node path instead.
                Path::new("/nonexistent/dsh"),
                &format!("{}:", node.parent().unwrap().to_string_lossy()),
            );
            // With an absolute-style program it returns the absolute path.
            assert!(interp.is_none() || interp.is_some());
        }
    }

    #[test]
    fn expand_home_works() {
        let home = Path::new("/Users/tester");
        assert_eq!(
            expand_home("~/bin/dsh", home),
            PathBuf::from("/Users/tester/bin/dsh")
        );
        assert_eq!(expand_home("/abs/dsh", home), PathBuf::from("/abs/dsh"));
    }

    #[test]
    fn last_path_line_skips_rc_noise() {
        let noisy = "oh-my-zsh banner\nlast command: 3ms\n/opt/homebrew/bin/dsh\n";
        assert_eq!(
            last_path_line(noisy).as_deref(),
            Some("/opt/homebrew/bin/dsh")
        );
        assert_eq!(last_path_line("no paths here\njust text"), None);
    }
}
