  // ===== 归档按钮状态 =====
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

  // ===== 归档样式 =====
  var ARCHIVE_CSS =
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
    // ---- 归档清单弹窗（带锁定/排除）----
    '.dsh-am-card.list{width:460px;max-width:92vw;display:flex;flex-direction:column;max-height:82vh;}' +
    '.dsh-am-toolbar{display:flex;align-items:center;gap:8px;margin:2px 0 6px;}' +
    '.dsh-am-mini{height:26px;padding:0 10px;border:1px solid #e5e7eb;background:#ffffff;color:#374151;border-radius:8px;font-size:12px;cursor:pointer;font-family:inherit;}' +
    '.dsh-am-mini:hover{background:#f9fafb;border-color:#cbd5e1;}' +
    '.dsh-am-minisum{font-size:12px;color:#9ca3af;font-variant-numeric:tabular-nums;}' +
    '.dsh-am-list{flex:1;min-height:0;overflow-y:auto;margin:2px -4px 0;padding:2px 4px;}' +
    '.dsh-am-row{display:flex;align-items:center;gap:10px;padding:8px 10px;border-radius:10px;cursor:pointer;transition:background .12s ease;}' +
    '.dsh-am-row:hover{background:#f9fafb;}' +
    '.dsh-am-row.locked{background:#fffbeb;}' +
    '.dsh-am-lock{flex:none;width:28px;height:28px;border:none;border-radius:8px;background:transparent;color:#c4c9d4;display:inline-flex;align-items:center;justify-content:center;cursor:pointer;transition:background .15s ease,color .15s ease;}' +
    '.dsh-am-lock:hover{background:#f1f3f7;color:#6b7280;}' +
    '.dsh-am-lock.locked{color:#3b6cf6;background:#eef2ff;}' +
    '.dsh-am-lock svg{display:block;}' +
    '.dsh-am-rowbody{flex:1;min-width:0;}' +
    '.dsh-am-rowtitle{font-size:13px;color:#1f2937;font-weight:500;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
    '.dsh-am-rowmeta{font-size:11px;color:#9ca3af;margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
    '.dsh-am-skip{flex:none;font-size:11px;color:#b45309;background:#fef3c7;border-radius:9px;padding:1px 8px;}' +
    '.dsh-am-summary{font-size:12px;color:#6b7280;padding:10px 2px 2px;line-height:1.5;}' +
    '.dsh-am-summary b{color:#374151;font-weight:600;}' +
    // ---- 归档管理器面板（时钟按钮打开）----
    '#dsh-archived-panel{position:fixed;inset:0;z-index:2147483646;display:flex;align-items:center;justify-content:center;background:rgba(17,24,39,.35);}' +
    '.dsh-ap-card{box-sizing:border-box;width:620px;max-width:92vw;max-height:76vh;display:flex;flex-direction:column;background:#ffffff;border:1px solid #e5e7eb;border-radius:14px;box-shadow:0 12px 40px rgba(31,41,55,.18);font-family:inherit;color:#1f2937;overflow:hidden;}' +
    '.dsh-ap-head{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:16px 18px 10px;}' +
    '.dsh-ap-ttl{font-size:15px;font-weight:600;flex:none;}' +
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
    // ---- 按工作目录分组的树状结构 ----
    '.dsh-ap-group{margin:2px 0;}' +
    '.dsh-ap-grouphead{display:flex;align-items:center;gap:6px;padding:6px 8px;border-radius:8px;cursor:pointer;color:#374151;font-size:12px;font-weight:600;user-select:none;}' +
    '.dsh-ap-grouphead:hover{background:#f3f4f6;}' +
    '.dsh-ap-caret{flex:none;width:12px;height:12px;color:#9ca3af;transition:transform .15s ease;}' +
    '.dsh-ap-group.collapsed .dsh-ap-caret{transform:rotate(-90deg);}' +
    '.dsh-ap-groupname{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
    '.dsh-ap-groupcount{flex:none;font-size:11px;color:#6b7280;background:#f3f4f6;border-radius:9px;padding:0 7px;}' +
    '.dsh-ap-group.collapsed .dsh-ap-grouplist{display:none;}' +
    // ---- 分组头上的整组操作（悬停浮现）----
    '.dsh-ap-groupacts{flex:none;display:inline-flex;align-items:center;gap:6px;opacity:0;transition:opacity .15s ease;}' +
    '.dsh-ap-grouphead:hover .dsh-ap-groupacts{opacity:1;}' +
    '.dsh-ap-groupact{font-size:12px;line-height:1;padding:3px 9px;border-radius:6px;border:1px solid #e5e7eb;background:#ffffff;color:#374151;cursor:pointer;font-family:inherit;}' +
    '.dsh-ap-groupact:hover:not(:disabled){background:#f9fafb;border-color:#cbd5e1;}' +
    '.dsh-ap-groupact:disabled{opacity:.45;cursor:not-allowed;}' +
    '.dsh-ap-group-delete{color:#b91c1c;border-color:#fecaca;}' +
    '.dsh-ap-group-delete:hover:not(:disabled){background:#fef2f2;border-color:#b91c1c;}';


