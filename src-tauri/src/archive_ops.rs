//! Pure, Tauri-free session-archive maintenance for dsh's on-disk storage.
//!
//! dsh exposes **no** RPC for un-archiving or physically deleting sessions
//! (verified against the installed `dsh-host-apiproxy` method registry: only
//! `workspace.archiveSession` exists; the archived ids live in the registry's
//! durable state and are never filtered in memory). So the launcher performs
//! these operations by editing dsh's durable storage directly and then asking
//! dsh to re-bootstrap (a restart, owned by the launcher):
//!
//!   * `~/.dsh/storages/workspace.json` — `global.archivedSessionIds` and the
//!     `tables.workspaces[*].sessionIds` membership.
//!   * `~/.dsh/sessions/<workspace-slug>/session-<uuid>/` — physical session
//!     directories (removed only on *physical delete*).
//!
//! Because dsh caches the registry in memory for its lifetime, edits made here
//! only take effect once dsh restarts. The caller decides whether/when to
//! restart; this module stays restart-agnostic and testable.
//!
//! Safety invariants:
//!   * session ids are strictly validated (`^[A-Za-z0-9-]+$` plus a required
//!     `session-` prefix) before they are used to build any filesystem path,
//!     so a hostile id can never traverse out of the storage tree;
//!   * `workspace.json` is written atomically (temp file + rename) and a
//!     one-shot backup (`.dsh-archive-ops.bak`) is kept before the first edit;
//!   * only the keys we own (`archivedSessionIds`, workspaces `sessionIds`)
//!     are touched; everything else is preserved verbatim.

use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Name of the workspace registry file under `<home>/storages/`.
const WORKSPACE_STORAGE_NAME: &str = "workspace.json";
/// Name of the sessions root directory under `<home>/`.
const SESSIONS_ROOT_NAME: &str = "sessions";
/// Backup suffix kept beside `workspace.json` before the first edit.
const BACKUP_SUFFIX: &str = ".dsh-archive-ops.bak";

// workspace.json structural keys (Node/dsh uses these exact names).
const GLOBAL_KEY: &str = "global";
const ARCHIVED_KEY: &str = "archivedSessionIds";
const TABLES_KEY: &str = "tables";
const WORKSPACES_KEY: &str = "workspaces";
const SESSION_IDS_KEY: &str = "sessionIds";

/// Outcome of one batch archive-maintenance operation (unarchive or delete).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveOpsReport {
    /// Number of session ids accepted for processing.
    pub processed: usize,
    /// Number of ids actually removed from `archivedSessionIds`.
    pub changed_archive: usize,
    /// Number of stale `sessionIds` references pruned from workspace tables.
    pub pruned_session_refs: usize,
    /// Number of session directories physically removed (delete only).
    pub deleted_dirs: usize,
    /// Ids we could not find a session directory for (delete only).
    pub missing_dirs: Vec<String>,
    /// Non-fatal diagnostics (e.g. a directory that failed to remove).
    pub warnings: Vec<String>,
}

/// Resolve the effective dsh home (storage root).
///
/// Precedence mirrors dsh's own `dsh-home-paths`: `$DSH_HOME` (non-blank)
/// first, then `~/.dsh`. The launcher spawns dsh with the same resolved
/// `HOME`, so this matches the running instance's storage location.
pub fn dsh_home() -> PathBuf {
    if let Ok(v) = std::env::var("DSH_HOME") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
        .join(".dsh")
}

/// Absolute path to the workspace registry file for a given dsh home.
pub fn workspace_storage_path(home: &Path) -> PathBuf {
    home.join("storages").join(WORKSPACE_STORAGE_NAME)
}

/// Absolute path to the sessions root for a given dsh home.
pub fn sessions_root(home: &Path) -> PathBuf {
    home.join(SESSIONS_ROOT_NAME)
}

/// Validate that `id` is a safe session id we may use to build paths.
fn is_safe_session_id(id: &str) -> bool {
    id.starts_with("session-")
        && id.len() <= 128
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !id.contains("..")
}

fn load_workspace_doc(path: &Path) -> Result<serde_json::Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("无法读取 {}: {e}", path.display()))?;
    let doc: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("{} 不是合法 JSON: {e}", path.display()))?;
    if !doc.get(GLOBAL_KEY).map(|g| g.is_object()).unwrap_or(false) {
        return Err(format!("{} 缺少 global 对象", path.display()));
    }
    if !doc
        .get(GLOBAL_KEY)
        .and_then(|g| g.get(ARCHIVED_KEY))
        .map(|a| a.is_array())
        .unwrap_or(false)
    {
        return Err(format!("{} 缺少 archivedSessionIds 数组", path.display()));
    }
    Ok(doc)
}

/// Keep a one-shot backup of `path` next to it if none exists yet.
fn ensure_backup(path: &Path) -> Result<(), String> {
    let backup = PathBuf::from(format!("{}{}", path.display(), BACKUP_SUFFIX));
    if backup.exists() {
        return Ok(());
    }
    fs::copy(path, &backup).map_err(|e| format!("备份 {} 失败: {e}", path.display()))?;
    Ok(())
}

/// Atomically replace `path` with `doc` (temp file in the same dir + rename).
fn save_workspace_doc_atomic(path: &Path, doc: &serde_json::Value) -> Result<(), String> {
    let bytes =
        serde_json::to_vec_pretty(doc).map_err(|e| format!("序列化 workspace 存储失败: {e}"))?;
    let dir = path
        .parent()
        .ok_or_else(|| format!("{} 无父目录", path.display()))?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        WORKSPACE_STORAGE_NAME,
        std::process::id()
    ));
    fs::write(&tmp, &bytes).map_err(|e| format!("写入临时文件 {} 失败: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("原子替换 {} 失败: {e}", path.display())
    })?;
    Ok(())
}

/// Current archived id set (a defensive snapshot for validation).
fn archived_set(doc: &serde_json::Value) -> BTreeSet<String> {
    doc.get(GLOBAL_KEY)
        .and_then(|g| g.get(ARCHIVED_KEY))
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Remove eligible ids from `global.archivedSessionIds`.
///
/// Returns the number of ids that were present and removed. Ids that are
/// absent are simply skipped (idempotent). Harmless no-ops are allowed and
/// counted as not-removed.
fn remove_from_archived(doc: &mut serde_json::Value, ids: &[String]) -> Result<usize, String> {
    let mut removed = 0usize;
    let archived = doc
        .get_mut(GLOBAL_KEY)
        .and_then(|g| g.get_mut(ARCHIVED_KEY))
        .and_then(|a| a.as_array_mut())
        .ok_or_else(|| "workspace 存储缺少 archivedSessionIds".to_string())?;
    let target: BTreeSet<&str> = ids.iter().map(|s| s.as_str()).collect();
    archived.retain(|v| {
        let keep = v.as_str().map(|s| !target.contains(s)).unwrap_or(true);
        if !keep {
            removed += 1;
        }
        keep
    });
    Ok(removed)
}

/// Remove `ids` from every workspace table's `sessionIds` array.
///
/// Physical deletion leaves stale session ids in the workspace membership
/// (dsh's registry keeps them; a deleted session has no header, so they are
/// inert but dirty). Pruning keeps the storage tidy. Returns the number of
/// references removed.
fn prune_workspace_session_refs(
    doc: &mut serde_json::Value,
    ids: &[String],
) -> Result<usize, String> {
    let mut pruned = 0usize;
    let target: BTreeSet<&str> = ids.iter().map(|s| s.as_str()).collect();
    let Some(workspaces) = doc
        .get_mut(TABLES_KEY)
        .and_then(|t| t.get_mut(WORKSPACES_KEY))
        .and_then(|w| w.as_object_mut())
    else {
        return Ok(0);
    };
    for record in workspaces.values_mut() {
        let Some(list) = record
            .get_mut(SESSION_IDS_KEY)
            .and_then(|s| s.as_array_mut())
        else {
            continue;
        };
        list.retain(|v| {
            let keep = v.as_str().map(|s| !target.contains(s)).unwrap_or(true);
            if !keep {
                pruned += 1;
            }
            keep
        });
    }
    Ok(pruned)
}

/// Locate every directory physically holding `id` under `sessions_root`.
///
/// dsh stores sessions as `<root>/<workspace-slug>/session-<uuid>/`; older or
/// edge layouts may place them directly at `<root>/session-<uuid>/`. We check
/// both the root level and one level deeper. Only directories actually named
/// `session-<id>` are considered; anything else is ignored.
fn find_session_dirs(root: &Path, id: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let target = match safe_dir_name(id) {
        Some(n) => n,
        None => return found,
    };
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return found,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.file_name().map(|n| n == target.as_str()).unwrap_or(false) && p.is_dir() {
            found.push(p);
            continue;
        }
        if p.is_dir() {
            let child = p.join(&target);
            if child.is_dir() {
                found.push(child);
            }
        }
    }
    found
}

/// The directory/segment name for a session id; `None` if the id is unsafe.
fn safe_dir_name(id: &str) -> Option<String> {
    if is_safe_session_id(id) {
        Some(id.to_string())
    } else {
        None
    }
}

/// Reject unsafe ids early; returns a short user-facing reason if invalid.
fn validate_ids(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Err("未选择任何会话".to_string());
    }
    for id in ids {
        if !is_safe_session_id(id) {
            return Err(format!("非法的会话 id: {id}"));
        }
    }
    Ok(())
}

/// **Unarchive** — remove ids from the archived set (no disk deletion).
///
/// dsh restores an un-archived session to its original sidebar slot on the
/// next boot because archiving never removes it from workspace membership.
pub fn restore_archived(home: &Path, ids: &[String]) -> Result<ArchiveOpsReport, String> {
    validate_ids(ids)?;
    let path = workspace_storage_path(home);
    let mut doc = load_workspace_doc(&path)?;

    // Mirror the archived set before edit; only proceed for ids actually
    // archived (defensive: the UI only lists archived ones, but never trust
    // the input blindly).
    let archived = archived_set(&doc);
    let eligible: Vec<String> = ids
        .iter()
        .filter(|id| archived.contains(*id))
        .cloned()
        .collect();
    if eligible.is_empty() {
        return Ok(ArchiveOpsReport {
            processed: ids.len(),
            ..Default::default()
        });
    }

    let changed = remove_from_archived(&mut doc, &eligible)?;
    ensure_backup(&path)?;
    save_workspace_doc_atomic(&path, &doc)?;
    Ok(ArchiveOpsReport {
        processed: ids.len(),
        changed_archive: changed,
        ..Default::default()
    })
}

/// **Physical delete** — remove archived ids, prune membership, and delete the
/// session directories from disk.
pub fn delete_archived(home: &Path, ids: &[String]) -> Result<ArchiveOpsReport, String> {
    validate_ids(ids)?;
    let path = workspace_storage_path(home);
    let mut doc = load_workspace_doc(&path)?;

    let archived = archived_set(&doc);
    let eligible: Vec<String> = ids
        .iter()
        .filter(|id| archived.contains(*id))
        .cloned()
        .collect();

    let mut report = ArchiveOpsReport {
        processed: ids.len(),
        ..Default::default()
    };

    // Remove directories first (failures are reported, not fatal).
    let root = sessions_root(home);
    for id in &eligible {
        let dirs = find_session_dirs(&root, id);
        if dirs.is_empty() {
            report.missing_dirs.push(id.clone());
            continue;
        }
        for dir in dirs {
            match fs::remove_dir_all(&dir) {
                Ok(()) => report.deleted_dirs += 1,
                Err(e) => report
                    .warnings
                    .push(format!("删除 {} 失败: {e}", dir.display())),
            }
        }
    }

    if eligible.is_empty() {
        return Ok(report);
    }

    let changed = remove_from_archived(&mut doc, &eligible)?;
    report.changed_archive = changed;
    let pruned = prune_workspace_session_refs(&mut doc, &eligible)?;
    report.pruned_session_refs = pruned;

    ensure_backup(&path)?;
    save_workspace_doc_atomic(&path, &doc)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc(archived: &[&str], tables: &[(&str, &[&str])]) -> serde_json::Value {
        let mut doc = serde_json::json!({
            "unit": { "name": "workspace", "version": 2 },
            "global": {
                "initialized": true,
                "workspaceIds": ["w1", "w2"],
                "archivedSessionIds": archived
            },
            "tables": { "workspaces": {} }
        });
        let workspaces = doc["tables"]["workspaces"].as_object_mut().unwrap();
        for (id, sessions) in tables {
            workspaces.insert(
                (*id).to_string(),
                serde_json::json!({
                    "path": format!("/tmp/{}", id),
                    "title": id,
                    "sessionIds": sessions,
                    "createdAt": "t",
                    "updatedAt": "t"
                }),
            );
        }
        doc
    }

    #[test]
    fn dsh_home_prefers_env() {
        std::env::set_var("DSH_HOME", "/tmp/x");
        assert_eq!(dsh_home(), PathBuf::from("/tmp/x"));
        std::env::remove_var("DSH_HOME");
    }

    #[test]
    fn remove_from_archived_filters_only_targets() {
        let mut doc = make_doc(
            &["session-a", "session-b", "session-c"],
            &[("w1", &["session-a", "session-b"])],
        );
        let removed = remove_from_archived(
            &mut doc,
            &["session-b".to_string(), "session-nope".to_string()],
        )
        .unwrap();
        assert_eq!(removed, 1);
        let archived: Vec<&str> = doc["global"]["archivedSessionIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(archived, vec!["session-a", "session-c"]);
        // untouched structure preserved
        assert_eq!(doc["global"]["workspaceIds"][1], "w2");
        assert_eq!(
            doc["tables"]["workspaces"]["w1"]["sessionIds"][1],
            "session-b"
        );
    }

    #[test]
    fn prune_removes_membership_references() {
        let mut doc = make_doc(
            &["session-a", "session-b"],
            &[("w1", &["session-a", "session-x"]), ("w2", &["session-b"])],
        );
        let pruned = prune_workspace_session_refs(&mut doc, &["session-a".to_string()]).unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(
            doc["tables"]["workspaces"]["w1"]["sessionIds"][0],
            "session-x"
        );
        // untouched workspace
        assert_eq!(
            doc["tables"]["workspaces"]["w2"]["sessionIds"][0],
            "session-b"
        );
    }

    #[test]
    fn unsafe_ids_rejected() {
        assert!(restore_archived(Path::new("/tmp"), &["../../etc/passwd".to_string()]).is_err());
        assert!(delete_archived(Path::new("/tmp"), &["not-a-session".to_string()]).is_err());
        assert!(restore_archived(Path::new("/tmp"), &[]).is_err());
        assert!(is_safe_session_id("session-060f2d73-6ee7"));
    }

    #[test]
    fn restore_persists_atomically_and_backs_up() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let dir = home.join("storages");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(WORKSPACE_STORAGE_NAME);
        let doc = make_doc(&["session-a", "session-b"], &[("w1", &["session-a"])]);
        fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

        let report =
            restore_archived(home, &["session-a".to_string(), "session-b".to_string()]).unwrap();
        assert_eq!(report.changed_archive, 2);
        assert_eq!(report.processed, 2);

        // reload and confirm
        let reloaded = load_workspace_doc(&path).unwrap();
        let archived: Vec<&str> = reloaded["global"]["archivedSessionIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(archived.is_empty());
        // workspace membership untouched by restore
        assert_eq!(
            reloaded["tables"]["workspaces"]["w1"]["sessionIds"][0],
            "session-a"
        );
        // backup exists
        assert!(Path::new(&format!("{}{}", path.display(), BACKUP_SUFFIX)).exists());
    }

    #[test]
    fn delete_removes_dirs_and_prunes() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let dir = home.join("storages");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(WORKSPACE_STORAGE_NAME);
        let doc = make_doc(
            &["session-a", "session-b"],
            &[("w1", &["session-a", "session-b"])],
        );
        fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

        // build fake session dirs: one per workspace-level layout
        let root = sessions_root(home);
        let ws = root.join("--tmp-w1--");
        fs::create_dir_all(ws.join("session-a")).unwrap();
        fs::create_dir_all(ws.join("session-b")).unwrap();
        fs::create_dir_all(root.join("session-a")).unwrap(); // root-level duplicate

        let report = delete_archived(home, &["session-a".to_string()]).unwrap();
        assert_eq!(report.deleted_dirs, 2, "both layouts found and removed");
        assert_eq!(report.missing_dirs.len(), 0);
        assert_eq!(report.changed_archive, 1);
        assert_eq!(report.pruned_session_refs, 1);
        assert!(!ws.join("session-a").exists());
        assert!(!root.join("session-a").exists());
        assert!(ws.join("session-b").exists(), "unrelated session untouched");

        // b is now missing when we try to delete it (already pruned in file,
        // but no dirs to remove -> reported as missing but still valid)
        let report2 = delete_archived(home, &["session-b".to_string()]).unwrap();
        assert_eq!(report2.deleted_dirs, 1);
        assert_eq!(report2.changed_archive, 1);
        assert!(report2.missing_dirs.is_empty());
    }

    #[test]
    fn delete_tolerates_missing_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let dir = home.join("storages");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(WORKSPACE_STORAGE_NAME);
        let doc = make_doc(&["session-ghost"], &[]);
        fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()).unwrap();

        let report = delete_archived(home, &["session-ghost".to_string()]).unwrap();
        assert_eq!(report.missing_dirs, vec!["session-ghost"]);
        assert_eq!(report.changed_archive, 1, "id still removed from archive");
        let reloaded = load_workspace_doc(&path).unwrap();
        let archived: Vec<&str> = reloaded["global"]["archivedSessionIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(archived.is_empty());
    }
}
