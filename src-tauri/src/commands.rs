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
pub fn restart_dsh(
    app: tauri::AppHandle,
    state: tauri::State<LauncherState>,
) -> Result<(), String> {
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
                let _ = Command::new("kill")
                    .args(["-KILL", "--", &pid.to_string()])
                    .status();
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
pub fn restore_archived_sessions(ids: Vec<String>) -> Result<ArchiveOpResponse, String> {
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
pub fn delete_archived_sessions(ids: Vec<String>) -> Result<ArchiveOpResponse, String> {
    let home = dsh_launcher::dsh_home();
    let report = dsh_launcher::delete_archived(&home, &ids)?;
    Ok(ArchiveOpResponse {
        report,
        restart_triggered: false,
    })
}

// ===== 右侧侧边栏相关命令 =====

/// Git diff 结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffResult {
    /// 变更文件列表
    pub files: Vec<GitFileDiff>,
    /// 概览信息
    pub summary: String,
}

/// 单个文件的 Git diff
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiff {
    /// 文件路径
    pub path: String,
    /// 状态：M(修改)、A(新增)、D(删除)、R(重命名)
    pub status: String,
    /// 新增行数
    pub additions: u32,
    /// 删除行数
    pub deletions: u32,
    /// diff 内容
    pub diff: String,
}

/// 获取当前工作区的 Git diff
///
/// 执行 `git diff` 和 `git diff --stat` 获取变更信息。
/// 如果当前目录不是 Git 仓库，返回错误。
#[tauri::command]
pub fn get_git_diff(cwd: Option<String>) -> Result<GitDiffResult, String> {
    let dir = cwd.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // 检查是否是 Git 仓库
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("执行 git 命令失败: {e}"))?;

    if !status.status.success() {
        return Err("当前目录不是 Git 仓库".to_string());
    }

    // 获取 diff --stat
    let stat_output = Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("获取 git diff --stat 失败: {e}"))?;

    let summary = String::from_utf8_lossy(&stat_output.stdout)
        .trim()
        .to_string();

    // 获取 diff --name-status
    let name_status_output = Command::new("git")
        .args(["diff", "--name-status"])
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("获取 git diff --name-status 失败: {e}"))?;

    let name_status = String::from_utf8_lossy(&name_status_output.stdout);

    // 获取完整 diff
    let diff_output = Command::new("git")
        .args(["diff"])
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("获取 git diff 失败: {e}"))?;

    let full_diff = String::from_utf8_lossy(&diff_output.stdout);

    // 解析文件列表
    let mut files = Vec::new();
    for line in name_status.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let status = parts[0].to_string();
            let path = parts[1].to_string();

            // 提取该文件的 diff 内容
            let file_diff = extract_file_diff(&full_diff, &path);

            // 计算新增/删除行数
            let (additions, deletions) = count_diff_lines(&file_diff);

            files.push(GitFileDiff {
                path,
                status,
                additions,
                deletions,
                diff: file_diff,
            });
        }
    }

    Ok(GitDiffResult { files, summary })
}

/// 从完整 diff 中提取指定文件的 diff 内容
fn extract_file_diff(full_diff: &str, file_path: &str) -> String {
    let mut result = String::new();
    let mut in_target_file = false;
    let header_pattern = format!("diff --git a/{}", file_path);

    for line in full_diff.lines() {
        if line.starts_with("diff --git") {
            if line.contains(&header_pattern) || line.ends_with(&format!("b/{}", file_path)) {
                in_target_file = true;
                result.push_str(line);
                result.push('\n');
            } else if in_target_file {
                break;
            }
        } else if in_target_file {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// 统计 diff 中的新增/删除行数
fn count_diff_lines(diff: &str) -> (u32, u32) {
    let mut additions = 0;
    let mut deletions = 0;

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    (additions, deletions)
}

/// 文件树节点
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode {
    /// 文件/文件夹名称
    pub name: String,
    /// 完整路径
    pub path: String,
    /// 是否是文件夹
    pub is_dir: bool,
    /// 子节点（仅文件夹有）
    pub children: Option<Vec<FileTreeNode>>,
    /// 是否因深度限制未展开（懒加载标记：前端展开该目录时需再请求一次子树）
    pub truncated: bool,
}

/// 获取文件树
///
/// 递归读取目录结构，限制深度避免过深。
/// 默认深度为 3 层。
#[tauri::command]
pub fn get_file_tree(root: Option<String>, depth: Option<u32>) -> Result<FileTreeNode, String> {
    let dir = root.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let max_depth = depth.unwrap_or(3);

    build_file_tree(&PathBuf::from(&dir), max_depth)
}

/// 递归构建文件树
fn build_file_tree(path: &PathBuf, remaining_depth: u32) -> Result<FileTreeNode, String> {
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let is_dir = metadata.is_dir();

    // 文件夹始终尝试读取子项，即使深度为 0 也至少显示一层文件
    let children = if is_dir {
        let mut entries = Vec::new();

        if let Ok(dir_entries) = std::fs::read_dir(path) {
            for entry in dir_entries.flatten() {
                let entry_path = entry.path();

                // 跳过隐藏文件和常见的忽略目录
                let entry_name = entry_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if entry_name.starts_with('.')
                    || entry_name == "node_modules"
                    || entry_name == "target"
                    || entry_name == "__pycache__"
                    || entry_name == ".git"
                {
                    continue;
                }

                // 深度为 0 时，只添加文件和文件夹的浅层信息（不递归子文件夹的子项）
                if remaining_depth == 0 {
                    let child_metadata = std::fs::metadata(&entry_path).ok();
                    let child_is_dir = child_metadata.map(|m| m.is_dir()).unwrap_or(false);
                    entries.push(FileTreeNode {
                        name: entry_name,
                        path: entry_path.to_string_lossy().to_string(),
                        is_dir: child_is_dir,
                        children: None,     // 不再递归
                        truncated: child_is_dir, // 子目录标记为待懒加载
                    });
                } else {
                    match build_file_tree(&entry_path, remaining_depth - 1) {
                        Ok(node) => entries.push(node),
                        Err(_) => continue,
                    }
                }
            }
        }

        // 排序：文件夹在前，文件在后，按名称排序
        entries.sort_by(|a, b| {
            if a.is_dir && !b.is_dir {
                std::cmp::Ordering::Less
            } else if !a.is_dir && b.is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        Some(entries)
    } else {
        None
    };

    Ok(FileTreeNode {
        name,
        path: path.to_string_lossy().to_string(),
        is_dir,
        children,
        truncated: false, // 已完整构建的节点不需要懒加载
    })
}

/// 读取文件内容
///
/// 读取指定路径的文件内容，返回 UTF-8 字符串。
/// 如果文件过大（超过 1MB），只返回前 1MB 内容。
#[tauri::command]
pub fn read_file_content(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);

    // 检查文件是否存在
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    // 检查是否是文件
    if !file_path.is_file() {
        return Err(format!("不是文件: {}", path));
    }

    // 检查文件大小，限制为 1MB
    let metadata = std::fs::metadata(&file_path).map_err(|e| format!("读取文件元数据失败: {e}"))?;

    let max_size: usize = 1024 * 1024; // 1MB
    if metadata.len() as usize > max_size {
        // 读取前 1MB
        use std::io::Read;
        let mut file = std::fs::File::open(&file_path).map_err(|e| format!("打开文件失败: {e}"))?;
        let mut buffer = vec![0u8; max_size];
        file.read_exact(&mut buffer)
            .map_err(|e| format!("读取文件失败: {e}"))?;

        String::from_utf8(buffer).map_err(|_| "文件包含非 UTF-8 字符".to_string())
    } else {
        std::fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {e}"))
    }
}

/// 读取文件内容为 base64 字符串
///
/// 用于右侧侧边栏的图片预览：图片文件无法以 UTF-8 文本展示，
/// 因此读取原始字节并编码为 base64，由前端拼成 `data:` URL 交给 `<img>` 渲染。
/// 为避免引入额外依赖，base64 编码在此手写实现。
#[tauri::command]
pub fn read_file_base64(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);

    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if !file_path.is_file() {
        return Err(format!("不是文件: {}", path));
    }

    let metadata = std::fs::metadata(&file_path).map_err(|e| format!("读取文件元数据失败: {e}"))?;
    let max_size: usize = 20 * 1024 * 1024; // 20MB
    if metadata.len() as usize > max_size {
        return Err("文件过大，无法预览（超过 20MB）".to_string());
    }

    let bytes = std::fs::read(&file_path).map_err(|e| format!("读取文件失败: {e}"))?;
    Ok(encode_base64(&bytes))
}

/// 手写 base64 编码（标准字母表），避免引入外部 crate。
fn encode_base64(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        out.push(CHARS[((n >> 6) & 63) as usize] as char);
        out.push(CHARS[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = (bytes[i] as u32) << 16;
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        out.push(CHARS[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

/// 将文本写入系统剪贴板（macOS 使用 pbcopy，无需引入额外 crate）。
#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    use std::io::Write;
    let mut child = Command::new("/usr/bin/pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动 pbcopy: {e}"))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "无法获取 pbcopy 的标准输入".to_string())?;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("写入剪贴板失败: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("等待 pbcopy 完成失败: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("pbcopy 退出码非零: {status}"))
    }
}

/// 在访达（Finder）中打开路径：文件用 `open -R` 定位并选中，目录用 `open` 直接打开。
#[tauri::command]
pub fn reveal_in_finder(path: String, is_dir: bool) -> Result<(), String> {
    let mut cmd = Command::new("/usr/bin/open");
    if !is_dir {
        cmd.arg("-R");
    }
    cmd.arg(&path);
    let status = cmd
        .status()
        .map_err(|e| format!("无法在访达中打开 {path}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("open 退出码非零: {status}"))
    }
}
