  // ===== 危险二次确认 =====
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

  function removeArchivedRows(ids) {
    var p = panelEl(); if (!p) return;
    var set = new Set(ids || []);
    if (!set.size) return;
    var list = p.querySelector('.dsh-ap-list');
    if (!list) return;
    // 逐组移除行；整组删空时一并移除该组（更新组计数徽标与总数）
    var groups = list.querySelectorAll('.dsh-ap-group');
    if (groups.length) {
      groups.forEach(function (group) {
        var glist = group.querySelector('.dsh-ap-grouplist');
        if (!glist) return;
        glist.querySelectorAll('.dsh-ap-row').forEach(function (row) {
          var cb = row.querySelector('input[type=checkbox]');
          var id = cb && cb.getAttribute('data-id');
          if (id && set.has(id)) row.remove();
        });
        var remaining = glist.querySelectorAll('.dsh-ap-row').length;
        var cnt = group.querySelector('.dsh-ap-groupcount');
        if (cnt) cnt.textContent = remaining;
        if (remaining === 0) group.remove();
      });
    } else {
      list.querySelectorAll('.dsh-ap-row').forEach(function (row) {
        var cb = row.querySelector('input[type=checkbox]');
        var id = cb && cb.getAttribute('data-id');
        if (id && set.has(id)) row.remove();
      });
    }
    var allRows = list.querySelectorAll('.dsh-ap-row');
    var countEl = p.querySelector('.dsh-ap-count');
    if (countEl) countEl.textContent = '共 ' + allRows.length + ' 个';
    if (allRows.length === 0) list.innerHTML = '<div class="dsh-ap-empty">暂无归档会话</div>';
    updateSelection();
  }

  function hasRunningSession() {
    return rpc('session.list', {}).then(function (res) {
      var items = (res && res.result && res.result.value && res.result.value.items) || [];
      return items.some(function (s) { return !!s.running; });
    });
  }

  function restartDshWeb() {
    setStatus('正在检查是否有对话在运行…');
    hasRunningSession().then(function (running) {
      if (running) {
        setStatus('有对话正在运行，已阻止重启 DSH；请等待其结束后再试。', 'err');
        return;
      }
      setStatus('正在重启 DSH Web…');
      tauriInvoke('restart_dsh', {})
        .then(function () {})
        .catch(function (e) {
          setStatus('重启失败：' + (e && e.message ? e.message : e), 'err');
        });
    }).catch(function () {
      setStatus('检查会话状态失败，已取消重启。', 'err');
    });
  }

  // 核心：还原指定 ids（工具栏按钮与目录分组按钮共用）
  function restoreArchived(ids) {
    if (!ids || !ids.length) return;
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

  // 工具栏「还原对话」：作用于勾选的会话
  function runRestore() {
    var ids = selectedIds();
    if (!ids.length) return;
    var btn = panelEl() && panelEl().querySelector('.dsh-ap-restore');
    if (btn && btn.disabled) return;
    if (btn) btn.disabled = true;
    restoreArchived(ids);
  }

  // 核心：永久删除指定 ids（带二次危险确认）；工具栏按钮与目录分组按钮共用
  function deleteArchived(ids) {
    if (!ids || !ids.length) return;
    showDangerConfirm(ids.length, function () {
      var delBtn = panelEl() && panelEl().querySelector('.dsh-ap-delete');
      if (delBtn) delBtn.disabled = true;
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

  // 工具栏「永久删除」：作用于勾选的会话
  function runDelete() {
    var ids = selectedIds();
    if (!ids.length) return;
    var btn = panelEl() && panelEl().querySelector('.dsh-ap-delete');
    if (btn && btn.disabled) return;
    deleteArchived(ids);
  }


