  // ===== 归档目标获取 =====
  // 返回 [{ sessionId, title, cwd }]，title 取自 session.list 的 projections.values.title，
  // 缺失时回退到 cwd 或截断的 id，保证清单弹窗始终有可读文案。
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
      // 会话 id -> { title, cwd } 索引，供清单展示
      var metaOf = {};
      sessItems.forEach(function (s) {
        var t = s.projections && s.projections.values && s.projections.values.title;
        var title = (Array.isArray(t) && t[0]) ? t[0] : (s.cwd || ('会话 ' + String(s.sessionId).slice(0, 8)));
        metaOf[s.sessionId] = { title: title, cwd: s.cwd || '' };
        if (s.running) runningIds.add(s.sessionId);
      });
      var targets = [];
      workspaces.forEach(function (ws) {
        (ws.sessionIds || []).forEach(function (id) {
          if (archivedIds.has(id) || runningIds.has(id)) return;
          var m = metaOf[id] || {};
          targets.push({
            sessionId: id,
            title: m.title || ('会话 ' + String(id).slice(0, 8)),
            cwd: m.cwd || ''
          });
        });
      });
      return targets;
    });
  }


