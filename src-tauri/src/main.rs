//! Tauri (Rust thin shell) entry point for the DeepSeek Harness desktop launcher.
//!
//! Lifecycle (PLAN §2 / §6):
//!   locate dsh → spawn `dsh web` → poll readiness (`GET /` → 200) → navigate
//!   webview → supervise (restart once on unexpected exit) → on app exit,
//!   SIGTERM the whole process group and verify the subtree is gone.
//!
//! We never import dsh internals; we only use its CLI + HTTP readiness probe +
//! the loopback-only shutdown route dsh serves itself.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dsh_launcher::{
    build_launch_plan, gui_url, load_settings, locate, save_settings, AppSettings, DshCandidate,
};
use serde::Serialize;
use tauri::{Manager, WindowEvent};

/// Bundle identifier; must match `tauri.conf.json`.
const IDENTIFIER: &str = "com.deepseek.harness.desktop";
/// Readiness poll ceiling; matches dsh-desktop-launcher's 30s.
const READY_TIMEOUT: Duration = Duration::from_secs(30);
/// Grace period after SIGTERM before SIGKILL of the process group.
const KILL_GRACE: Duration = Duration::from_secs(5);
/// Base poll interval for the readiness probe.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Injected into the dsh webview (after it navigates to the dsh SPA) to add an
/// "归档所有对话" (archive all conversations) button to dsh's left sidebar.
///
/// Why injection instead of a dsh plugin: the user wants to keep dsh stock and
/// own the feature in the launcher. The button talks to dsh's own RPC surface
/// (`/api/<method>`, `client-request` envelope) — same origin, so no CORS.
///
/// Design notes (see FEASIBILITY.md §launcher-injection):
///   * dsh's sidebar classes are hashed (e.g. `pC0e7a_*`), so we anchor by the
///     search button's `aria-label` ("搜索会话" / "Search sessions"), not by class
///     or by visible text (that control is an icon-only button). Falls back to the
///     old Chinese text anchors when the search button isn't found.
///   * The script is idempotent and installs a `MutationObserver` so SPA
///     re-renders (and even a full reload) re-add the button.
///   * Bulk archive is destructive-but-reversible; we use a two-click confirm
///     inside the page (no `window.confirm`, which Tauri may not permit).
const INJECT_JS: &str = r##"(
function () {
  var BTN_ID = 'dsh-archive-all-btn';
  function rpc(method, payload) {
    var id = (window.crypto && crypto.randomUUID)
      ? crypto.randomUUID()
      : ('r-' + Date.now() + '-' + Math.random().toString(16).slice(2));
    // base 取 window.location.origin，即 launch 页配置的 dsh 服务地址
    // （如 http://127.0.0.1:3080），不写死端口，自动跟随配置。
    return fetch(window.location.origin + '/api/' + method, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ type: 'client-request', rpcId: id, method: method, payload: payload || {} })
    }).then(function (r) { return r.json(); });
  }
  function findAnchor() {
    // 优先锚定到「搜索会话」图标按钮：两种侧边栏模式都有，且是图标按钮
    // （无法用可见文字匹配）。用 aria-label 兼容中英文 locale。找不到再回退到
    // 旧的中文文字锚点。
    var btns = document.querySelectorAll('button');
    for (var i = 0; i < btns.length; i++) {
      var al = btns[i].getAttribute('aria-label') || '';
      if (/搜索|search/i.test(al)) return btns[i];
    }
    var names = ['添加工作区', '插件市场', '设置'];
    for (var n = 0; n < names.length; n++) {
      for (var j = 0; j < btns.length; j++) {
        if (btns[j].textContent && btns[j].textContent.trim() === names[n]) return btns[j];
      }
    }
    return null;
  }
  // 归档盒图标（outline，沿用 dsh 图标描边风格，stroke=currentColor 继承按钮颜色）。
  // 用「收纳盒」而非垃圾桶，明确表达「归档」语义——点下去是归档（可从归档区恢复），
  // 不是永久删除。
  var ICON_ARCHIVE =
    '<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<rect x="2.5" y="3" width="11" height="2.6" rx="1"/>' +
    '<path d="M3 6h10a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1z"/>' +
    '<path d="M6 9.5h4"/>' +
    '</svg>';
  // 与 dsh 侧边栏的图标按钮（搜索/添加）同款：28x28 圆形、无边框、透明底，
  // 颜色用 dsh 的次级文字色，hover 用 dsh 自带的 interactive-bg-hover 变量——
  // 浅色/深色主题下都能和原生按钮完全一致。之前带 1px 浅灰边框的幽灵按钮太突兀，
  // 现改为纯图标按钮（废纸篓）。
  function setIdle(btn) {
    if (!btn) return;
    btn.innerHTML = ICON_ARCHIVE;
    btn.classList.remove('armed');
    btn.title = '归档所有对话（可从归档区恢复）';
    btn.setAttribute('aria-label', '归档所有对话（可从归档区恢复）');
  }
  function inject() {
    if (document.getElementById(BTN_ID)) return;
    var anchor = findAnchor();
    if (!anchor || !anchor.parentNode) return;
    var btn = document.createElement('button');
    btn.id = BTN_ID;
    btn.type = 'button';
    setIdle(btn);
    // 注意：color / background 不能写进内联 style（内联优先级高于样式表，
    // 会导致下方 #ID:hover 的覆盖失效）。改放在样式表的 #ID 规则里。
    btn.style.cssText = 'flex:none;display:inline-flex;align-items:center;justify-content:center;box-sizing:border-box;width:28px;height:28px;margin:0 2px 0 0;padding:0;border:none;border-radius:50%;cursor:pointer;';
    if (!document.getElementById('dsh-archive-style')) {
      var st = document.createElement('style');
      st.id = 'dsh-archive-style';
      // 按钮样式 + 确认弹框样式（弹框为普通对话框，取代原先的双击确认）
      st.textContent =
        '#' + BTN_ID + '{transition:background .15s ease,color .15s ease;color:var(--dsw-alias-label-secondary,#6b7280);background:transparent;}' +
        '#' + BTN_ID + ' svg{display:block;}' +
        '#' + BTN_ID + ':hover{background:var(--dsw-alias-interactive-bg-hover,#eceef1);}' +
        '#' + BTN_ID + ':active{background:var(--dsw-alias-interactive-bg-active,#e5e7eb);}' +
        '#dsh-archive-modal{position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;background:rgba(17,24,39,.35);}' +
        '.dsh-am-card{box-sizing:border-box;width:320px;max-width:90vw;background:#ffffff;border:1px solid #e5e7eb;border-radius:12px;box-shadow:0 10px 30px rgba(17,24,39,.18);padding:20px;font-family:inherit;color:#1f2937;}' +
        '.dsh-am-title{font-size:15px;font-weight:600;margin:0 0 8px;line-height:1.4;}' +
        '.dsh-am-msg{font-size:13px;line-height:1.6;color:#6b7280;margin:0 0 18px;}' +
        '.dsh-am-actions{display:flex;justify-content:flex-end;gap:8px;}' +
        '.dsh-am-cancel,.dsh-am-ok{height:32px;padding:0 14px;border-radius:8px;font-size:13px;font-weight:600;cursor:pointer;border:1px solid transparent;}' +
        '.dsh-am-cancel{background:#ffffff;color:#1f2937;border-color:#e5e7eb;}' +
        '.dsh-am-cancel:hover{background:#f9fafb;}' +
        '.dsh-am-ok{background:linear-gradient(135deg,#4f7cff,#8b5cf6);color:#ffffff;}' +
        '.dsh-am-ok:hover{filter:brightness(1.05);}';
      (document.head || document.documentElement).appendChild(st);
    }
    btn.addEventListener('click', function () { onArchiveAll(btn); });
    // 放到「搜索」图标按钮的左侧：
    //  - 宽模式：搜索按钮包在 searchSlot 里（默认仅 28px），插到搜索按钮前会撑破
    //    搜索框，故插到 searchSlot 之前（标题与搜索框之间）。
    //  - 折叠 rail：没有 searchSlot，插到搜索按钮的父节点（36x36 搜索框）之前，
    //    即同一行搜索图标左侧。
    var ref = anchor.closest('[class*="searchSlot"]') || anchor.parentNode;
    ref.parentNode.insertBefore(btn, ref);
  }
  // 普通确认弹框（单击图标后弹出，取代双击确认）。确认才执行归档。
  function showConfirm(count, onOk) {
    var prev = document.getElementById('dsh-archive-modal');
    if (prev) prev.remove();
    var overlay = document.createElement('div');
    overlay.id = 'dsh-archive-modal';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.innerHTML =
      '<div class="dsh-am-card">' +
        '<div class="dsh-am-title">归档所有对话</div>' +
        '<div class="dsh-am-msg">确定要归档全部 ' + count + ' 个对话吗？归档后可在归档区恢复，不会永久删除。</div>' +
        '<div class="dsh-am-actions">' +
          '<button type="button" class="dsh-am-cancel">取消</button>' +
          '<button type="button" class="dsh-am-ok">确认归档</button>' +
        '</div>' +
      '</div>';
    document.body.appendChild(overlay);
    function close() { overlay.remove(); document.removeEventListener('keydown', onKey); }
    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);
    overlay.addEventListener('mousedown', function (e) { if (e.target === overlay) close(); });
    overlay.querySelector('.dsh-am-cancel').addEventListener('click', close);
    overlay.querySelector('.dsh-am-ok').addEventListener('click', function () { close(); onOk(); });
  }
  // 获取当前工作区的可见会话（与侧边栏显示逻辑一致）。
  //
  // DSH 侧边栏的过滤规则（dsh-client-ui-workspace sessionVisible）：
  //   1. 排除已归档会话（archivedSessionIds）
  //   2. 排除 blank 会话（除非是当前会话）
  //   3. 排除 subagent 子会话
  //   4. 按工作区分组显示
  //
  // 通过 dsh 的 workspace.list 拿所有工作区的会话（排除已归档），再结合
  // session.list 的 running 标记，跳过「正在运行/生成中」的会话，避免打断。
  // 完全不依赖侧边栏/页面是否折叠（接口直接给全量数据）。
  function getArchiveTargets() {
    return Promise.all([
      rpc('workspace.list', {}),
      rpc('session.list', {})
    ]).then(function (results) {
      var wsResult = results[0];
      var sessResult = results[1];
      var wsValue = (wsResult && wsResult.result && wsResult.result.value) || {};
      var workspaces = wsValue.items || [];
      var archivedIds = new Set(wsValue.archivedSessionIds || []);
      // 正在运行（生成中）的会话不归档，避免打断
      var runningIds = new Set();
      var sessItems = (sessResult && sessResult.result && sessResult.result.value && sessResult.result.value.items) || [];
      sessItems.forEach(function (s) { if (s.running) runningIds.add(s.sessionId); });
      var targets = [];
      workspaces.forEach(function (ws) {
        (ws.sessionIds || []).forEach(function (id) {
          if (archivedIds.has(id) || runningIds.has(id)) return;
          targets.push({ sessionId: id });
        });
      });
      return targets;
    });
  }
  function onArchiveAll(btn) {
    getArchiveTargets().then(function (items) {
      if (!items.length) { btn.title = '没有可归档的对话'; btn.setAttribute('aria-label', '没有可归档的对话'); setTimeout(function () { setIdle(btn); }, 1500); return; }
      showConfirm(items.length, function () { executeArchive(btn, items); });
    }).catch(function () { btn.title = '读取会话失败'; btn.setAttribute('aria-label', '读取会话失败'); setTimeout(function () { setIdle(btn); }, 1500); });
  }
  function executeArchive(btn, items) {
    if (!items || !items.length) { setIdle(btn); return; }
    var done = 0, fail = 0, total = items.length;
    btn.title = '归档中 0/' + total;
    btn.setAttribute('aria-label', '归档中 0/' + total);
    var seq = Promise.resolve();
    items.forEach(function (it) {
      seq = seq.then(function () {
        return rpc('workspace.archiveSession', { sessionId: it.sessionId })
          .then(function (r) { if (r && r.result && r.result.ok) done++; else fail++; })
          .catch(function () { fail++; })
          .then(function () { var msg = '归档中 ' + (done + fail) + '/' + total; btn.title = msg; btn.setAttribute('aria-label', msg); });
      });
    });
    seq.then(function () {
      var msg = '已归档 ' + done + ' 个（失败 ' + fail + '）';
      btn.title = msg;
      btn.setAttribute('aria-label', msg);
      setTimeout(function () { setIdle(btn); }, 2500);
    });
  }
  inject();
  if (!window.__dshArchiveObserver && document.body) {
    var scheduled = false;
    window.__dshArchiveObserver = new MutationObserver(function () {
      if (scheduled) return; scheduled = true;
      setTimeout(function () { scheduled = false; inject(); }, 300);
    });
    window.__dshArchiveObserver.observe(document.body, { childList: true, subtree: true });
  }
}
)()"##;

/// Serializable view of a discovered dsh candidate (for the setup UI).
#[derive(Debug, Clone, Serialize)]
struct CandidateView {
    executable: String,
    version: String,
    source: String,
}

/// State pushed to the webview's status page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusPayload {
    state: String,
    message: Option<String>,
    port: u16,
    url: String,
    candidates: Vec<CandidateView>,
}

/// Shared launcher state (managed by Tauri).
struct LauncherState {
    status: Mutex<StatusPayload>,
    /// Process group id of the supervised dsh child (for tree teardown).
    pgid: Mutex<Option<i32>>,
    /// Set when the app is tearing down, so the supervisor stops restarting.
    stopping: Mutex<bool>,
    settings_path: PathBuf,
}

/// Current state kind strings (kept in sync with ui/index.html).
const ST_STARTING: &str = "starting";
const ST_READY: &str = "ready";
const ST_MISSING_DSH: &str = "missing-dsh";
const ST_ERROR: &str = "error";
const ST_RESTARTING: &str = "restarting";

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

/// Minimal HTTP/1.0 GET; returns `Some(true)` if the server answered 200,
/// `Some(false)` if it answered but not 200, `None` if it did not answer.
fn http_ok(host: &str, port: u16, path: &str) -> Option<bool> {
    let addr: SocketAddr = {
        let mut addrs: Vec<_> = (host, port).to_socket_addrs().ok()?.collect();
        addrs.pop()?
    };
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .ok()?;
    let req = format!(
        "GET {path} HTTP/1.0\r\nHost: {host}:{port}\r\nConnection: close\r\nAccept: */*\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).ok()?;
    let head = String::from_utf8_lossy(&buf[..n]);
    let status = head.lines().next()?;
    Some(status.contains(" 200 "))
}

/// Poll the readiness signal until satisfied or the timeout elapses.
/// PLAN 门禁 0 / FEASIBILITY.md: dsh has no real `/health` JSON; the agreed
/// signal (matching `@linxin666/dsh-desktop-launcher`) is `GET /` → 200.
/// We also accept `/manifest.webmanifest` → 200 as a stronger "backend is
/// serving real requests" corroboration to avoid SPA catch-all false positives.
fn wait_for_ready(port: u16) -> bool {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        let root = http_ok("127.0.0.1", port, "/").unwrap_or(false);
        let manifest = http_ok("127.0.0.1", port, "/manifest.webmanifest").unwrap_or(false);
        if root && manifest {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Check whether a DSH instance is already serving on the given port.
///
/// We probe `GET /` and `GET /manifest.webmanifest` — the same readiness
/// signals used by [`wait_for_ready`].  If both return 200 we consider the
/// port occupied by a healthy DSH and the launcher can skip spawning a new
/// child.
fn dsh_already_running(port: u16) -> bool {
    http_ok("127.0.0.1", port, "/").unwrap_or(false)
        && http_ok("127.0.0.1", port, "/manifest.webmanifest").unwrap_or(false)
}

/// Spawn dsh in its own process group (so we can tear down the whole tree) and
/// return the child. `stderr` is piped so we can surface a tail on failure
/// (PLAN Slice 1B).
fn spawn_dsh(plan: &dsh_launcher::LaunchPlan) -> std::io::Result<Child> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args)
        .envs(&plan.env)
        .current_dir(&plan.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // New process group: pgid == pid, so `kill -- -pid` kills dsh + workers.
    cmd.process_group(0);
    cmd.spawn()
}

/// Send SIGTERM to the whole process group, wait up to KILL_GRACE, then
/// SIGKILL any survivors. Returns whether the group is confirmed gone (PLAN
/// 验收点 4: ps tree verification).
fn terminate_tree(pgid: i32) -> bool {
    let _ = Command::new("kill")
        .args(["-TERM", "--", &format!("-{pgid}")])
        .output();
    std::thread::sleep(KILL_GRACE);
    if pgid_alive(pgid) {
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{pgid}")])
            .output();
        std::thread::sleep(Duration::from_millis(500));
    }
    !pgid_alive(pgid)
}

/// Heuristic: is any process still in the group? Use `pgrep -g <pgid>`.
fn pgid_alive(pgid: i32) -> bool {
    match Command::new("pgrep").arg("-g").arg(pgid.to_string()).output() {
        Ok(out) => !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

/// Tail of a child's stderr (used for the error page diagnostics).
fn stderr_tail(child: &mut Child, max_bytes: usize) -> String {
    let mut s = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let mut buf = vec![0u8; max_bytes];
        if let Ok(n) = stderr.read(&mut buf) {
            s = String::from_utf8_lossy(&buf[..n]).trim().to_string();
        }
    }
    s
}

/// Push the current status to the webview via an injected JS call.
fn notify(window: &tauri::WebviewWindow, state: &LauncherState) {
    let payload = state.status.lock().unwrap().clone();
    let js = format!(
        "window.__setLauncherState && window.__setLauncherState({})",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
    );
    let _ = window.eval(&js);
}

/// After dsh is reachable, repeatedly eval the sidebar-injection script. The
/// script is idempotent and self-reinstalls via a `MutationObserver`, so this
/// just needs to fire until dsh's SPA has mounted the sidebar (a few seconds).
fn spawn_injection(window: tauri::WebviewWindow) {
    std::thread::spawn(move || {
        for _ in 0..90 {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let _ = window.eval(INJECT_JS);
        }
    });
}

fn set_status(state: &LauncherState, kind: &str, message: Option<String>) {
    let mut s = state.status.lock().unwrap();
    s.state = kind.into();
    s.message = message;
}

/// Run one supervised lifecycle of dsh: spawn, wait ready, navigate, then
/// watch for exit. Returns the child's exit (or `None` if it was killed by us).
///
/// If a DSH instance is already serving on the configured port (e.g. launched
/// by `cargo tauri dev` or another session), we skip spawning a new child and
/// connect to the existing one instead of crashing with `EADDRINUSE`.
fn supervise_once(
    app: &tauri::AppHandle,
    state: &LauncherState,
    settings: &AppSettings,
    candidate: &DshCandidate,
) -> Option<std::process::ExitStatus> {
    let window = app.get_webview_window("main");

    // ── Fast path: another DSH is already serving on this port. ──────────
    if dsh_already_running(settings.port) {
        eprintln!(
            "[dsh] port {} already has a running DSH instance; connecting to it instead of spawning a new one.",
            settings.port
        );
        set_status(state, ST_READY, Some(gui_url(settings.port)));
        if let Some(w) = &window {
            let url = gui_url(settings.port);
            let _ = w.eval(&format!("window.location.href = '{url}'"));
            notify(w, state);
            spawn_injection(w.clone());
        }
        // We don't own the child process, so there is nothing to supervise.
        // Return None (no exit status) — the caller treats this as a clean
        // outcome because the state is already ST_READY.
        return None;
    }

    // ── Normal path: spawn a new dsh web child. ──────────────────────────
    let plan = build_launch_plan(candidate, settings, &home_dir());

    let mut child = match spawn_dsh(&plan) {
        Ok(c) => c,
        Err(e) => {
            set_status(
                state,
                ST_ERROR,
                Some(format!("无法启动 dsh: {e}")),
            );
            if let Some(w) = &window {
                notify(w, state);
            }
            return None;
        }
    };
    let pgid = child.id() as i32;
    *state.pgid.lock().unwrap() = Some(pgid);

    set_status(state, ST_STARTING, None);
    if let Some(w) = &window {
        notify(w, state);
    }

    let ready = wait_for_ready(settings.port);
    if !ready {
        // Distinguish "port taken by another program" from "dsh failed to come
        // up". If the port answers but not via our readiness signature it is a
        // clash; otherwise dsh did not start. Per PLAN 门禁 B we never auto-change
        // the port.
        let clash = http_ok("127.0.0.1", settings.port, "/").unwrap_or(false);
        let msg = if clash {
            format!(
                "端口 {} 已被其他程序占用；请在设置中修改 dsh 端口。",
                settings.port
            )
        } else {
            format!(
                "dsh 在 {} 秒内未就绪。请确认 dsh 版本支持 --port，且 web profile 可用。",
                READY_TIMEOUT.as_secs()
            )
        };
        set_status(state, ST_ERROR, Some(msg));
        if let Some(w) = &window {
            notify(w, state);
        }
        let _ = child.kill();
        return None;
    }

    set_status(state, ST_READY, Some(gui_url(settings.port)));
    if let Some(w) = &window {
        let url = gui_url(settings.port);
        let _ = w.eval(&format!("window.location.href = '{url}'"));
        notify(w, state);
        spawn_injection(w.clone());
    }

    // Watch the child until it exits or we are stopping.
    loop {
        if *state.stopping.lock().unwrap() {
            let _ = child.kill();
            return None;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let tail = stderr_tail(&mut child, 4096);
                if !tail.is_empty() {
                    eprintln!("[dsh] child exited {status}; stderr tail:\n{tail}");
                }
                return Some(status);
            }
            Ok(None) => std::thread::sleep(POLL_INTERVAL),
            Err(_) => return None,
        }
    }
}

/// Locate dsh and drive the supervisor (with one restart on unexpected exit).
fn run_launcher(app: tauri::AppHandle) {
    let state_guard = app.state::<LauncherState>();
    let state: &LauncherState = &*state_guard;
    let home = home_dir();
    let settings = load_settings(&state.settings_path);

    let outcome = locate(&settings, &home);
    let candidates: Vec<CandidateView> = outcome
        .candidates
        .iter()
        .map(|c| CandidateView {
            executable: c.executable.to_string_lossy().into_owned(),
            version: c.version.clone(),
            source: format!("{:?}", c.source),
        })
        .collect();
    {
        let mut s = state.status.lock().unwrap();
        s.state = ST_STARTING.into();
        s.message = None;
        s.candidates = candidates.clone();
    }

    let candidate = match outcome.primary {
        Some(c) => c,
        None => {
            set_status(
                &state,
                ST_MISSING_DSH,
                Some(format!(
                    "未找到 dsh。请在下方指定 dsh 路径（或确认 dsh 已在 PATH 中），保存后重启。设置文件位于：{}",
                    state.settings_path.display()
                )),
            );
            if let Some(w) = app.get_webview_window("main") {
                notify(&w, &state);
            }
            return;
        }
    };

    // First lifecycle.
    let first = supervise_once(&app, &state, &settings, &candidate);

    // If we connected to an already-running DSH (fast path), supervise_once
    // returned None but set state to ST_READY — nothing to restart.
    let already_connected = state.status.lock().unwrap().state == ST_READY;
    if already_connected {
        return;
    }

    let unexpected = first.map(|st| !st.success()).unwrap_or(true);
    if unexpected && !*state.stopping.lock().unwrap() {
        // PLAN 门禁 E: restart once on unexpected exit. A clean exit via the web
        // UI shutdown button (dsh's own loopback /api/.../shutdown) is treated
        // as a normal stop; here we still restart once to recover from crashes,
        // then surface the error page if it fails again.
        set_status(&state, ST_RESTARTING, None);
        if let Some(w) = app.get_webview_window("main") {
            notify(&w, &state);
        }
        std::thread::sleep(Duration::from_secs(1));
        let second = supervise_once(&app, &state, &settings, &candidate);
        if second.map(|st| !st.success()).unwrap_or(true) {
            set_status(
                &state,
                ST_ERROR,
                Some("dsh 进程已退出且重启后仍不可用。请查看日志或重新选择 dsh。".into()),
            );
            if let Some(w) = app.get_webview_window("main") {
                notify(&w, &state);
            }
        }
    }
}

// ----- Tauri commands exposed to the setup/error page -----

#[tauri::command]
fn get_status(state: tauri::State<LauncherState>) -> StatusPayload {
    state.status.lock().unwrap().clone()
}

/// Persist a user-chosen dsh path (Tier 0, PLAN 门禁 A) and re-run the
/// launcher. The webview reloads after the flow restarts.
#[tauri::command]
fn select_dsh(
    app: tauri::AppHandle,
    state: tauri::State<LauncherState>,
    path: String,
) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err(format!("路径不是可执行文件: {path}"));
    }
    let mut settings = load_settings(&state.settings_path);
    settings.dsh_path = Some(p);
    settings.source = "user".into();
    save_settings(&state.settings_path, &settings).map_err(|e| e.to_string())?;

    // Stop any running supervisor/child, then restart the flow with the new path.
    *state.stopping.lock().unwrap() = true;
    if let Some(pgid) = *state.pgid.lock().unwrap() {
        terminate_tree(pgid);
    }
    *state.stopping.lock().unwrap() = false;
    std::thread::spawn(move || run_launcher(app));
    Ok(())
}

fn main() {
    let settings_path = dsh_launcher::settings_path(IDENTIFIER);
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
            let state = LauncherState {
                status: Mutex::new(initial),
                pgid: Mutex::new(None),
                stopping: Mutex::new(false),
                settings_path: settings_path.clone(),
            };
            app.manage(state);
            let app_handle = app.handle().clone();
            std::thread::spawn(move || run_launcher(app_handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_status, select_dsh])
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                // 点击关闭（红绿灯按钮）→ 隐藏到程序坞，而非最小化或退出；
                // 后台 dsh 继续运行。Cmd+Q 走 ExitRequested，不受此拦截影响。
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Destroyed => {
                // App is going away: tear down the dsh process group.
                if let Some(state) = window.try_state::<LauncherState>() {
                    *state.stopping.lock().unwrap() = true;
                    if let Some(pgid) = *state.pgid.lock().unwrap() {
                        terminate_tree(pgid);
                    }
                }
            }
            _ => {}
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
            }
        });
}
