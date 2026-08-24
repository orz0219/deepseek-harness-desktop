  // ===== 归档管理器面板 =====
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
    if (rest) { rest.disabled = n === 0 || !hasTauri(); rest.textContent = '还原对话（' + n + '）'; }
    if (del) { del.disabled = n === 0 || !hasTauri(); del.textContent = '永久删除（' + n + '）'; }
  }

  var lastRows = [];
  var searchTerm = '';
  var collapsedGroups = {};
  var CARET_SVG = '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6l4 4 4-4"/></svg>';

  function applySearch(rows) {
    var q = (searchTerm || '').trim().toLowerCase();
    if (!q) return rows;
    return rows.filter(function (r) {
      return (r.title || '').toLowerCase().indexOf(q) !== -1
        || (r.meta || '').toLowerCase().indexOf(q) !== -1;
    });
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
    // 按工作目录（cwd）分组：相同工作目录的会话归到一个可折叠分组下，形成树状结构
    var groups = {};
    var order = [];
    rows.forEach(function (row) {
      var k = row.cwd || '(未知工作目录)';
      if (!groups[k]) { groups[k] = []; order.push(k); }
      groups[k].push(row);
    });
    order.sort();
    order.forEach(function (k) {
      var gitems = groups[k];
      var collapsed = !!collapsedGroups[k];
      var group = document.createElement('div');
      group.className = 'dsh-ap-group' + (collapsed ? ' collapsed' : '');
      group.setAttribute('data-cwd', k);
      var head = document.createElement('div');
      head.className = 'dsh-ap-grouphead';
      head.title = k === '(未知工作目录)' ? '未记录工作目录' : k;
      var caret = document.createElement('span');
      caret.className = 'dsh-ap-caret';
      caret.innerHTML = CARET_SVG;
      var name = document.createElement('span');
      name.className = 'dsh-ap-groupname';
      name.textContent = k;
      var cnt = document.createElement('span');
      cnt.className = 'dsh-ap-groupcount';
      cnt.textContent = gitems.length;
      head.appendChild(caret);
      head.appendChild(name);
      head.appendChild(cnt);
      // 整组操作：悬停分组头浮现「还原 / 删除」按钮，作用于该工作目录下的全部会话
      var anyRunning = gitems.some(function (r) { return !!r.running; });
      var acts = document.createElement('span');
      acts.className = 'dsh-ap-groupacts';
      var rBtn = document.createElement('button');
      rBtn.type = 'button';
      rBtn.className = 'dsh-ap-groupact dsh-ap-group-restore';
      rBtn.textContent = '还原';
      rBtn.title = '还原该工作目录下的全部会话';
      if (!hasTauri() || anyRunning) {
        rBtn.disabled = true;
        rBtn.title = !hasTauri() ? '当前页面不在启动器窗口内，无法操作' : '有会话正在运行，暂不可操作';
      }
      rBtn.addEventListener('click', function (e) {
        e.stopPropagation();
        restoreArchived(gitems.map(function (r) { return r.id; }));
      });
      var dBtn = document.createElement('button');
      dBtn.type = 'button';
      dBtn.className = 'dsh-ap-groupact dsh-ap-group-delete';
      dBtn.textContent = '删除';
      dBtn.title = '永久删除该工作目录下的全部会话';
      if (!hasTauri() || anyRunning) {
        dBtn.disabled = true;
        dBtn.title = !hasTauri() ? '当前页面不在启动器窗口内，无法操作' : '有会话正在运行，暂不可操作';
      }
      dBtn.addEventListener('click', function (e) {
        e.stopPropagation();
        deleteArchived(gitems.map(function (r) { return r.id; }));
      });
      acts.appendChild(rBtn);
      acts.appendChild(dBtn);
      head.appendChild(acts);
      head.addEventListener('click', function () {
        collapsedGroups[k] = !collapsedGroups[k];
        group.classList.toggle('collapsed', collapsedGroups[k]);
      });
      group.appendChild(head);
      var glist = document.createElement('div');
      glist.className = 'dsh-ap-grouplist';
      gitems.forEach(function (row) { glist.appendChild(buildRow(row)); });
      group.appendChild(glist);
      list.appendChild(group);
    });
    updateSelection();
  }

  function buildRow(row) {
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
    return lab;
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
        var rawArchived = wsValue.archivedSessionIds || [];
        var overrides = loadOverrides();
        var liveOverrides = rawArchived.filter(function (id) { return overrides.has(id); });
        saveOverrides(new Set(liveOverrides));
        var archivedIds = rawArchived.filter(function (id) { return !overrides.has(id); });
        var sessItems = (sessResult && sessResult.result && sessResult.result.value && sessResult.result.value.items) || [];
        var sessById = {};
        sessItems.forEach(function (s) { sessById[s.sessionId] = s; });
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
          if (s.origin === 'subagent') return;
          if (s.blank) return;
          var projTitle = s.projections && s.projections.values && s.projections.values.title;
          var title = projTitle || basename(s.cwd) || '(无标题)';
          var wsName = wsOf[id] || (s.cwd ? basename(s.cwd) : '') || '未知工作区';
          var meta = wsName;
          rows.push({
            id: id,
            title: title,
            meta: meta,
            time: fmtTime(s.updatedAt),
            running: !!s.running,
            cwd: s.cwd
          });
        });
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


