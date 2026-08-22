use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dsh_launcher::{
    build_launch_plan, format_env_snapshot, gui_url, locate, load_settings, AppSettings,
    DshCandidate,
};
use tauri::Manager;

use crate::inject_js::INJECT_JS;
use crate::probe::{home_dir, http_probe, dsh_signature_ok, wait_for_ready, spawn_dsh};
use crate::state::{
    CandidateView, LauncherState, Lifecycle, ST_ERROR, ST_MISSING_DSH, ST_READY, ST_STARTING,
};
use crate::{POLL_INTERVAL, READY_TIMEOUT, WATCH_INTERVAL};

/// Push the current status to the webview via an injected JS call. Harmless on
/// pages that do not define the hook (e.g. the dsh SPA).
fn notify(window: &tauri::WebviewWindow, state: &LauncherState) {
    let payload = state.status.lock().unwrap().clone();
    let js = format!(
        "window.__setLauncherState && window.__setLauncherState({})",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
    );
    let _ = window.eval(&js);
}

/// Surface a terminal state (error / missing-dsh / restarting / stopped) on
/// the bundled UI page.
///
/// After the webview navigated away to the dsh SPA, the launcher's status
/// handler no longer exists there — so when we must show a problem we navigate
/// BACK to the launcher origin first. The freshly-loaded page pulls the latest
/// status itself (`get_status`), so no notification timing games.
fn show_detail_page(app: &tauri::AppHandle, state: &LauncherState, kind: &str, message: &str) {
    set_status(state, kind, Some(message.to_string()));
    dsh_launcher::logging::info(&format!("状态 → {kind}: {message}"));
    if let Some(w) = app.get_webview_window("main") {
        let origin = state.launcher_origin.lock().unwrap().clone();
        let on_launcher_page = w
            .url()
            .map(|u| u.to_string().starts_with(&origin))
            .unwrap_or(false);
        if !on_launcher_page {
            let target = format!("{origin}/");
            // 原生 navigate() 更可靠；失败回退 eval。
            if let Ok(u) = tauri::Url::parse(&target) {
                if w.navigate(u).is_err() {
                    let _ = w.eval(format!("window.location.href = '{target}'"));
                }
            } else {
                let _ = w.eval(format!("window.location.href = '{target}'"));
            }
        }
        notify(&w, state);
    }
}

/// True when this supervision run has been superseded (newer save/restart
/// request) or the app is tearing down.
fn superseded(state: &LauncherState, generation: u64) -> bool {
    state.generation.load(Ordering::SeqCst) != generation || state.stopping.load(Ordering::SeqCst)
}

fn navigate_to_dsh(window: &tauri::WebviewWindow, port: u16, state: &LauncherState) {
    let url = gui_url(port);
    // 首次连接：从启动器页跨页跳到 dsh。
    if let Ok(u) = tauri::Url::parse(&url) {
        if window.navigate(u).is_err() {
            let _ = window.eval(format!("window.location.href = '{url}'"));
        }
    } else {
        let _ = window.eval(format!("window.location.href = '{url}'"));
    }
    notify(window, state);
    spawn_injection(window.clone());
}

/// After dsh is reachable, repeatedly eval the sidebar-injection script. The
/// script is idempotent and self-reinstalls via a `MutationObserver`, so this
/// just needs to fire until dsh's SPA has mounted the sidebar (a few seconds).
/// A guard prevents overlapping injection threads across reconnects.
fn spawn_injection(window: tauri::WebviewWindow) {
    static LAST_START: Mutex<Option<Instant>> = Mutex::new(None);
    {
        let mut last = LAST_START.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(110) {
                return;
            }
        }
        *last = Some(Instant::now());
    }
    std::thread::spawn(move || {
        for _ in 0..90 {
            std::thread::sleep(Duration::from_millis(1500));
            let _ = window.eval(INJECT_JS);
        }
    });
}

fn set_status(state: &LauncherState, kind: &str, message: Option<String>) {
    let mut s = state.status.lock().unwrap();
    s.state = kind.into();
    s.message = message;
}

/// Run one supervised lifecycle of dsh: establish (connect to an existing
/// healthy instance or spawn a new child), navigate, then watch until the
/// service is lost, the child exits, or the run is superseded.
fn supervise_once(
    app: &tauri::AppHandle,
    state: &LauncherState,
    settings: &AppSettings,
    candidate: &DshCandidate,
) -> Lifecycle {
    let generation = state.generation.load(Ordering::SeqCst);
    let window = app.get_webview_window("main");

    // ── Fast path: another DSH instance is already serving on this port. ──
    if dsh_signature_ok(settings.port) {
        dsh_launcher::logging::info(&format!(
            "端口 {} 已有健康的 dsh 实例，直接连接（不重复拉起）",
            settings.port
        ));
        set_status(state, ST_READY, Some(gui_url(settings.port)));
        if let Some(w) = &window {
            navigate_to_dsh(w, settings.port, state);
        }
        // We don't own the process, but we DO watch the endpoint: if it dies
        // the caller gets ConnectionLost and can recover like any crash.
        loop {
            if superseded(state, generation) {
                return Lifecycle::Aborted;
            }
            std::thread::sleep(WATCH_INTERVAL);
            if !dsh_signature_ok(settings.port) {
                return Lifecycle::ConnectionLost("已连接的 dsh 实例停止响应".into());
            }
        }
    }

    // ── Normal path: spawn a new dsh web child. ──────────────────────────
    let plan = build_launch_plan(candidate, settings, &home_dir());
    dsh_launcher::logging::info(&format!(
        "启动命令: {} {}",
        plan.program.display(),
        plan.args.join(" ")
    ));
    dsh_launcher::logging::info(&format!(
        "spawn env 快照:\n{}",
        format_env_snapshot(&plan.env)
    ));

    let (mut child, stderr) = match spawn_dsh(&plan) {
        Ok(c) => c,
        Err(e) => {
            show_detail_page(app, state, ST_ERROR, &format!("无法启动 dsh: {e}"));
            return Lifecycle::StartFailed;
        }
    };
    let pgid = child.id() as i32;
    *state.pgid.lock().unwrap() = Some(pgid);

    set_status(state, ST_STARTING, None);
    if let Some(w) = &window {
        notify(w, state);
    }

    if !wait_for_ready(settings.port) {
        // Distinguish "port taken by another program" from "dsh failed to come
        // up". If the port answers but not via our readiness signature it is a
        // clash; otherwise dsh did not start. Per PLAN 门禁 B we never auto-change
        // the port.
        let clash = http_probe("127.0.0.1", settings.port, "/").is_some();
        let msg = if clash {
            format!(
                "端口 {} 已被其他程序占用；请在下方设置中修改 dsh 端口。",
                settings.port
            )
        } else {
            format!(
                "dsh 在 {} 秒内未就绪。请确认 dsh 版本支持 --port，且 web profile 可用。",
                READY_TIMEOUT.as_secs()
            )
        };
        let tail = stderr.tail();
        if !tail.is_empty() {
            dsh_launcher::logging::error(&format!("dsh 启动失败 stderr 尾部:\n{tail}"));
        }
        let _ = child.kill();
        *state.pgid.lock().unwrap() = None;
        show_detail_page(app, state, ST_ERROR, &msg);
        return Lifecycle::StartFailed;
    }

    set_status(state, ST_READY, Some(gui_url(settings.port)));
    if let Some(w) = &window {
        navigate_to_dsh(w, settings.port, state);
    }

    // Watch the child until it exits or we are superseded/stopping.
    loop {
        if superseded(state, generation) {
            let _ = child.kill();
            return Lifecycle::Aborted;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                *state.pgid.lock().unwrap() = None;
                let tail = stderr.tail();
                if !tail.is_empty() {
                    dsh_launcher::logging::info(&format!(
                        "[dsh] child exited {status}; stderr tail:\n{tail}"
                    ));
                } else {
                    dsh_launcher::logging::info(&format!("[dsh] child exited {status}"));
                }
                return Lifecycle::Exited(status);
            }
            Ok(None) => std::thread::sleep(POLL_INTERVAL),
            Err(e) => {
                dsh_launcher::logging::error(&format!("watch child 失败: {e}"));
                return Lifecycle::Aborted;
            }
        }
    }
}

/// Locate dsh and drive the supervisor for one lifecycle. If dsh is killed
/// (process dies abnormally or the connected instance stops responding) we
/// exit the entire desktop app — the user relaunches it fresh rather than us
/// trying to recover the WebView, which has proven unreliable here. A clean
/// exit (the dsh Web UI 关机 button) or a start failure keeps the app open
/// with an informative page so the user can re-select / restart dsh.
pub fn run_launcher(app: tauri::AppHandle) {
    let state_guard = app.state::<LauncherState>();
    let state: &LauncherState = &state_guard;

    // Remember where the launcher UI lives so error pages can navigate back
    // even after the webview moved to the dsh SPA.
    {
        let mut origin = state.launcher_origin.lock().unwrap();
        if origin.is_empty() {
            if let Some(w) = app.get_webview_window("main") {
                if let Ok(url) = w.url() {
                    *origin = url.to_string().trim_end_matches('/').to_string();
                }
            }
            if origin.is_empty() {
                *origin = "tauri://localhost".into();
            }
        }
    }

    let home = home_dir();
    let settings = load_settings(&state.settings_path);

    let outcome = locate(&settings, &home);
    let candidates: Vec<CandidateView> = outcome
        .candidates
        .iter()
        .map(|c| CandidateView {
            executable: c.executable.to_string_lossy().into_owned(),
            version: c.version.clone(),
            source: c.source.as_str().to_string(),
        })
        .collect();
    for c in &outcome.candidates {
        dsh_launcher::logging::info(&format!(
            "候选 dsh: {} · {} · 来源={}",
            c.executable.display(),
            c.version,
            c.source.as_str()
        ));
    }
    {
        let mut s = state.status.lock().unwrap();
        s.state = ST_STARTING.into();
        s.message = None;
        s.candidates = candidates.clone();
    }

    let candidate = match outcome.primary {
        Some(c) => c,
        None => {
            show_detail_page(
                &app,
                state,
                ST_MISSING_DSH,
                &format!(
                    "未找到 dsh。请在下方指定 dsh 路径（或确认 dsh 已在 PATH 中），保存后自动重启。设置文件位于：{}",
                    state.settings_path.display()
                ),
            );
            return;
        }
    };

    let lifecycle = supervise_once(&app, state, &settings, &candidate);
    match lifecycle {
        Lifecycle::Aborted => return,
        Lifecycle::StartFailed => return,
        Lifecycle::Exited(status) if status.success() => {
            // Clean exit — typically the dsh Web UI 关机 button. Keep the app
            // open and offer a restart instead of tearing everything down.
            show_detail_page(
                &app,
                state,
                ST_ERROR,
                "dsh 已正常退出（可能通过 Web 界面关机）。点击「重新启动」可再次拉起。",
            );
            return;
        }
        Lifecycle::Exited(_) => {
            // dsh was killed / crashed → exit the whole app so the user
            // relaunches it cleanly. The run loop's teardown SIGTERMs the
            // (already dead) child, so no orphan is left behind.
            dsh_launcher::logging::error("dsh 进程异常退出；退出桌面应用。");
            app.exit(0);
        }
        Lifecycle::ConnectionLost(reason) => {
            // The connected dsh instance stopped responding → same as a kill:
            // exit the app rather than attempt an unreliable WebView recovery.
            dsh_launcher::logging::error(&format!("{reason}；退出桌面应用。"));
            app.exit(0);
        }
    }
}
