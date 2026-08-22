use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use dsh_launcher::{load_settings, save_settings};
use serde::Serialize;

use crate::probe::{dsh_signature_ok, kill_port_owner, terminate_tree};
use crate::state::{LauncherState, StatusPayload};
use crate::supervisor::run_launcher;

// ----- Tauri commands exposed to the setup/error page -----

#[tauri::command]
pub fn get_status(state: tauri::State<LauncherState>) -> StatusPayload {
    state.status.lock().unwrap().clone()
}

/// Abort any running supervision and clear process bookkeeping. Used by the
/// save/restart commands before spawning a fresh supervisor run. Bumping the
/// generation makes the old supervisor thread exit quietly instead of racing
/// us with another spawn.
fn stop_current_run(state: &LauncherState) {
    state.generation.fetch_add(1, Ordering::SeqCst);
    if let Some(pgid) = state.pgid.lock().unwrap().take() {
        terminate_tree(pgid);
    }
    state.stopping.store(false, Ordering::SeqCst);
}

/// Persist Tier-0 settings (dsh path + optional port, PLAN 门禁 A/B) and
/// restart the supervised lifecycle. `path` may be empty to reuse the saved
/// path (used by the in-app 重新启动 button).
#[tauri::command]
pub fn select_dsh(
    app: tauri::AppHandle,
    state: tauri::State<LauncherState>,
    path: Option<String>,
    port: Option<u16>,
) -> Result<(), String> {
    let mut settings = load_settings(&state.settings_path);

    if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
        let p = PathBuf::from(path.trim());
        if !p.is_file() {
            return Err(format!("路径不是可执行文件: {}", p.display()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let executable = std::fs::metadata(&p)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false);
            if !executable {
                return Err(format!("文件没有可执行权限（chmod +x）: {}", p.display()));
            }
        }
        settings.dsh_path = Some(p);
        settings.source = "user".into();
    }

    if let Some(port) = port {
        if port < 1024 {
            return Err(format!("端口需 ≥ 1024（当前 {port}）"));
        }
        settings.port = port;
    }

    save_settings(&state.settings_path, &settings).map_err(|e| e.to_string())?;
    dsh_launcher::logging::info(&format!(
        "已保存设置: dsh_path={} port={}",
        settings
            .dsh_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unchanged>".into()),
        settings.port
    ));

    stop_current_run(&state);
    std::thread::spawn(move || run_launcher(app));
    Ok(())
}

/// Restart dsh from the error / stopped page (reuses the saved path).
///
/// 两种情况：
/// - launcher 拥有 dsh（pgid 有值）：`stop_current_run` 直接杀掉受监督子进程树，
///   随后 `run_launcher` 重生即可（原行为）。
/// - 外部 dsh（pgid 为 None，例如之前已有一个 dsh 在端口上、launcher 只是连接）：
///   launcher 没有进程句柄，于是先按端口清理占用进程，等端口释放后再 `run_launcher`，
///   此时端口已空，会走 normal path 由 launcher 自己拉起并接管 dsh。
/// 无论哪种，启动时的健康探测（supervise_once 的 fast path）都保持不变。
#[tauri::command]
pub fn restart_dsh(app: tauri::AppHandle, state: tauri::State<LauncherState>) -> Result<(), String> {
    dsh_launcher::logging::info("用户请求重新启动 dsh");
    // 先判断 launcher 是否拥有 dsh（pgid 有值 = 自己拉起的受监督子进程）。
    let owned = state.pgid.lock().unwrap().is_some();
    if !owned {
        let port = load_settings(&state.settings_path).port;
        let killed = kill_port_owner(port);
        // 等待端口释放，避免 run_launcher 立刻又命中 fast path 重新连接旧实例。
        let mut deadline = Instant::now() + Duration::from_secs(10);
        while dsh_signature_ok(port) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(200));
        }
        // 兜底：TERM 后仍响应则强杀。
        if dsh_signature_ok(port) {
            for pid in &killed {
                let _ = Command::new("kill").args(["-KILL", "--", &pid.to_string()]).status();
            }
            deadline = Instant::now() + Duration::from_secs(5);
            while dsh_signature_ok(port) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    stop_current_run(&state);
    std::thread::spawn(move || run_launcher(app));
    Ok(())
}

/// Outcome returned to the injected archived-session panel for one batch op.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveOpResponse {
    /// Per-id outcome (changed archive set, pruned refs, deleted dirs, …).
    report: dsh_launcher::ArchiveOpsReport,
    /// Always `false`: the launcher no longer force-restarts dsh after an
    /// archive op. The injected panel keeps its own list correct in place via
    /// an override cache, and dsh's in-memory/sidebar state resyncs on the
    /// next natural restart (deliberately avoided so running sessions are not
    /// interrupted by a page reload).
    restart_triggered: bool,
}

/// 撤销归档：把选中的会话从 dsh 的归档集合中移除。
///
/// 注意：不再强制重启 dsh。直接改磁盘后，注入脚本里的覆盖缓存已能让归档面板
/// 就地保持正确；dsh 的内存/侧边栏状态会在下一次自然重启时对齐。强制重启会
/// 重载整个页面、打断正在运行的对话，因此这里刻意不做。
#[tauri::command]
pub fn restore_archived_sessions(
    ids: Vec<String>,
) -> Result<ArchiveOpResponse, String> {
    let home = dsh_launcher::dsh_home();
    let report = dsh_launcher::restore_archived(&home, &ids)?;
    Ok(ArchiveOpResponse {
        report,
        restart_triggered: false,
    })
}

/// 物理删除：从磁盘移除选中的会话目录，并从归档集合与工作区成员中清除。
///
/// 同样不再强制重启 dsh（理由同上）。
#[tauri::command]
pub fn delete_archived_sessions(
    ids: Vec<String>,
) -> Result<ArchiveOpResponse, String> {
    let home = dsh_launcher::dsh_home();
    let report = dsh_launcher::delete_archived(&home, &ids)?;
    Ok(ArchiveOpResponse {
        report,
        restart_triggered: false,
    })
}

