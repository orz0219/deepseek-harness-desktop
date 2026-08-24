  // ===== 归档目标获取 =====
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


