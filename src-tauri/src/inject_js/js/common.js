//! 共享工具函数
//!
//! 提供注入脚本中各模块共用的基础功能：
//! - RPC 调用（与 dsh API 通信）
//! - DOM 锚点查找（定位侧边栏元素）
//! - 样式注入（确保 CSS 变量可用）
//! - Tooltip 工具（自定义提示框）
//! - Tauri 命令调用封装

/// 共享工具函数的 JavaScript 代码

  // ===== RPC 调用 =====
  function rpc(method, payload) {
    var id = (window.crypto && crypto.randomUUID)
      ? crypto.randomUUID()
      : ('r-' + Date.now() + '-' + Math.random().toString(16).slice(2));
    return fetch(window.location.origin + '/api/' + method, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ type: 'client-request', rpcId: id, method: method, payload: payload || {} })
    }).then(function (r) { return r.json(); });
  }

  // ===== Tauri 命令调用 =====
  function tauriInvoke(cmd, args) {
    var t = window.__TAURI__;
    var fn = t && t.core && t.core.invoke;
    if (!fn) return Promise.reject(new Error('no-tauri'));
    return fn(cmd, args);
  }

  function hasTauri() {
    return !!(window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke);
  }

  // ===== DOM 锚点查找 =====
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

  function isRail() {
    var a = findAnchor();
    if (!a) return false;
    // 宽模式搜索按钮包在 searchSlot 内；rail（收起）模式没有 searchSlot。
    return !a.closest('[class*="searchSlot"]');
  }

  // ===== 样式注入 =====
  function ensureStyle(id, css) {
    if (document.getElementById(id)) return;
    var st = document.createElement('style');
    st.id = id;
    st.textContent = css;
    (document.head || document.documentElement).appendChild(st);
  }

  // ===== Tooltip 工具 =====
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
    } else if (side === 'left') {
      tip.style.left = (r.left - 10) + 'px';
      tip.style.top = (r.top + r.height / 2) + 'px';
      tip.style.transform = 'translateX(-100%) translateY(-50%)';
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

  // ===== DOM 工具 =====
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

  // ===== 工具函数 =====
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

  // ===== Tooltip 样式 =====
  var TOOLTIP_CSS = 
    '.dsh-tip{position:fixed;z-index:2147483640;width:max-content;max-width:50vw;padding:3px 7px;border-radius:8px;background:var(--dsw-alias-tooltip-bg);color:var(--dsw-static-neutral-bluish-00);font-size:13px;line-height:20px;white-space:pre-line;overflow-wrap:break-word;pointer-events:none;font-family:inherit;animation:dsh-tip-in 150ms ease-in-out;}' +
    '.dsh-tip[data-side="bottom"]{transform:translateX(-50%);}' +
    '.dsh-tip[data-side="right"]{transform:translateY(-50%);}' +
    '.dsh-tip[data-side="left"]{transform:translateX(-100%) translateY(-50%);}' +
    '.dsh-tip[data-side="top"]{transform:translate(-50%,-100%);}' +
    '@keyframes dsh-tip-in{from{opacity:0;}}';

  // 确保 tooltip 样式存在
  ensureStyle('dsh-tip-style', TOOLTIP_CSS);

