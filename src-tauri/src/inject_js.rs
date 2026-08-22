//! 注入到 dsh webview 的脚本：在 dsh 侧边栏添加「归档所有对话」按钮与归档管理器面板。
//! 该脚本通过 MutationObserver 自愈、幂等重装，由 launcher 反复 eval 直到 dsh SPA 挂载侧边栏。
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
pub const INJECT_JS: &str = r##"(
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
    btn.setAttribute('aria-label', '全部归档');
  }
  function setIdle2(btn) {
    if (!btn) return;
    btn.innerHTML = ICON_CLOCK;
    btn.setAttribute('aria-label', '归档历史');
  }
  // 自定义 tooltip：对齐 dsh 原生 Tooltip 组件（深色底板 + 白字、底部居中、
  // 500ms 悬停延迟）。原生 title 属性会弹出浏览器默认黄框，与 dsh 不一致，
  // 故这些按钮改用 aria-label 作为文本源 + 本自定义气泡（颜色直接复用 dsh 的
  // CSS 变量，浅色/深色主题下与 dsh 自身提示完全一致）。
  var tipTimer = null, currentTip = null, tipBtn = null, tipInterval = null;
  function hideTip() {
    if (tipTimer) { clearTimeout(tipTimer); tipTimer = null; }
    if (tipInterval) { clearInterval(tipInterval); tipInterval = null; }
    if (currentTip) { currentTip.remove(); currentTip = null; }
    tipBtn = null;
  }
  function showTip(btn) {
    hideTip();
    var r = btn.getBoundingClientRect();
    var side = btn.getAttribute('data-tip-side') || 'bottom';
    var tip = document.createElement('div');
    tip.className = 'dsh-tip';
    tip.setAttribute('data-side', side);
    tip.setAttribute('role', 'tooltip');
    tip.textContent = btn.getAttribute('aria-label') || '';
    document.body.appendChild(tip);
    if (side === 'bottom') {
      tip.style.left = (r.left + r.width / 2) + 'px';
      tip.style.top = (r.bottom + 8) + 'px';
    } else if (side === 'right') {
      tip.style.left = (r.right + 10) + 'px';
      tip.style.top = (r.top + r.height / 2) + 'px';
    } else {
      tip.style.left = (r.left + r.width / 2) + 'px';
      tip.style.top = (r.top - 8) + 'px';
    }
    currentTip = tip;
    tipBtn = btn;
    // 归档中等状态会更新 aria-label，气泡保持同步
    tipInterval = setInterval(function () {
      if (currentTip && tipBtn) currentTip.textContent = tipBtn.getAttribute('aria-label') || '';
    }, 200);
  }
  function attachTooltip(btn, side) {
    if (!btn) return;
    btn.setAttribute('data-tip-side', side || 'bottom');
    btn.addEventListener('mouseenter', function () {
      if (tipTimer) clearTimeout(tipTimer);
      tipTimer = setTimeout(function () { showTip(btn); }, 500);
    });
    btn.addEventListener('mouseleave', hideTip);
    btn.addEventListener('focus', function () { showTip(btn); });
    btn.addEventListener('blur', hideTip);
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
      '.dsh-ap-head{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:16px 18px 10px;}' +
      '.dsh-ap-ttl{font-size:15px;font-weight:600;flex:none;}' +
      // 面板内搜索框：对齐 dsh 工作区展开态搜索框（圆角描边胶囊 + 13px 输入，
      // 复用 dsh CSS 变量，浅色/深色主题下与 dsh 自身搜索框一致）。
      '.dsh-ap-searchwrap{display:flex;align-items:center;gap:6px;flex:1;min-width:0;height:30px;padding:0 10px;box-sizing:border-box;border:1px solid var(--dsw-alias-border-l2,#e5e7eb);border-radius:10px;background:transparent;color:var(--dsw-alias-label-tertiary,#9ca3af);}' +
      '.dsh-ap-searchwrap:focus-within{border-color:var(--dsw-alias-border-focused,#3b6cf6);}' +
      '.dsh-ap-searchicon{flex:none;}' +
      '.dsh-ap-search{flex:1;min-width:0;border:none;outline:none;background:transparent;font-size:13px;line-height:18px;color:var(--dsw-alias-label-primary,#1f2937);}' +
      '.dsh-ap-search::placeholder{color:var(--dsw-alias-label-tertiary,#9ca3af);}' +
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
      '@keyframes dsh-ap-rot{to{transform:rotate(360deg);}}' +
      // ---- 自定义 tooltip（对齐 dsh 原生 Tooltip：深色底板白字、底部居中、淡入）----
      '.dsh-tip{position:fixed;z-index:2147483640;width:max-content;max-width:50vw;padding:3px 7px;border-radius:8px;background:var(--dsw-alias-tooltip-bg);color:var(--dsw-static-neutral-bluish-00);font-size:13px;line-height:20px;white-space:pre-line;overflow-wrap:break-word;pointer-events:none;font-family:inherit;animation:dsh-tip-in 150ms ease-in-out;}' +
      '.dsh-tip[data-side="bottom"]{transform:translateX(-50%);}' +
      '.dsh-tip[data-side="right"]{transform:translateY(-50%);}' +
      '.dsh-tip[data-side="top"]{transform:translate(-50%,-100%);}' +
      '@keyframes dsh-tip-in{from{opacity:0;}}' +
    (document.head || document.documentElement).appendChild(st);
  }
  function addButtonAfter(ref, id, label, onClick) {
    var b = document.createElement('button');
    b.id = id;
    b.type = 'button';
    b.setAttribute('aria-label', label);
    b.style.cssText = 'flex:none;display:inline-flex;align-items:center;justify-content:center;box-sizing:border-box;width:28px;height:28px;margin:0 2px 0 0;padding:0;border:none;border-radius:50%;cursor:pointer;';
    b.addEventListener('click', onClick);
    ref.parentNode.insertBefore(b, ref.nextSibling);
    return b;
  }
  function isRail() {
    var a = findAnchor();
    if (!a) return false;
    // 宽模式搜索按钮包在 searchSlot 内；rail（收起）模式没有 searchSlot。
    return !a.closest('[class*="searchSlot"]');
  }
  function inject() {
    ensureStyle();
    // 收起（rail）模式下原生只保留「添加工作区」＋ 与搜索两个图标；注入的
    // 「全部归档 / 归档历史」在此态下移除，避免破坏 rail 的 36x36 图标节奏，
    // 同时保证「添加工作区」按钮位置正常显示。展开后 inject 会重新创建它们。
    if (isRail()) {
      var ex = document.getElementById(BTN_ID); if (ex && ex.parentNode) ex.parentNode.removeChild(ex);
      var ex2 = document.getElementById(BTN_ID2); if (ex2 && ex2.parentNode) ex2.parentNode.removeChild(ex2);
      return;
    }
    var anchor = findAnchor();
    var arch = document.getElementById(BTN_ID);
    if (!arch) {
      if (!anchor || !anchor.parentNode) return;
      arch = document.createElement('button');
      arch.id = BTN_ID;
      arch.type = 'button';
      setIdle(arch);
      attachTooltip(arch, 'bottom');
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
      archv = addButtonAfter(arch, BTN_ID2, '归档历史', function () { openArchivedPanel(); });
      setIdle2(archv);
      attachTooltip(archv, 'bottom');
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
      if (!items.length) { btn.setAttribute('aria-label', '没有可归档的对话'); setTimeout(function () { setIdle(btn); }, 1500); return; }
      showConfirm(items.length, function () { executeArchive(btn, items); });
    }).catch(function () { btn.setAttribute('aria-label', '读取会话失败'); setTimeout(function () { setIdle(btn); }, 1500); });
  }
  function executeArchive(btn, items) {
    if (!items || !items.length) { setIdle(btn); return; }
    var done = 0, fail = 0, total = items.length;
    btn.setAttribute('aria-label', '归档中 0/' + total);
    var seq = Promise.resolve();
    items.forEach(function (it) {
      seq = seq.then(function () {
        return rpc('workspace.archiveSession', { sessionId: it.sessionId })
          .then(function (r) { if (r && r.result && r.result.ok) { done++; removeOverride(it.sessionId); } else fail++; })
          .catch(function () { fail++; })
          .then(function () { var msg = '归档中 ' + (done + fail) + '/' + total; btn.setAttribute('aria-label', msg); });
      });
    });
    seq.then(function () {
      var msg = '已归档 ' + done + ' 个（失败 ' + fail + '）';
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
  // 归档覆盖缓存：记录「本 dsh 实例中已被撤销/删除、但 dsh 内存尚未感知」的
  // 会话 id。撤销/删除走 Rust 直接改磁盘，绕过了 dsh 内存；且为了不打断正在运行
  // 的对话，我们不重启 dsh，于是面板重读 workspace.list 拿到的 archivedSessionIds
  // 仍是旧值。用此缓存做差集，保证归档历史面板显示正确。dsh 重启后磁盘即为准，
  // 缓存相减为空操作并自动清理，无副作用。
  var OVERRIDE_KEY = 'dsh-desktop-archive-overrides';
  function loadOverrides() {
    try { return new Set(JSON.parse(localStorage.getItem(OVERRIDE_KEY) || '[]')); }
    catch (e) { return new Set(); }
  }
  function saveOverrides(set) {
    try { localStorage.setItem(OVERRIDE_KEY, JSON.stringify(Array.from(set))); } catch (e) {}
  }
  function applyOverrides(ids) {
    if (!ids || !ids.length) return;
    var o = loadOverrides();
    ids.forEach(function (id) { o.add(id); });
    saveOverrides(o);
  }
  function removeOverride(id) {
    var o = loadOverrides();
    if (o.delete(id)) saveOverrides(o);
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
    if (rest) { rest.disabled = n === 0 || !hasTauri(); rest.textContent = '还原对话（' + n + '）'; }
    if (del) { del.disabled = n === 0 || !hasTauri(); del.textContent = '永久删除（' + n + '）'; }
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
      empty.textContent = (searchTerm && searchTerm.trim()) ? '未找到匹配的对话' : '暂无归档会话';
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
  var lastRows = [];
  var searchTerm = '';
  function applySearch(rows) {
    var q = (searchTerm || '').trim().toLowerCase();
    if (!q) return rows;
    return rows.filter(function (r) {
      return (r.title || '').toLowerCase().indexOf(q) !== -1
        || (r.meta || '').toLowerCase().indexOf(q) !== -1; // meta 即所属文件夹/工作区名
    });
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
        // 用覆盖缓存（已撤销/删除但 dsh 内存未更新的 id）做差集得到真正应显示的
        // 归档集；同时把缓存里已不在 dsh 报告集合中的 id 清掉（dsh 重启后磁盘为准，
        // 差集为空操作，缓存自动收敛）。
        var rawArchived = wsValue.archivedSessionIds || [];
        var overrides = loadOverrides();
        var liveOverrides = rawArchived.filter(function (id) { return overrides.has(id); });
        saveOverrides(new Set(liveOverrides));
        var archivedIds = rawArchived.filter(function (id) { return !overrides.has(id); });
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
        lastRows = rows;
        var filtered = applySearch(rows);
        renderArchived(filtered, filtered.length);
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
          '<div class="dsh-ap-searchwrap">' +
            '<svg class="dsh-ap-searchicon" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/></svg>' +
            '<input type="text" class="dsh-ap-search" placeholder="搜索对话标题或文件夹" aria-label="搜索对话标题或文件夹">' +
          '</div>' +
          '<button type="button" class="dsh-ap-close" aria-label="关闭">' +
            '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>' +
          '</button>' +
        '</div>' +
        '<div class="dsh-ap-toolbar">' +
          '<label class="dsh-ap-selectall"><input type="checkbox">全选</label>' +
          '<span class="dsh-ap-count"></span>' +
          '<button type="button" class="dsh-ap-refresh">重启 DSH</button>' +
        '</div>' +
        '<div class="dsh-ap-list"></div>' +
        '<div class="dsh-ap-status" style="display:none"></div>' +
        '<div class="dsh-ap-actions">' +
          '<button type="button" class="dsh-ap-restore" disabled>还原对话（0）</button>' +
          '<button type="button" class="dsh-ap-delete" disabled>永久删除（0）</button>' +
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
    overlay.querySelector('.dsh-ap-refresh').addEventListener('click', restartDshWeb);
    overlay.querySelector('.dsh-ap-restore').addEventListener('click', runRestore);
    overlay.querySelector('.dsh-ap-delete').addEventListener('click', runDelete);
    searchTerm = '';
    var searchInput = overlay.querySelector('.dsh-ap-search');
    searchInput.addEventListener('input', function () {
      searchTerm = searchInput.value || '';
      var filtered = applySearch(lastRows);
      renderArchived(filtered, filtered.length);
    });
    if (!hasTauri()) {
      setStatus('当前页面不在启动器窗口内，仅可查看归档；撤销 / 删除需要启动器支持。', 'err');
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
        '<div class="dsh-am-title">永久删除会话</div>' +
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
    return label + ' ' + n + ' 个会话，重启 DSH生效。';
  }
  // 就地移除面板里被撤销/删除的若干行，不重新拉取整个列表。已知具体 id，
  // 直接把这些行从 DOM 摘除并更新计数即可；覆盖缓存（applyOverrides）已保证
  // 之后再次打开/刷新面板时它们不会复活。
  function removeArchivedRows(ids) {
    var p = panelEl(); if (!p) return;
    var set = new Set(ids || []);
    if (!set.size) return;
    var rows = p.querySelectorAll('.dsh-ap-row');
    var removed = 0;
    rows.forEach(function (row) {
      var cb = row.querySelector('input[type=checkbox]');
      var id = cb && cb.getAttribute('data-id');
      if (id && set.has(id)) { row.remove(); removed++; }
    });
    if (!removed) return;
    var countEl = p.querySelector('.dsh-ap-count');
    if (countEl) {
      var m = /共\s*(\d+)\s*个/.exec(countEl.textContent || '');
      var n = m ? parseInt(m[1], 10) : 0;
      countEl.textContent = '共 ' + Math.max(0, n - removed) + ' 个';
    }
    var list = p.querySelector('.dsh-ap-list');
    if (list && !list.querySelector('.dsh-ap-row')) {
      list.innerHTML = '<div class="dsh-ap-empty">暂无归档会话</div>';
    }
    updateSelection();
  }

  // 复用「全部归档」里的 running 检测：直接问 session.list，看是否有会话
  // 正在运行（生成中），避免重启打断正在跑的对话。
  function hasRunningSession() {
    return rpc('session.list', {}).then(function (res) {
      var items = (res && res.result && res.result.value && res.result.value.items) || [];
      return items.some(function (s) { return !!s.running; });
    });
  }

  // 手动「重启 DSH Web」：由用户主动点击。先检测是否有对话在跑——若有则
  // 直接阻止（不重启），避免打断。确认无运行后，走轻量级重启：杀掉 launcher
  // 拥有的 dsh 进程组，监督器自动重生并重载页面，完成一次完整重新同步；外部
  // dsh 无法杀，返回 false 提示手动重启。
  function restartDshWeb() {
    setStatus('正在检查是否有对话在运行…');
    hasRunningSession().then(function (running) {
      if (running) {
        setStatus('有对话正在运行，已阻止重启 DSH；请等待其结束后再试。', 'err');
        return;
      }
      setStatus('正在重启 DSH Web…');
      tauriInvoke('restart_dsh', {})
        .then(function () {
          // restart_dsh 会停止监督器并重生 dsh、重载页面，无需额外操作
        })
        .catch(function (e) {
          setStatus('重启失败：' + (e && e.message ? e.message : e), 'err');
        });
    }).catch(function () {
      setStatus('检查会话状态失败，已取消重启。', 'err');
    });
  }
  function runRestore() {
    var ids = selectedIds();
    if (!ids.length || panelEl().querySelector('.dsh-ap-restore').disabled) return;
    panelEl().querySelector('.dsh-ap-restore').disabled = true;
    setStatus('正在还原对话…');
    tauriInvoke('restore_archived_sessions', { ids: ids })
      .then(function (res) {
        if (!panelEl()) return;
        setStatus(opOutcome(res, '已还原'), 'ok');
        applyOverrides(ids);
        removeArchivedRows(ids);
      })
      .catch(function (e) {
        if (!panelEl()) return;
        setStatus('还原对话失败：' + (e && e.message ? e.message : e), 'err');
        updateSelection();
      });
  }
  function runDelete() {
    var ids = selectedIds();
    if (!ids.length || panelEl().querySelector('.dsh-ap-delete').disabled) return;
    showDangerConfirm(ids.length, function () {
      if (!panelEl()) return;
      panelEl().querySelector('.dsh-ap-delete').disabled = true;
      setStatus('正在永久删除…');
      tauriInvoke('delete_archived_sessions', { ids: ids })
        .then(function (res) {
          if (!panelEl()) return;
          var r = res && res.report ? res.report : {};
          var deleted = (r.deletedDirs != null) ? r.deletedDirs : 0;
          var msg = (res && res.restartTriggered)
            ? '已永久删除 ' + deleted + ' 个会话目录，DSH 正在重启以生效…'
            : '已永久删除 ' + deleted + ' 个会话目录。归档面板已即时更新；DSH 侧边栏将在下次重启后同步（当前对话不受影响）。';
          setStatus(msg, 'ok');
          applyOverrides(ids);
          removeArchivedRows(ids);
        })
        .catch(function (e) {
          if (!panelEl()) return;
          setStatus('永久删除失败：' + (e && e.message ? e.message : e), 'err');
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
    window.__dshArchiveObserver.observe(document.body, { childList: true, attributes: true, subtree: true });
  }
}
)()"##;
