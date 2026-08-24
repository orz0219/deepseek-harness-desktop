//! Tauri (Rust thin shell) entry point for the DeepSeek Harness desktop launcher.
//!
//! Lifecycle (PLAN §2 / §6):
//!   locate dsh → spawn `dsh web` → poll readiness → navigate webview →
//! supervise (restart once on unexpected exit) → on app exit, SIGTERM the
//! whole process group and verify the subtree is gone.
//!
//! We never import dsh internals; we only use its CLI + HTTP readiness probe +
//! the loopback-only shutdown route dsh serves itself.

use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;
use std::time::Duration;

use dsh_launcher::{gui_url, load_settings};
use tauri::{Manager, WindowEvent};

use crate::commands::{
    copy_to_clipboard, delete_archived_sessions, get_file_tree, get_git_diff, get_status,
    read_file_base64, read_file_content, reveal_in_finder, restart_dsh,
    restore_archived_sessions, select_dsh,
};
use crate::probe::terminate_tree;
use crate::state::{LauncherState, StatusPayload, ST_STARTING};
use crate::supervisor::run_launcher;

/// Bundle identifier; must match `tauri.conf.json`.
pub const IDENTIFIER: &str = "com.deepseek.harness.desktop";
/// Readiness poll ceiling; matches dsh-desktop-launcher's 30s.
pub const READY_TIMEOUT: Duration = Duration::from_secs(30);
/// Grace period after SIGTERM before SIGKILL of the process group.
pub const KILL_GRACE: Duration = Duration::from_secs(5);
/// Base poll interval for the readiness probe.
pub const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// How often the supervisor re-checks a healthy dsh (owned or not).
pub const WATCH_INTERVAL: Duration = Duration::from_secs(3);

mod commands;
mod inject_js;
mod probe;
mod state;
mod supervisor;

fn main() {
    let settings_path = dsh_launcher::settings_path(IDENTIFIER);
    let logging_ok = dsh_launcher::logging::init(IDENTIFIER);
    let settings0 = load_settings(&settings_path);
    let initial = StatusPayload {
        state: ST_STARTING.into(),
        message: None,
        port: settings0.port,
        url: gui_url(settings0.port),
        candidates: vec![],
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(move |app| {
            dsh_launcher::logging::info(&format!(
                "启动器启动；日志={}; 设置={}",
                if logging_ok {
                    dsh_launcher::logging::logs_dir(IDENTIFIER)
                        .join("launcher.log")
                        .display()
                        .to_string()
                } else {
                    "<不可写，仅 stderr>".into()
                },
                settings_path.display()
            ));
            let state = LauncherState {
                status: Mutex::new(initial),
                pgid: Mutex::new(None),
                stopping: AtomicBool::new(false),
                generation: AtomicU64::new(0),
                torn_down: AtomicBool::new(false),
                settings_path: settings_path.clone(),
                launcher_origin: Mutex::new(String::new()),
            };
            app.manage(state);
            let app_handle = app.handle().clone();
            std::thread::spawn(move || run_launcher(app_handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            select_dsh,
            restart_dsh,
            restore_archived_sessions,
            delete_archived_sessions,
            get_git_diff,
            get_file_tree,
            read_file_content,
            read_file_base64,
            copy_to_clipboard,
            reveal_in_finder,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 点击关闭（红绿灯按钮）→ 隐藏到程序坞，而非最小化或退出；
                // 后台 dsh 继续运行。Cmd+Q 走 ExitRequested，不受此拦截影响。
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // 点击程序坞图标（macOS Reopen）→ 若窗口被隐藏则重新显示，
            // 与 CloseRequested 里的 hide() 配合，实现「关闭即隐藏到程序坞、点坞恢复」。
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                return;
            }
            // Quit paths: reap the dsh subtree reliably (Destroyed events are
            // best-effort during exit; ExitRequested/Exit are the sanctioned
            // hooks, and teardown() is idempotent so both are handled).
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                if let Some(state) = app_handle.try_state::<LauncherState>() {
                    teardown(&state);
                }
            }
        });
}

/// App-exit teardown: stop supervisors and reap the dsh subtree. Idempotent —
/// ExitRequested and Exit may both fire.
fn teardown(state: &LauncherState) {
    if state.torn_down.swap(true, Ordering::SeqCst) {
        return;
    }
    state.stopping.store(true, Ordering::SeqCst);
    if let Some(pgid) = state.pgid.lock().unwrap().take() {
        terminate_tree(pgid);
    }
}
