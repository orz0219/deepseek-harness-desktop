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
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dsh_launcher::{
    build_launch_plan, format_env_snapshot, gui_url, load_settings, locate, parse_status_code,
    save_settings, AppSettings, DshCandidate,
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
/// How often the supervisor re-checks a healthy dsh (owned or not).
const WATCH_INTERVAL: Duration = Duration::from_secs(3);
/// Supervisor budget: one initial run plus one recovery attempt (PLAN 门禁 E).
const MAX_ATTEMPTS: usize = 2;

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
  var BTN_ID2 = 'dsh-archived-btn';
  var PANEL_ID = 'dsh-archived-panel';
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
  // 时钟/历史图标（同在「归档盒」按钮右侧）。同样 outline 描边、stroke=currentColor，
  // 与归档盒/原生图标按钮保持一致的视觉语言；颜色与 hover 由 #BTN_ID2 样式表规则接管。
  var ICON_CLOCK =
    '<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<circle cx="8" cy="8" r="6.2"/>' +
    '<path d="M8 4.9V8l2.4 1.5"/>' +
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
  function setIdle2(btn) {
    if (!btn) return;
    btn.innerHTML = ICON_CLOCK;
    btn.title = '已归档的会话（管理）';
    btn.setAttribute('aria-label', '已归档的会话（管理）');
  }
  function ensureStyle() {
    if (document.getElementById('dsh-archive-style')) return;
    var st = document.createElement('style');
    st.id = 'dsh-archive-style';
    // 按钮样式 + 归档确认弹框 + 归档管理器面板样式（均为浅色冷调，见 THEME.md）。
    st.textContent =
      '#' + BTN_ID + ',#' + BTN_ID2 + '{transition:background .15s ease,color .15s ease;color:var(--dsw-alias-label-secondary,#6b7280);background:transparent;}' +
      '#' + BTN_ID + ' svg,#' + BTN_ID2 + ' svg{display:block;}' +
      '#' + BTN_ID + ':hover,#' + BTN_ID2 + ':hover{background:var(--dsw-alias-interactive-bg-hover,#eceef1);}' +
      '#' + BTN_ID + ':active,#' + BTN_ID2 + ':active{background:var(--dsw-alias-interactive-bg-active,#e5e7eb);}' +
      '#dsh-archive-modal{position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;background:rgba(17,24,39,.35);}' +
      '.dsh-am-card{box-sizing:border-box;width:320px;max-width:90vw;background:#ffffff;border:1px solid #e5e7eb;border-radius:12px;box-shadow:0 10px 30px rgba(17,24,39,.18);padding:20px;font-family:inherit;color:#1f2937;}' +
      '.dsh-am-title{font-size:15px;font-weight:600;margin:0 0 8px;line-height:1.4;}' +
      '.dsh-am-msg{font-size:13px;line-height:1.6;color:#6b7280;margin:0 0 18px;}' +
      '.dsh-am-actions{display:flex;justify-content:flex-end;gap:8px;}' +
      '.dsh-am-cancel,.dsh-am-ok,.dsh-am-delete-ok{height:32px;padding:0 14px;border-radius:8px;font-size:13px;font-weight:600;cursor:pointer;border:1px solid transparent;}' +
      '.dsh-am-cancel{background:#ffffff;color:#1f2937;border-color:#e5e7eb;}' +
      '.dsh-am-cancel:hover{background:#f9fafb;}' +
      '.dsh-am-ok{background:linear-gradient(135deg,#4f7cff,#8b5cf6);color:#ffffff;}' +
      '.dsh-am-ok:hover{filter:brightness(1.05);}' +
      '.dsh-am-delete-ok{background:#b91c1c;color:#ffffff;}' +
      '.dsh-am-delete-ok:hover{filter:brightness(1.08);}' +
      // ---- 归档管理器面板（时钟按钮打开）----
      '#dsh-archived-panel{position:fixed;inset:0;z-index:2147483646;display:flex;align-items:center;justify-content:center;background:rgba(17,24,39,.35);}' +
      '.dsh-ap-card{box-sizing:border-box;width:620px;max-width:92vw;max-height:76vh;display:flex;flex-direction:column;background:#ffffff;border:1px solid #e5e7eb;border-radius:14px;box-shadow:0 12px 40px rgba(31,41,55,.18);font-family:inherit;color:#1f2937;overflow:hidden;}' +
      '.dsh-ap-head{display:flex;align-items:center;justify-content:space-between;padding:16px 18px 10px;}' +
      '.dsh-ap-ttl{font-size:15px;font-weight:600;}' +
      '.dsh-ap-close{width:26px;height:26px;border:none;border-radius:50%;background:transparent;color:#6b7280;display:inline-flex;align-items:center;justify-content:center;cursor:pointer;}' +
      '.dsh-ap-close:hover{background:#f3f4f6;}' +
      '.dsh-ap-close svg{width:14px;height:14px;}' +
      '.dsh-ap-toolbar{display:flex;align-items:center;gap:10px;padding:0 18px 10px;border-bottom:1px solid #eef0f3;font-size:12px;color:#6b7280;}' +
      '.dsh-ap-selectall{display:inline-flex;align-items:center;gap:6px;cursor:pointer;color:#374151;}' +
      '.dsh-ap-selectall input{width:14px;height:14px;accent-color:#3b6cf6;margin:0;}' +
      '.dsh-ap-count{margin-left:auto;font-variant-numeric:tabular-nums;}' +
      '.dsh-ap-refresh{height:26px;padding:0 10px;border:1px solid #e5e7eb;background:#ffffff;color:#374151;border-radius:8px;font-size:12px;cursor:pointer;}' +
      '.dsh-ap-refresh:hover{background:#f9fafb;border-color:#374151;}' +
      '.dsh-ap-list{flex:1;min-height:0;overflow-y:auto;padding:6px 18px 10px;}' +
      '.dsh-ap-empty{color:#9ca3af;text-align:center;padding:30px 0;font-size:13px;}' +
      '.dsh-ap-row{display:flex;align-items:center;gap:10px;padding:9px 10px;border-radius:9px;cursor:pointer;}' +
      '.dsh-ap-row:hover{background:#f9fafb;}' +
      '.dsh-ap-row input{width:14px;height:14px;accent-color:#3b6cf6;flex:none;margin:0;}' +
      '.dsh-ap-row input:disabled{accent-color:#d1d5db;cursor:default;}' +
      '.dsh-ap-rowbody{flex:1;min-width:0;}' +
      '.dsh-ap-rowtitle{font-size:13px;color:#1f2937;font-weight:500;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
      '.dsh-ap-rowmeta{font-size:11px;color:#6b7280;margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
      '.dsh-ap-rowtime{flex:none;font-size:11px;color:#9ca3af;font-variant-numeric:tabular-nums;}' +
      '.dsh-ap-row.is-running{opacity:.75;}' +
      '.dsh-ap-badge{flex:none;font-size:11px;color:#1e40af;background:#e5edff;border-radius:9px;padding:1px 8px;}' +
      '.dsh-ap-status{min-height:18px;padding:6px 18px;font-size:12px;color:#6b7280;line-height:1.5;}' +
      '.dsh-ap-status.err{color:#b91c1c;}' +
      '.dsh-ap-status.ok{color:#166534;}' +
      '.dsh-ap-actions{display:flex;justify-content:flex-end;gap:8px;padding:12px 18px 16px;border-top:1px solid #eef0f3;}' +
      '.dsh-ap-restore{height:32px;padding:0 14px;border-radius:8px;font-size:13px;font-weight:600;cursor:pointer;background:#ffffff;color:#1f2937;border:1px solid #e5e7eb;}' +
      '.dsh-ap-restore:hover:not(:disabled){background:#f9fafb;border-color:#374151;}' +
      '.dsh-ap-restore:disabled{color:#9ca3af;cursor:not-allowed;}' +
      '.dsh-ap-delete{height:32px;padding:0 14px;border-radius:8px;font-size:13px;font-weight:600;cursor:pointer;background:#ffffff;color:#b91c1c;border:1px solid #fecaca;}' +
      '.dsh-ap-delete:hover:not(:disabled){background:#fef2f2;border-color:#b91c1c;}' +
      '.dsh-ap-delete:disabled{color:#fca5a5;border-color:#fee2e2;cursor:not-allowed;}' +
      '.dsh-ap-loading{display:flex;align-items:center;gap:8px;color:#6b7280;padding:26px 0;justify-content:center;font-size:13px;}' +
      '.dsh-ap-spin{width:14px;height:14px;border:2px solid #e5e7eb;border-top-color:#3b6cf6;border-radius:50%;animation:dsh-ap-rot .8s linear infinite;}' +
      '@keyframes dsh-ap-rot{to{transform:rotate(360deg);}}';
    (document.head || document.documentElement).appendChild(st);
  }
  function addButtonAfter(ref, id, label, onClick) {
    var b = document.createElement('button');
    b.id = id;
    b.type = 'button';
    b.title = label;
    b.setAttribute('aria-label', label);
    b.style.cssText = 'flex:none;display:inline-flex;align-items:center;justify-content:center;box-sizing:border-box;width:28px;height:28px;margin:0 2px 0 0;padding:0;border:none;border-radius:50%;cursor:pointer;';
    b.addEventListener('click', onClick);
    ref.parentNode.insertBefore(b, ref.nextSibling);
    return b;
  }
  function inject() {
    ensureStyle();
    var anchor = findAnchor();
    var arch = document.getElementById(BTN_ID);
    if (!arch) {
      if (!anchor || !anchor.parentNode) return;
      arch = document.createElement('button');
      arch.id = BTN_ID;
      arch.type = 'button';
      setIdle(arch);
      // 注意：color / background 不能写进内联 style（内联优先级高于样式表，
      // 会导致下方 #ID:hover 的覆盖失效）。改放在样式表的 #ID 规则里。
      arch.style.cssText = 'flex:none;display:inline-flex;align-items:center;justify-content:center;box-sizing:border-box;width:28px;height:28px;margin:0 2px 0 0;padding:0;border:none;border-radius:50%;cursor:pointer;';
      arch.addEventListener('click', function () { onArchiveAll(arch); });
      // 放到「搜索」图标按钮的左侧：
      //  - 宽模式：搜索按钮包在 searchSlot 里（默认仅 28px），插到搜索按钮前会撑破
      //    搜索框，故插到 searchSlot 之前（标题与搜索框之间）。
      //  - 折叠 rail：没有 searchSlot，插到搜索按钮的父节点（36x36 搜索框）之前，
      //    即同一行搜索图标左侧。
      var ref = anchor.closest('[class*="searchSlot"]') || anchor.parentNode;
      ref.parentNode.insertBefore(arch, ref);
    }
    // 时钟按钮：「归档按钮右侧」。
    var archv = document.getElementById(BTN_ID2);
    if (!archv && arch && arch.parentNode) {
      archv = addButtonAfter(arch, BTN_ID2, '已归档的会话（管理）', function () { openArchivedPanel(); });
      setIdle2(archv);
    }
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
  // ================= 归档管理器（时钟按钮 → 面板） =================
  function tauriInvoke(cmd, args) {
    var t = window.__TAURI__;
    var fn = t && t.core && t.core.invoke;
    if (!fn) return Promise.reject(new Error('no-tauri'));
    return fn(cmd, args);
  }
  function hasTauri() {
    return !!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke);
  }
  function basename(p) { if (!p) return ''; var parts = String(p).split('/'); return parts[parts.length - 1] || ''; }
  function pad2(n) { return (n < 10 ? '0' : '') + n; }
  function fmtTime(ms) {
    if (!ms) return '—';
    var d = new Date(ms), now = new Date();
    var date = (d.getFullYear() === now.getFullYear())
      ? (pad2(d.getMonth() + 1) + '-' + pad2(d.getDate()))
      : (d.getFullYear() + '-' + pad2(d.getMonth() + 1) + '-' + pad2(d.getDate()));
    return date + ' ' + pad2(d.getHours()) + ':' + pad2(d.getMinutes());
  }
  function panelEl() { return document.getElementById(PANEL_ID); }
  function setStatus(msg, kind) {
    var p = panelEl(); if (!p) return;
    var s = p.querySelector('.dsh-ap-status');
    if (!s) return;
    s.textContent = msg || '';
    s.className = 'dsh-ap-status' + (kind ? ' ' + kind : '');
    if (msg) s.style.display = 'block'; else s.style.display = 'none';
  }
  function selectedIds() {
    var p = panelEl(); if (!p) return [];
    var out = [];
    p.querySelectorAll('.dsh-ap-row input[type=checkbox]:checked').forEach(function (cb) {
      out.push(cb.getAttribute('data-id'));
    });
    return out;
  }
  function updateSelection() {
    var p = panelEl(); if (!p) return;
    var boxes = p.querySelectorAll('.dsh-ap-row input[type=checkbox]');
    var all = p.querySelector('.dsh-ap-selectall input');
    var checked = 0, count = 0;
    boxes.forEach(function (cb) {
      if (cb.disabled) return;
      count++;
      if (cb.checked) checked++;
    });
    if (all) { all.checked = count > 0 && checked === count; all.indeterminate = checked > 0 && checked < count; }
    var n = checked;
    var rest = p.querySelector('.dsh-ap-restore');
    var del = p.querySelector('.dsh-ap-delete');
    if (rest) { rest.disabled = n === 0 || !hasTauri(); rest.textContent = '撤销归档（' + n + '）'; }
    if (del) { del.disabled = n === 0 || !hasTauri(); del.textContent = '物理删除（' + n + '）'; }
  }
  function renderArchived(rows, total) {
    var p = panelEl(); if (!p) return;
    var countEl = p.querySelector('.dsh-ap-count');
    if (countEl) countEl.textContent = '共 ' + total + ' 个';
    var list = p.querySelector('.dsh-ap-list');
    list.innerHTML = '';
    if (!rows.length) {
      var empty = document.createElement('div');
      empty.className = 'dsh-ap-empty';
      empty.textContent = '暂无归档会话';
      list.appendChild(empty);
      updateSelection();
      return;
    }
    rows.forEach(function (row) {
      var lab = document.createElement('label');
      lab.className = 'dsh-ap-row' + (row.running ? ' is-running' : '');
      var cb = document.createElement('input');
      cb.type = 'checkbox';
      cb.className = 'dsh-ap-rowcheck';
      cb.setAttribute('data-id', row.id);
      if (row.running) cb.disabled = true;
      cb.addEventListener('change', updateSelection);
      var body = document.createElement('div');
      body.className = 'dsh-ap-rowbody';
      var title = document.createElement('div');
      title.className = 'dsh-ap-rowtitle';
      title.textContent = row.title;
      body.appendChild(title);
      var meta = document.createElement('div');
      meta.className = 'dsh-ap-rowmeta';
      meta.textContent = row.meta;
      body.appendChild(meta);
      lab.appendChild(cb);
      lab.appendChild(body);
      if (row.running) {
        var badge = document.createElement('span');
        badge.className = 'dsh-ap-badge';
        badge.textContent = '运行中';
        lab.appendChild(badge);
      }
      var time = document.createElement('span');
      time.className = 'dsh-ap-rowtime';
      time.textContent = row.time;
      lab.appendChild(time);
      list.appendChild(lab);
    });
    updateSelection();
  }
  function loadArchived() {
    var p = panelEl(); if (!p) return Promise.resolve();
    var list = p.querySelector('.dsh-ap-list');
    list.innerHTML = '<div class="dsh-ap-loading"><span class="dsh-ap-spin"></span>加载归档会话…</div>';
    setStatus('', '');
    return Promise.all([rpc('workspace.list', {}), rpc('session.list', {})])
      .then(function (results) {
        var wsResult = results[0], sessResult = results[1];
        var wsValue = (wsResult && wsResult.result && wsResult.result.value) || {};
        var workspaces = wsValue.items || [];
        var archivedIds = wsValue.archivedSessionIds || [];
        var sessItems = (sessResult && sessResult.result && sessResult.result.value && sessResult.result.value.items) || [];
        var sessById = {};
        sessItems.forEach(function (s) { sessById[s.sessionId] = s; });
        // 存档 id → 所属工作区名
        var wsOf = {};
        workspaces.forEach(function (ws) {
          (ws.sessionIds || []).forEach(function (id) { if (!wsOf[id]) wsOf[id] = ws.title; });
        });
        var rows = [];
        archivedIds.forEach(function (id) {
          var s = sessById[id];
          if (!s) {
            rows.push({ id: id, title: '(无标题)', meta: '(记录缺失)', time: '—', running: false });
            return;
          }
          if (s.origin === 'subagent') return;          // 子代理会话隐藏
          if (s.blank) return;                          // 占位「新会话」行不管理
          var projTitle = s.projections && s.projections.values && s.projections.values.title;
          var title = projTitle || basename(s.cwd) || '(无标题)';
          var wsName = wsOf[id] || (s.cwd ? basename(s.cwd) : '') || '未知工作区';
          var meta = wsName;
          rows.push({
            id: id,
            title: title,
            meta: meta,
            time: fmtTime(s.updatedAt),
            running: !!s.running
          });
        });
        // 最近优先
        var times = {};
        sessItems.forEach(function (s) { times[s.sessionId] = s.updatedAt || 0; });
        rows.sort(function (a, b) { return (times[b.id] || 0) - (times[a.id] || 0); });
        renderArchived(rows, archivedIds.length);
      })
      .catch(function (e) {
        list.innerHTML = '<div class="dsh-ap-empty">读取归档会话失败</div>';
        setStatus('读取失败：' + (e && e.message ? e.message : e), 'err');
      });
  }
  function openArchivedPanel() {
    var p = panelEl();
    if (p) { closeArchivedPanel(); return; }
    var overlay = document.createElement('div');
    overlay.id = PANEL_ID;
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.innerHTML =
      '<div class="dsh-ap-card">' +
        '<div class="dsh-ap-head">' +
          '<div class="dsh-ap-ttl">已归档的会话</div>' +
          '<button type="button" class="dsh-ap-close" aria-label="关闭">' +
            '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>' +
          '</button>' +
        '</div>' +
        '<div class="dsh-ap-toolbar">' +
          '<label class="dsh-ap-selectall"><input type="checkbox">全选</label>' +
          '<span class="dsh-ap-count"></span>' +
          '<button type="button" class="dsh-ap-refresh">刷新</button>' +
        '</div>' +
        '<div class="dsh-ap-list"></div>' +
        '<div class="dsh-ap-status" style="display:none"></div>' +
        '<div class="dsh-ap-actions">' +
          '<button type="button" class="dsh-ap-restore" disabled>撤销归档（0）</button>' +
          '<button type="button" class="dsh-ap-delete" disabled>物理删除（0）</button>' +
        '</div>' +
      '</div>';
    document.body.appendChild(overlay);
    function close() { closeArchivedPanel(); document.removeEventListener('keydown', onKey); }
    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);
    overlay.addEventListener('mousedown', function (e) { if (e.target === overlay) close(); });
    overlay.querySelector('.dsh-ap-close').addEventListener('click', close);
    overlay.querySelector('.dsh-ap-selectall input').addEventListener('change', function (e) {
      var want = e.target.checked;
      overlay.querySelectorAll('.dsh-ap-row input[type=checkbox]').forEach(function (cb) {
        if (!cb.disabled) cb.checked = want;
      });
      updateSelection();
    });
    overlay.querySelector('.dsh-ap-refresh').addEventListener('click', loadArchived);
    overlay.querySelector('.dsh-ap-restore').addEventListener('click', runRestore);
    overlay.querySelector('.dsh-ap-delete').addEventListener('click', runDelete);
    if (!hasTauri()) {
      setStatus('当前页面不在启动器窗口内，仅可查看归档；撤销 / 删除需要重启器支持。', 'err');
    }
    loadArchived();
  }
  function closeArchivedPanel() {
    var p = panelEl();
    if (p) p.remove();
  }
  // 危险二次确认（物理删除）。复用确认弹框样式，红字确认键。
  function showDangerConfirm(count, onOk) {
    var prev = document.getElementById('dsh-archive-modal');
    if (prev) prev.remove();
    var overlay = document.createElement('div');
    overlay.id = 'dsh-archive-modal';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.innerHTML =
      '<div class="dsh-am-card">' +
        '<div class="dsh-am-title">物理删除会话</div>' +
        '<div class="dsh-am-msg">将永久删除选中的 ' + count + ' 个会话，并从磁盘移除会话文件，此操作<b>不可恢复</b>。确定继续吗？</div>' +
        '<div class="dsh-am-actions">' +
          '<button type="button" class="dsh-am-cancel">取消</button>' +
          '<button type="button" class="dsh-am-delete-ok">确认删除</button>' +
        '</div>' +
      '</div>';
    document.body.appendChild(overlay);
    function close() { overlay.remove(); document.removeEventListener('keydown', onKey); }
    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);
    overlay.addEventListener('mousedown', function (e) { if (e.target === overlay) close(); });
    overlay.querySelector('.dsh-am-cancel').addEventListener('click', close);
    overlay.querySelector('.dsh-am-delete-ok').addEventListener('click', function () { close(); onOk(); });
  }
  function opOutcome(res, label) {
    var r = res && res.report ? res.report : {};
    var n = (r.changedArchive != null) ? r.changedArchive : 0;
    if (res && res.restartTriggered) {
      return label + ' ' + n + ' 个会话，DSH 正在重启以生效…';
    }
    return label + ' ' + n + ' 个会话。当前 DSH 由外部程序启动，请手动重启 DSH 后生效；生效前请勿再归档会话（修改可能被覆盖）。';
  }
  function refreshAfterOp() {
    setTimeout(function () { if (panelEl()) loadArchived(); }, 1400);
  }
  function runRestore() {
    var ids = selectedIds();
    if (!ids.length || panelEl().querySelector('.dsh-ap-restore').disabled) return;
    panelEl().querySelector('.dsh-ap-restore').disabled = true;
    setStatus('正在撤销归档…');
    tauriInvoke('restore_archived_sessions', { ids: ids })
      .then(function (res) {
        if (!panelEl()) return;
        setStatus(opOutcome(res, '已撤销'), 'ok');
        refreshAfterOp();
      })
      .catch(function (e) {
        if (!panelEl()) return;
        setStatus('撤销归档失败：' + (e && e.message ? e.message : e), 'err');
        updateSelection();
      });
  }
  function runDelete() {
    var ids = selectedIds();
    if (!ids.length || panelEl().querySelector('.dsh-ap-delete').disabled) return;
    showDangerConfirm(ids.length, function () {
      if (!panelEl()) return;
      panelEl().querySelector('.dsh-ap-delete').disabled = true;
      setStatus('正在物理删除…');
      tauriInvoke('delete_archived_sessions', { ids: ids })
        .then(function (res) {
          if (!panelEl()) return;
          var r = res && res.report ? res.report : {};
          var deleted = (r.deletedDirs != null) ? r.deletedDirs : 0;
          var msg = (res && res.restartTriggered)
            ? '已物理删除 ' + deleted + ' 个会话目录，DSH 正在重启以生效…'
            : '已物理删除 ' + deleted + ' 个会话目录。当前 DSH 由外部程序启动，请手动重启 DSH 后生效；生效前请勿再归档会话。';
          setStatus(msg, 'ok');
          refreshAfterOp();
        })
        .catch(function (e) {
          if (!panelEl()) return;
          setStatus('物理删除失败：' + (e && e.message ? e.message : e), 'err');
          updateSelection();
        });
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
    /// Set when the app is tearing down, so supervisors stop watching/retrying.
    stopping: AtomicBool,
    /// Bumped whenever a (re)start is requested externally (save/restart
    /// command); running supervisors observe it and abort quietly. This closes
    /// the save-settings race where an old supervisor could spawn a competing
    /// dsh child after its teardown.
    generation: AtomicU64,
    /// App-exit teardown runs once (ExitRequested / Exit may both fire).
    torn_down: AtomicBool,
    settings_path: PathBuf,
    /// Origin of the bundled UI page ("http://localhost:<port>" in dev,
    /// "tauri://localhost" in prod). Error pages live here; after dsh took
    /// over the view we navigate BACK here to surface any problem.
    launcher_origin: Mutex<String>,
}

/// Outcome of one supervised lifecycle attempt.
enum Lifecycle {
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
const ST_STARTING: &str = "starting";
const ST_READY: &str = "ready";
const ST_MISSING_DSH: &str = "missing-dsh";
const ST_ERROR: &str = "error";
const ST_RESTARTING: &str = "restarting";

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

/// Minimal HTTP/1.0 GET; returns the response status code, or `None` when the
/// server did not answer in time.
fn http_probe(host: &str, port: u16, path: &str) -> Option<u16> {
    let addr: SocketAddr = {
        let mut addrs: Vec<_> = (host, port).to_socket_addrs().ok()?.collect();
        addrs.pop()?
    };
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    let req = format!(
        "GET {path} HTTP/1.0\r\nHost: {host}:{port}\r\nConnection: close\r\nAccept: */*\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).ok()?;
    parse_status_code(&String::from_utf8_lossy(&buf[..n]))
}

/// Signature test for "a dsh web instance is serving on this port":
///
/// * `GET /` and `GET /manifest.webmanifest` → 200 (the agreed readiness signal,
///   matching `@linxin666/dsh-desktop-launcher`), **and**
/// * `GET /api` → 404 (dsh mounts the RPC namespace; unknown SPA paths return
///   the catch-all HTML 200 — FEASIBILITY 门禁 0).
///
/// The third probe filters out unrelated local servers that happen to answer
/// 200 on both static paths. Residual ambiguity (another dsh-shaped service)
/// is accepted per the "follow dsh's current behavior" principle.
fn dsh_signature_ok(port: u16) -> bool {
    http_probe("127.0.0.1", port, "/") == Some(200)
        && http_probe("127.0.0.1", port, "/manifest.webmanifest") == Some(200)
        && http_probe("127.0.0.1", port, "/api") == Some(404)
}

/// Poll the readiness signal until satisfied or the timeout elapses.
fn wait_for_ready(port: u16) -> bool {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if dsh_signature_ok(port) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Spawn dsh in its own process group (so we can tear down the whole tree) and
/// return the child plus its continuously-drained stderr capture (PLAN Slice 1B).
fn spawn_dsh(plan: &dsh_launcher::LaunchPlan) -> std::io::Result<(Child, StderrCapture)> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args)
        .envs(&plan.env)
        .current_dir(&plan.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // New process group: pgid == pid, so `kill -- -pid` kills dsh + workers.
    cmd.process_group(0);
    let mut child = cmd.spawn()?;
    let stderr = child.stderr.take().expect("stderr was piped");
    let capture = StderrCapture::attach(stderr);
    Ok((child, capture))
}

/// Send SIGTERM to the whole process group, poll up to KILL_GRACE, then
/// SIGKILL any survivors. Returns whether the group is confirmed gone (PLAN
/// 验收点 4: ps tree verification).
///
/// Polling (instead of an unconditional sleep) keeps app quit instant when dsh
/// goes down promptly.
fn terminate_tree(pgid: i32) -> bool {
    let _ = Command::new("kill")
        .args(["-TERM", "--", &format!("-{pgid}")])
        .output();
    let deadline = Instant::now() + KILL_GRACE;
    while Instant::now() < deadline {
        if !pgid_alive(pgid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .output();
    let kill_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < kill_deadline {
        if !pgid_alive(pgid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !pgid_alive(pgid)
}

/// Heuristic: is any process still in the group? Use `pgrep -g <pgid>`.
fn pgid_alive(pgid: i32) -> bool {
    match Command::new("pgrep")
        .arg("-g")
        .arg(pgid.to_string())
        .output()
    {
        Ok(out) => !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

/// Continuously drained stderr ring buffer (last ~16 KiB).
///
/// Spawning with `Stdio::piped()` and only reading after exit deadlocks the
/// child once it fills the OS pipe buffer (~64 KiB) — dsh freezes mid-write
/// and looks hung. A dedicated reader thread keeps the pipe empty at all times
/// and retains a bounded tail for post-mortem diagnostics.
struct StderrCapture {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl StderrCapture {
    const CAPACITY: usize = 16 * 1024;

    fn attach(mut stderr: ChildStderr) -> Self {
        let buf = Arc::new(Mutex::new(Vec::with_capacity(Self::CAPACITY)));
        let writer = buf.clone();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut b = writer.lock().unwrap();
                        b.extend_from_slice(&chunk[..n]);
                        if b.len() > Self::CAPACITY {
                            let drop = b.len() - Self::CAPACITY;
                            b.drain(..drop);
                        }
                    }
                }
            }
        });
        StderrCapture { buf }
    }

    fn tail(&self) -> String {
        let b = self.buf.lock().unwrap();
        String::from_utf8_lossy(&b[..]).trim().to_string()
    }
}

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
            let _ = w.eval(format!("window.location.href = '{origin}/'"));
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
    let _ = window.eval(format!("window.location.href = '{url}'"));
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

/// Locate dsh and drive the supervisor: up to MAX_ATTEMPTS lifecycles, with a
/// restart-once on unexpected exit / connection loss (PLAN 门禁 E). Clean
/// exits (the Web UI shutdown button) surface an informative page instead.
fn run_launcher(app: tauri::AppHandle) {
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

    let mut last_problem = String::new();
    for attempt in 0..MAX_ATTEMPTS {
        let lifecycle = supervise_once(&app, state, &settings, &candidate);
        match lifecycle {
            Lifecycle::Aborted => return,
            Lifecycle::StartFailed => return,
            Lifecycle::Exited(status) if status.success() => {
                // Clean exit — typically the dsh Web UI 关机 button. Don't
                // restart behind the user's back; tell them and offer restart.
                show_detail_page(
                    &app,
                    state,
                    ST_ERROR,
                    "dsh 已正常退出（可能通过 Web 界面关机）。点击「重新启动」可再次拉起。",
                );
                return;
            }
            Lifecycle::Exited(status) => {
                last_problem = format!("dsh 进程异常退出（{status}）");
            }
            Lifecycle::ConnectionLost(reason) => {
                last_problem = reason;
            }
        }
        if attempt + 1 < MAX_ATTEMPTS {
            dsh_launcher::logging::error(&format!("{last_problem}；按门禁 E 重启一次"));
            show_detail_page(&app, state, ST_RESTARTING, &last_problem);
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    show_detail_page(
        &app,
        state,
        ST_ERROR,
        &format!("{last_problem}；重启后仍不可用。请查看日志或重新选择 dsh。日志位于 ~/Library/Logs/{IDENTIFIER}/。"),
    );
}

// ----- Tauri commands exposed to the setup/error page -----

#[tauri::command]
fn get_status(state: tauri::State<LauncherState>) -> StatusPayload {
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
fn select_dsh(
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
#[tauri::command]
fn restart_dsh(app: tauri::AppHandle, state: tauri::State<LauncherState>) -> Result<(), String> {
    dsh_launcher::logging::info("用户请求重新启动 dsh");
    stop_current_run(&state);
    std::thread::spawn(move || run_launcher(app));
    Ok(())
}

/// Outcome returned to the injected archived-session panel for one batch op.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveOpResponse {
    /// Per-id outcome (changed archive set, pruned refs, deleted dirs, …).
    report: dsh_launcher::ArchiveOpsReport,
    /// True when the launcher owns the dsh child and has requested a restart
    /// so the edits take effect; false in the fast path (dsh started
    /// externally) where the caller must restart dsh manually.
    restart_triggered: bool,
}

/// If the launcher owns the dsh child (normal supervised path), terminate its
/// process group on a background thread; the supervisor's restart-once logic
/// then respawns dsh from the updated storage. Returns whether a restart was
/// armed. In the fast path (we connected to an externally started dsh) we
/// never kill a process we don't own, so this returns false and the UI tells
/// the user to restart dsh manually.
fn arm_dsh_restart(state: &LauncherState) -> bool {
    let pgid = match *state.pgid.lock().unwrap() {
        Some(p) => p,
        None => return false,
    };
    std::thread::spawn(move || {
        terminate_tree(pgid);
    });
    true
}

/// 撤销归档：把选中的会话从 dsh 的归档集合中移除。
#[tauri::command]
fn restore_archived_sessions(
    state: tauri::State<LauncherState>,
    ids: Vec<String>,
) -> Result<ArchiveOpResponse, String> {
    let home = dsh_launcher::dsh_home();
    let report = dsh_launcher::restore_archived(&home, &ids)?;
    Ok(ArchiveOpResponse {
        report,
        restart_triggered: arm_dsh_restart(&state),
    })
}

/// 物理删除：从磁盘移除选中的会话目录，并从归档集合与工作区成员中清除。
#[tauri::command]
fn delete_archived_sessions(
    state: tauri::State<LauncherState>,
    ids: Vec<String>,
) -> Result<ArchiveOpResponse, String> {
    let home = dsh_launcher::dsh_home();
    let report = dsh_launcher::delete_archived(&home, &ids)?;
    Ok(ArchiveOpResponse {
        report,
        restart_triggered: arm_dsh_restart(&state),
    })
}

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
            delete_archived_sessions
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
