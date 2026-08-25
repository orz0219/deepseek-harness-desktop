  // ===== 归档执行 =====
  function onArchiveAll(btn) {
    getArchiveTargets().then(function (rows) {
      if (!rows.length) { btn.setAttribute('aria-label', '没有可归档的对话'); setTimeout(function () { setIdle(btn); }, 1500); return; }
      // 弹窗内勾选锁定的对话将从本次归档中排除；未锁定项照常归档。
      showArchiveChecklist(rows, function (selected) { executeArchive(btn, selected); });
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


