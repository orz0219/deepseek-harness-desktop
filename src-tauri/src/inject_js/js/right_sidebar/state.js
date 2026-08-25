  // ===== 状态管理 =====
  var rsState = {
    expanded: true,
    activeTab: 'git',
    gitData: null,
    fileTree: null,
    dirCache: {},   // 文件树懒加载缓存：path -> children 数组
    currentCwd: null  // 当前对话的工作目录
  };

  // 加载保存的状态
  function loadRsState() {
    try {
      var saved = JSON.parse(localStorage.getItem(RS_STORAGE_KEY) || '{}');
      if (saved.expanded !== undefined) rsState.expanded = saved.expanded;
      if (saved.activeTab) {
        // 预览 tab 已废弃，历史保存的 'preview' 状态回退到 git
        rsState.activeTab = (saved.activeTab === 'preview') ? 'git' : saved.activeTab;
      }
    } catch (e) {}
  }

  function saveRsState() {
    try {
      localStorage.setItem(RS_STORAGE_KEY, JSON.stringify({
        expanded: rsState.expanded,
        activeTab: rsState.activeTab
      }));
    } catch (e) {}
  }

  // ===== 获取「当前正在查看的对话」的工作目录（绝对路径）=====
  // 通过 getCurrentSessionId()（DOM 追踪）拿到当前选中的会话 id，再到 session.list 取它的 cwd。
  // 说明：DOM 追踪是唯一可行的选择 —— session.list 的 current 字段在桌面端不随当前查看的会话更新，不可靠。
  // 拿不到当前会话 cwd 时返回 null，由调用方决定：弹窗展示场景应静默，不兜底调用系统打开。
  function getCurrentCwd() {
    if (rsState.currentCwd) return Promise.resolve(rsState.currentCwd);

    return rpc('session.list', {}).then(function (res) {
      var items = (res && res.result && res.result.value && res.result.value.items) || [];
      // 先解析当前会话 id（localStorage 优先），再用它从会话列表取出 cwd。
      var sid = getCurrentSessionId(items);
      var session = sid ? items.find(function (s) { return s.sessionId === sid; }) : null;
      // 兜底：localStorage 为空/失效时，用侧边栏「选中行」的标题反查会话，
      // 确保总能拿到当前会话的 cwd，而不是回落到进程 cwd（打包后进程 cwd 是
      // app 自身目录，会显示错误且固定的目录、不跟随会话切换）。
      if (!session) session = matchSelectedSession(items);
      if (session && session.cwd) {
        rsState.currentCwd = session.cwd;
        return session.cwd;
      }
      return null;
    }).catch(function (e) {
      console.error('获取 session 列表失败:', e);
      return null;
    });
  }

  // 解析「当前正在查看的会话」id。
  // session.list 的 current 字段在桌面端不随当前查看的会话更新，不可靠；
  // 因此按以下优先级解析（items 为 session.list 结果，可选，用于校验 id 仍有效）：
  //   1) harness 持久化在 localStorage 的当前会话 id（最可靠，真实 GUI 实打实写入）
  //   2) DOM 上的 data 属性（部分 fork/旧版 harness 可能带）
  //   3) URL 路径 / hash 中的 session id
  function getCurrentSessionId(items) {
    // 方法1（最可靠）：harness 把当前选中的会话 id 持久化在 localStorage。
    try {
      var stored = localStorage.getItem('dsh.sessions.current');
      if (stored) {
        var parsed = JSON.parse(stored);
        if (parsed && parsed.sessionId) {
          // 校验该会话仍在列表中（避免陈旧 id 指向已删除会话）
          if (!items || items.some(function (s) { return s.sessionId === parsed.sessionId; })) {
            return parsed.sessionId;
          }
        }
      }
    } catch (e) {}

    // 方法2：DOM data 属性（部分 fork / 旧版 harness 可能带）。
    var dataSelectors = [
      '[data-session-id].active',
      '[data-session-id][aria-selected="true"]',
      '[data-session-id][data-active="true"]',
      '[data-session-id][data-current="true"]',
      '.session-item.active',
      '.session-item.selected',
      '.session-item.current',
      '.session-row.active',
      '.session-row.selected',
      '[class*="session"][class*="active"]',
      '[class*="session"][class*="selected"]',
      '[class*="session"][class*="current"]',
      '[class*="sidebar"] [class*="active"][data-id]',
      '[class*="sidebar"] [class*="selected"][data-id]',
    ];
    for (var i = 0; i < dataSelectors.length; i++) {
      var nodes = document.querySelectorAll(dataSelectors[i]);
      for (var j = 0; j < nodes.length; j++) {
        var el = nodes[j];
        var sid = el.getAttribute('data-session-id') ||
                  el.getAttribute('data-id') ||
                  el.getAttribute('data-session') ||
                  el.getAttribute('data-sid');
        if (sid) return sid;
        // 尝试从 href 或 onclick 中提取
        var href = el.getAttribute('href') || '';
        var m = href.match(/session[=\/]([^&\/]+)/);
        if (m) return m[1];
      }
    }

    // 方法3：从 URL 路径 / hash 中提取 session ID
    var parts = (window.location.pathname || '').split('/');
    var sessionIndex = parts.indexOf('session');
    if (sessionIndex !== -1 && sessionIndex + 1 < parts.length) {
      return parts[sessionIndex + 1];
    }
    var hashMatch = (window.location.hash || '').match(/session[=\/]([^&\/]+)/);
    if (hashMatch) return hashMatch[1];

    return null;
  }

  // 兜底：找到侧边栏中带 selected 类的会话行，用其标题在 session.list 中反查会话。
  // 真实 harness 的会话行使用 hash 类名（如 YDXeBa_sessionRow + YDXeBa_selected），
  // 且没有暴露 session id 的 data 属性，所以无法直接取 id；但选中行的标题与
  // session.list 条目的 projections.values.title 一致，可用包含匹配反查。
  function matchSelectedSession(items) {
    if (!items || !items.length) return null;
    var row = document.querySelector('[role="treeitem"][class*="selected"]') ||
              document.querySelector('[class*="sessionRow"][class*="selected"]');
    if (!row) return null;
    var titleEl = row.querySelector('[class*="title"]');
    var text = (titleEl ? titleEl.textContent : row.textContent) || '';
    text = text.split('\n')[0].trim();
    if (!text) return null;
    // 标题可能含截断/附加文案，用包含匹配并取最长命中的，降低碰撞误判。
    var best = null, bestLen = 0;
    for (var k = 0; k < items.length; k++) {
      var t = (items[k].projections && items[k].projections.values && items[k].projections.values.title) || '';
      if (t && text.indexOf(t) !== -1 && t.length > bestLen) {
        best = items[k];
        bestLen = t.length;
      }
    }
    return best;
  }

  // 监听 session 切换：通过 DOM 追踪当前会话（见 getCurrentSessionId），
  // 变化时清除 cwd 缓存并刷新右侧抽屉。
  function watchSessionChange() {
    var lastSessionId = getCurrentSessionId();

    // 定期检查 session 是否变化
    setInterval(function () {
      var currentSessionId = getCurrentSessionId();
      if (currentSessionId !== lastSessionId) {
        lastSessionId = currentSessionId;
        rsState.currentCwd = null;
        if (rsState.expanded) {
          refreshCurrentPane();
        }
      }
    }, 500);

    // 监听 DOM 变化（侧边栏高亮变化）
    if (document.body) {
      var observer = new MutationObserver(function () {
        var newSessionId = getCurrentSessionId();
        if (newSessionId !== lastSessionId) {
          lastSessionId = newSessionId;
          rsState.currentCwd = null;
          if (rsState.expanded) {
            refreshCurrentPane();
          }
        }
      });

      // 观察侧边栏区域的变化
      var sidebar = document.querySelector('[class*="sidebar"], [class*="Sidebar"]');
      if (sidebar) {
        observer.observe(sidebar, { childList: true, subtree: true, attributes: true });
      }
    }
  }


