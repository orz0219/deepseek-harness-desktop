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
      var currentSessionId = getCurrentSessionId();
      if (currentSessionId) {
        var session = items.find(function (s) { return s.sessionId === currentSessionId; });
        if (session && session.cwd) {
          rsState.currentCwd = session.cwd;
          return session.cwd;
        }
      }
      return null;
    }).catch(function (e) {
      console.error('获取 session 列表失败:', e);
      return null;
    });
  }

  // 通过 DOM 追踪当前选中的会话 id。
  // DOM 追踪是唯一可行的选择：session.list 的 current 字段在桌面端不随当前查看的会话更新，不可靠；
  // 而侧边栏当前会话项（高亮 / aria-selected）由 DSH 实打实渲染，是可靠来源。
  function getCurrentSessionId() {
    // 方法1：从 DOM 中查找被选择的 session（优先）
    // dsh 的侧边栏会话项通常有 data 属性或特定的 class
    var selectors = [
      // 高亮/选中状态的 session 项
      '[data-session-id].active',
      '[data-session-id][aria-selected="true"]',
      '[data-session-id][data-active="true"]',
      '[data-session-id][data-current="true"]',
      // dsh 可能使用的 class
      '.session-item.active',
      '.session-item.selected',
      '.session-item.current',
      '.session-row.active',
      '.session-row.selected',
      // 侧边栏中被选中的项
      '[class*="session"][class*="active"]',
      '[class*="session"][class*="selected"]',
      '[class*="session"][class*="current"]',
      // 通用高亮项
      '[class*="sidebar"] [class*="active"][data-id]',
      '[class*="sidebar"] [class*="selected"][data-id]',
    ];

    for (var i = 0; i < selectors.length; i++) {
      var items = document.querySelectorAll(selectors[i]);
      for (var j = 0; j < items.length; j++) {
        var el = items[j];
        // 尝试多种属性获取 session ID
        var sid = el.getAttribute('data-session-id') ||
                  el.getAttribute('data-id') ||
                  el.getAttribute('data-session') ||
                  el.getAttribute('data-sid');
        if (sid) return sid;

        // 尝试从 href 或 onclick 中提取
        var href = el.getAttribute('href') || '';
        var match = href.match(/session[=\/]([^&\/]+)/);
        if (match) return match[1];
      }
    }

    // 方法2：从 localStorage 中获取当前 session ID
    try {
      var stored = localStorage.getItem('dsh.sessions.current');
      if (stored) {
        var parsed = JSON.parse(stored);
        if (parsed && parsed.sessionId) return parsed.sessionId;
      }
    } catch (e) {}

    // 方法3：从 URL 中提取 session ID
    var pathname = window.location.pathname;
    var parts = pathname.split('/');
    var sessionIndex = parts.indexOf('session');
    if (sessionIndex !== -1 && sessionIndex + 1 < parts.length) {
      return parts[sessionIndex + 1];
    }

    // 方法4：从 URL hash 中提取
    var hash = window.location.hash;
    var hashMatch = hash.match(/session[=\/]([^&\/]+)/);
    if (hashMatch) return hashMatch[1];

    return null;
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


