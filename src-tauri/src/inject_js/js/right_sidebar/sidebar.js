  // ===== 标签页切换 =====
  function switchTab(tabName) {
    rsState.activeTab = tabName;

    // 更新标签页样式
    document.querySelectorAll('.dsh-rs-tab').forEach(function (tab) {
      tab.classList.toggle('active', tab.getAttribute('data-tab') === tabName);
    });

    // 更新内容区
    document.querySelectorAll('.dsh-rs-pane').forEach(function (pane) {
      pane.classList.toggle('active', pane.getAttribute('data-pane') === tabName);
    });

    // 按需加载数据
    // 切到 Git 标签时总是重新加载：避免使用过期缓存。否则应用启动早期
    // cwd 解析尚未稳定（取到其它会话目录）时加载的数据会被一直缓存，
    // 导致之后切回 Git 标签仍显示旧目录的改动（例如显示成 novel 项目的 diff）。
    if (tabName === 'git') {
      rsState.gitData = null;
      rsState.currentCwd = null; // 强制重新解析当前会话目录，避免沿用早期错误缓存
      loadGitData();
    } else if (tabName === 'files' && !rsState.fileTree) {
      loadFileTree();
    }

    saveRsState();
  }

  // ===== 工具函数 =====
  function escapeHtml(str) {
    if (!str) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  // ===== 面板切换 =====
  // 找到 dsh web 的挂载根节点（#root，由 index.html 创建；React 只向其渲染、不管理其 className/style，故挂 class 不会被覆写）
  function getHostRoot() {
    return document.getElementById('root');
  }

  // 面板展开时给 #root 右侧让出 360px（块级 width:auto + margin-right 必收缩宽度，与 box-sizing 无关），
  // 使 dsh web 对话区（AppFrame 的 1fr 中间列）右移、不被固定面板遮挡
  function applyHostPush(expanded) {
    var host = getHostRoot();
    if (!host) return;
    // 必须用内联样式：dsh web 自身有 #root{margin:0}（ID 选择器优先级高于类选择器），
    // 仅用类设置 margin-right 会被覆盖；内联样式优先级最高，可稳定生效。
    // #root 由 index.html 创建、React 不管理其 style/class，故内联改动不会被 React 覆写。
    host.style.transition = 'margin-right .25s ease';
    host.style.marginRight = expanded ? '360px' : '';
  }

  function toggleRightSidebar() {
    var sidebar = document.getElementById(RS_ID);
    if (!sidebar) return;

    rsState.expanded = !rsState.expanded;
    sidebar.classList.toggle('expanded', rsState.expanded);
    applyHostPush(rsState.expanded);

    var toggle = document.getElementById(RS_TOGGLE_ID);
    if (toggle) {
      toggle.innerHTML = rsState.expanded ? ICON_CHEVRON_RIGHT : ICON_CHEVRON_LEFT;
      toggle.setAttribute('aria-label', rsState.expanded ? '收起右侧面板' : '展开右侧面板');
    }

    saveRsState();
  }

  // ===== 右侧侧边栏初始化 =====
  function initRightSidebar() {
    ensureStyle('dsh-right-sidebar-style', RS_CSS);
    loadRsState();

    // 检查是否已存在
    if (document.getElementById(RS_ID)) return;

    // 创建容器
    var sidebar = document.createElement('div');
    sidebar.id = RS_ID;
    if (rsState.expanded) sidebar.classList.add('expanded');
    applyHostPush(rsState.expanded);

    // 创建切换按钮
    var toggle = document.createElement('button');
    toggle.id = RS_TOGGLE_ID;
    toggle.type = 'button';
    toggle.innerHTML = rsState.expanded ? ICON_CHEVRON_RIGHT : ICON_CHEVRON_LEFT;
    toggle.setAttribute('aria-label', rsState.expanded ? '收起右侧面板' : '展开右侧面板');
    toggle.addEventListener('click', toggleRightSidebar);

    // 创建面板
    var panel = document.createElement('div');
    panel.id = RS_PANEL_ID;

    // 标签页导航
    var tabs = document.createElement('div');
    tabs.className = 'dsh-rs-tabs';
    tabs.innerHTML =
      '<button class="dsh-rs-tab' + (rsState.activeTab === 'git' ? ' active' : '') + '" data-tab="git">' + ICON_GIT + ' Git 改动</button>' +
      '<button class="dsh-rs-tab' + (rsState.activeTab === 'files' ? ' active' : '') + '" data-tab="files">' + ICON_FOLDER + ' 文件树</button>';

    // 绑定标签页点击事件
    tabs.querySelectorAll('.dsh-rs-tab').forEach(function (tab) {
      tab.addEventListener('click', function () {
        switchTab(tab.getAttribute('data-tab'));
      });
    });

    // 内容区
    var content = document.createElement('div');
    content.className = 'dsh-rs-content';
    content.innerHTML =
      '<div class="dsh-rs-pane' + (rsState.activeTab === 'git' ? ' active' : '') + '" data-pane="git"></div>' +
      '<div class="dsh-rs-pane' + (rsState.activeTab === 'files' ? ' active' : '') + '" data-pane="files"></div>';

    // 工具栏
    var toolbar = document.createElement('div');
    toolbar.className = 'dsh-rs-toolbar';
    toolbar.innerHTML =
      '<span class="dsh-rs-toolbar-title" id="dsh-rs-cwd" title="当前工作目录">加载工作目录…</span>' +
      '<button class="dsh-rs-toolbar-btn" onclick="copyCwd()" aria-label="复制路径" title="复制当前工作目录路径">' + ICON_COPY + '</button>' +
      '<button class="dsh-rs-toolbar-btn" onclick="revealCwd()" aria-label="在访达中显示" title="在访达中显示当前目录">' + ICON_FOLDER + '</button>' +
      '<button class="dsh-rs-toolbar-btn" onclick="refreshCurrentPane()" aria-label="刷新">' + ICON_REFRESH + '</button>';

    // 组装面板
    panel.appendChild(tabs);
    panel.appendChild(toolbar);
    // 首屏不在最早同步获取：那时 RPC 尚未就绪会让请求永久 pending、缓存永远填不上，
    // 工具栏卡在「加载工作目录…」。延迟到 dsh 接管且后端就绪后再填充一次缓存，
    // 之后各模块直接读 rsState.currentCwd 缓存即可。
    setTimeout(updateCwdTitle, 1200);
    panel.appendChild(content);
    // 面板底部提示条：复制/在访达打开等操作的反馈，始终可见且不受 dsh web 样式影响
    var hint = document.createElement('div');
    hint.id = 'dsh-rs-hint';
    hint.className = 'dsh-rs-hint';
    panel.appendChild(hint);

    // 组装容器
    sidebar.appendChild(toggle);
    sidebar.appendChild(panel);
    document.body.appendChild(sidebar);

    // 初始加载数据
    if (rsState.activeTab === 'git') {
      loadGitData();
    } else if (rsState.activeTab === 'files') {
      loadFileTree();
    }

    // 监听 session 切换
    watchSessionChange();

    // MutationObserver 自愈
    if (!window.__dshRightSidebarObserver && document.body) {
      var scheduled = false;
      window.__dshRightSidebarObserver = new MutationObserver(function () {
        if (scheduled) return;
        scheduled = true;
        setTimeout(function () {
          scheduled = false;
          if (!document.getElementById(RS_ID)) {
            initRightSidebar();
          }
          // 宿主内容区可能此时才挂载/重挂载，确保避让状态与展开态一致
          applyHostPush(rsState.expanded);
        }, 300);
      });
      window.__dshRightSidebarObserver.observe(document.body, { childList: true, subtree: true });
    }

    // 全局右键处理：
    // - 文件树节点 → 自定义菜单（复制绝对路径 / 在访达中打开）
    // - 可编辑元素或已选中文字 → 保留原生菜单（复制/粘贴可用）
    // - 其它区域（背景/链接等）→ 屏蔽原生菜单，避免单页程序出现「返回上一页」等导航项
    if (!window.__dshCtxMenuGlobal) {
      window.__dshCtxMenuGlobal = true;
      document.addEventListener('contextmenu', function (e) {
        var item = e.target && e.target.closest ? e.target.closest('.dsh-rs-tree-item') : null;
        if (item) {
          e.preventDefault();
          closeTreeContextMenu();
          var path = item.getAttribute('data-path');
          if (path) {
            var isDir = item.classList.contains('file-dir');
            openTreeContextMenu(e, path, isDir);
          }
          return;
        }
        var sel = (window.getSelection && window.getSelection().toString && window.getSelection().toString()) || '';
        var editable = e.target && e.target.closest
          ? e.target.closest('input,textarea,[contenteditable="true"]')
          : null;
        if (editable || sel.trim()) return; // 保留原生菜单（复制 / 粘贴）
        e.preventDefault(); // 背景/链接等：不弹原生导航菜单
      });
    }
  }

  // 全局函数：刷新当前面板
  window.refreshCurrentPane = function () {
    rsState.gitData = null;
    rsState.fileTree = null;
    rsState.currentCwd = null;  // 清除 cwd 缓存
    switchTab(rsState.activeTab);
    updateCwdTitle();  // 刷新后重新解析并更新工作目录标题
  };

   // 更新工具栏标题为当前工作目录绝对路径
   function updateCwdTitle() {
     var el = document.getElementById('dsh-rs-cwd');
     if (!el) return;
     if (rsState.currentCwd) {            // 缓存命中：模块直接读缓存，不每次重新获取
       el.textContent = rsState.currentCwd;
       return;
     }
     getCurrentCwd().then(function (cwd) {
       el.textContent = cwd ? cwd : '无法获取工作目录';
     });
   }

   // 全局函数：复制当前工作目录绝对路径到剪贴板
   window.copyCwd = function () {
     getCurrentCwd().then(function (cwd) {
       if (!cwd) { showRsToast('无法获取当前工作目录'); return; }
       tauriInvoke('copy_to_clipboard', { text: cwd })
         .then(function () { showRsToast('已复制：' + cwd); })
         .catch(function (e) { showRsToast('复制失败：' + (e && e.message ? e.message : e)); });
     });
   };

   // 全局函数：在访达（Finder）中显示当前工作目录
   window.revealCwd = function () {
     getCurrentCwd().then(function (cwd) {
       if (!cwd) { showRsToast('无法获取当前工作目录'); return; }
       tauriInvoke('reveal_in_finder', { path: cwd, is_dir: true })
         .then(function () { showRsToast('已在访达中打开'); })
         .catch(function (e) { showRsToast('打开访达失败：' + (e && e.message ? e.message : e)); });
     });
   };

  // 快捷键支持：Cmd+Shift+B 切换面板
  document.addEventListener('keydown', function (e) {
    if (e.metaKey && e.shiftKey && e.key === 'B') {
      e.preventDefault();
      toggleRightSidebar();
    }
  });

