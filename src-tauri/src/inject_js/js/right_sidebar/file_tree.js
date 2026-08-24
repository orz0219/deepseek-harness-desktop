  // ===== 文件树加载 =====
  function loadFileTree() {
    var pane = document.querySelector('.dsh-rs-pane[data-pane="files"]');
    if (!pane) return;

    pane.innerHTML = '<div class="dsh-rs-loading"><span class="spinner"></span>加载文件树…</div>';

    getCurrentCwd().then(function (cwd) {
      // 初始只预加载前几层，更深的目录在展开时懒加载（truncated 标记）
      var args = { depth: 6 };
      if (cwd) args.root = cwd;
      return tauriInvoke('get_file_tree', args);
    })
      .then(function (result) {
        rsState.fileTree = result;
        renderFileTree(result);
      })
      .catch(function (e) {
        pane.innerHTML =
          '<div class="dsh-rs-empty">' +
            ICON_FOLDER +
            '<div class="title">无法加载文件树</div>' +
            '<div class="desc">' + (e && e.message ? e.message : '读取目录失败') + '</div>' +
          '</div>';
      });
  }

  function renderFileTree(tree) {
    var pane = document.querySelector('.dsh-rs-pane[data-pane="files"]');
    if (!pane) return;

    // 文件树骨架（过滤框 + 树容器）只构建一次，之后只更新树容器，避免过滤输入时失焦
    if (!pane.querySelector('.dsh-rs-tree-filter-input')) {
      pane.innerHTML =
        '<div class="dsh-rs-tree-filter"><input class="dsh-rs-tree-filter-input" type="text" placeholder="过滤文件…" /></div>' +
        '<div class="dsh-rs-tree-wrap"></div>';
      bindFilterInput(pane);
    }
    // 重新加载（刷新 / 切换目录）时清空过滤框，恢复完整树，并失效懒加载缓存
    rsState.dirCache = {};
    var input = pane.querySelector('.dsh-rs-tree-filter-input');
    if (input) input.value = '';
    renderTreeInto(pane, tree ? tree.children : null, false);
  }

  // 仅更新树容器内容（保留过滤框及其焦点）
  function renderTreeInto(pane, items, expandAll) {
    var wrap = pane.querySelector('.dsh-rs-tree-wrap');
    if (!wrap) return;
    if (!items || !items.length) {
      wrap.innerHTML =
        '<div class="dsh-rs-empty">' +
          ICON_FOLDER +
          '<div class="title">' + (expandAll ? '无匹配文件' : '空目录') + '</div>' +
          '<div class="desc">' + (expandAll ? '没有文件名包含该关键字' : '当前目录没有文件') + '</div>' +
        '</div>';
      return;
    }
    wrap.innerHTML = '<div class="dsh-rs-tree">' + renderTreeItems(items, 0, expandAll) + '</div>';
    // 过滤模式（expandAll）下，深度截断的目录也会被展开，需批量触发懒加载
    if (expandAll) loadExpandedLazy(pane, true);
  }

  // 绑定过滤框输入事件
  function bindFilterInput(pane) {
    var input = pane.querySelector('.dsh-rs-tree-filter-input');
    if (!input) return;
    input.addEventListener('input', function () {
      var q = input.value.trim().toLowerCase();
      if (!q) {
        renderTreeInto(pane, rsState.fileTree ? rsState.fileTree.children : null, false);
      } else {
        var filtered = filterTreeData(rsState.fileTree ? rsState.fileTree.children : null, q);
        renderTreeInto(pane, filtered, true);
      }
    });
    // 过滤框内的点击 / 按键不要冒泡触发面板其它交互
    input.addEventListener('click', function (e) { e.stopPropagation(); });
    input.addEventListener('keydown', function (e) { e.stopPropagation(); });
  }

  // 按文件名递归过滤：保留名称命中的节点，以及包含命中后代的文件夹
  function filterTreeData(items, q) {
    var out = [];
    (items || []).forEach(function (item) {
      var nameMatch = (item.name || '').toLowerCase().indexOf(q) !== -1;
      var kids = item.isDir ? filterTreeData(item.children, q) : null;
      if (nameMatch || (kids && kids.length)) {
        var copy = {};
        for (var k in item) copy[k] = item[k];
        if (item.isDir) copy.children = kids || [];
        out.push(copy);
      }
    });
    return out;
  }

  function renderTreeItems(items, depth, expandAll) {
    if (!items) return '';
    depth = depth || 0;
    expandAll = expandAll || false;
    var html = '';

    // 排序：文件夹在前，文件在后，按名称排序
    var sorted = items.slice().sort(function (a, b) {
      var aDir = a.isDir === true;
      var bDir = b.isDir === true;
      if (aDir && !bDir) return -1;
      if (!aDir && bDir) return 1;
      return (a.name || '').localeCompare(b.name || '');
    });

    sorted.forEach(function (item, index) {
      // 使用路径哈希生成唯一 id
      var pathHash = 0;
      var pathStr = item.path || '';
      for (var i = 0; i < pathStr.length; i++) {
        pathHash = ((pathHash << 5) - pathHash) + pathStr.charCodeAt(i);
        pathHash = pathHash & pathHash;
      }
      var id = 'tree-' + Math.abs(pathHash).toString(36);
      var isLast = index === sorted.length - 1;
      var indent = depth * 20;
      // camelCase: isDir
      var isDir = item.isDir === true;

      html += '<div class="dsh-rs-tree-item' + (isDir ? ' file-dir' : '') + '" data-path="' + escapeHtml(item.path) + '"';

      if (isDir) {
        html += ' onclick="event.stopPropagation(); toggleTreeNode(this, \'' + id + '\')"';
      } else {
        html += ' onclick="event.stopPropagation(); previewFile(\'' + escapeHtml(item.path) + '\')"';
      }

      html += ' style="padding-left:' + (indent + 8) + 'px">';

      // 树状连接线
      if (depth > 0) {
        html += '<span class="tree-line" style="left:' + (indent - 12) + 'px"></span>';
        html += '<span class="tree-branch' + (isLast ? ' tree-branch-last' : '') + '" style="left:' + (indent - 12) + 'px"></span>';
      }

      if (isDir) {
        html += '<span class="chevron' + (expandAll ? ' expanded' : '') + '" id="chevron-' + id + '">' + ICON_CHEVRON_RIGHT + '</span>';
        html += '<span class="folder-icon' + (expandAll ? ' opened' : '') + '" id="folder-' + id + '">' + (expandAll ? ICON_FOLDER_OPENED : ICON_FOLDER_CLOSED) + '</span>';
      } else {
        html += '<span style="width:16px"></span>';
        html += getFileIcon(item.name);
      }

      html += '<span class="name">' + escapeHtml(item.name) + '</span>';
      html += '</div>';

      // 目录始终渲染 children 容器：有数据则递归渲染，
      // 深度截断（truncated）的目录标记 data-lazy，展开时再实时请求子树
      if (isDir) {
        var lazy = item.truncated === true && (!item.children || !item.children.length);
        html += '<div class="dsh-rs-tree-children' + (expandAll ? ' expanded' : '') + '" id="' + id + '"';
        if (lazy) html += ' data-lazy="1"';
        html += ' data-depth="' + (depth + 1) + '">';
        if (item.children && item.children.length) {
          html += renderTreeItems(item.children, depth + 1, expandAll);
        } else if (lazy) {
          html += '<div class="dsh-rs-tree-loading">加载中…</div>';
        }
        html += '</div>';
      }
    });

    return html;
  }

  // 根据文件名获取文件类型 class
  function getFileClass(name, isDir) {
    if (isDir) return 'file-dir';
    var ext = name.split('.').pop().toLowerCase();
    var extMap = {
      // 代码文件
      'js': 'file-js', 'jsx': 'file-js', 'ts': 'file-ts', 'tsx': 'file-ts',
      'py': 'file-py', 'rb': 'file-rb', 'go': 'file-go', 'rs': 'file-rs',
      'java': 'file-java', 'c': 'file-c', 'cpp': 'file-c', 'h': 'file-c',
      'cs': 'file-cs', 'php': 'file-php', 'swift': 'file-swift', 'kt': 'file-kt',
      // 标记语言
      'html': 'file-html', 'htm': 'file-html', 'xml': 'file-xml',
      'css': 'file-css', 'scss': 'file-css', 'less': 'file-css',
      'json': 'file-json', 'yaml': 'file-yaml', 'yml': 'file-yaml',
      'md': 'file-md', 'markdown': 'file-md',
      // 配置文件
      'toml': 'file-config', 'ini': 'file-config', 'env': 'file-config',
      'gitignore': 'file-config', 'dockerignore': 'file-config',
      'dockerfile': 'file-config', 'makefile': 'file-config',
      // 图片
      'png': 'file-image', 'jpg': 'file-image', 'jpeg': 'file-image',
      'gif': 'file-image', 'svg': 'file-image', 'webp': 'file-image', 'ico': 'file-image',
      // 文档
      'txt': 'file-text', 'log': 'file-text', 'csv': 'file-text',
      'pdf': 'file-pdf', 'doc': 'file-doc', 'docx': 'file-doc',
      // 压缩包
      'zip': 'file-archive', 'tar': 'file-archive', 'gz': 'file-archive',
      'rar': 'file-archive', '7z': 'file-archive',
      // 其他
      'sh': 'file-shell', 'bash': 'file-shell', 'zsh': 'file-shell',
      'sql': 'file-sql', 'graphql': 'file-graphql',
    };
    return extMap[ext] || 'file-default';
  }

  // 根据文件名获取文件图标
  function getFileIcon(name) {
    var ext = name.split('.').pop().toLowerCase();
    var iconMap = {
      'js': ICON_FILE_JS, 'jsx': ICON_FILE_JS,
      'ts': ICON_FILE_TS, 'tsx': ICON_FILE_TS,
      'py': ICON_FILE_PY,
      'html': ICON_FILE_HTML, 'htm': ICON_FILE_HTML,
      'css': ICON_FILE_CSS, 'scss': ICON_FILE_CSS, 'less': ICON_FILE_CSS,
      'json': ICON_FILE_JSON,
      'md': ICON_FILE_MD,
      'png': ICON_FILE_IMAGE, 'jpg': ICON_FILE_IMAGE, 'jpeg': ICON_FILE_IMAGE,
      'gif': ICON_FILE_IMAGE, 'svg': ICON_FILE_IMAGE, 'webp': ICON_FILE_IMAGE,
      'sh': ICON_FILE_SHELL, 'bash': ICON_FILE_SHELL, 'zsh': ICON_FILE_SHELL,
    };
    return iconMap[ext] || ICON_FILE;
  }

  // 全局函数：切换树节点展开/折叠
  window.toggleTreeNode = function (el, id) {
    var children = document.getElementById(id);
    var chevron = document.getElementById('chevron-' + id);
    var folderIcon = document.getElementById('folder-' + id);

    if (children) {
      var isExpanding = !children.classList.contains('expanded');
      children.classList.toggle('expanded');

      // 更新 chevron 旋转
      if (chevron) {
        chevron.classList.toggle('expanded');
      }

      // 更新文件夹图标颜色
      if (folderIcon) {
        if (isExpanding) {
          folderIcon.innerHTML = ICON_FOLDER_OPENED;
          folderIcon.classList.add('opened');
        } else {
          folderIcon.innerHTML = ICON_FOLDER_CLOSED;
          folderIcon.classList.remove('opened');
        }
      }

      // 懒加载：首次展开深度截断的目录时实时请求子树
      if (isExpanding && children.getAttribute('data-lazy') === '1') {
        loadLazyChildren(el, children, false);
      }
    }
  };

  // 懒加载目录子树：带缓存（过滤重渲染时避免重复请求）
  function loadLazyChildren(el, container, expandAll) {
    var path = el.getAttribute('data-path');
    if (!path || container.getAttribute('data-loading') === '1') return;

    // 命中缓存直接渲染
    if (rsState.dirCache && Object.prototype.hasOwnProperty.call(rsState.dirCache, path)) {
      applyLazyChildren(container, rsState.dirCache[path], expandAll);
      return;
    }

    container.setAttribute('data-loading', '1');
    var depth = parseInt(container.getAttribute('data-depth'), 10) || 0;
    tauriInvoke('get_file_tree', { root: path, depth: 2 })
      .then(function (node) {
        if (!rsState.dirCache) rsState.dirCache = {};
        var items = node && node.children ? node.children : [];
        rsState.dirCache[path] = items;
        applyLazyChildren(container, items, expandAll);
      })
      .catch(function (err) {
        container.removeAttribute('data-loading');
        console.error('[dsh] load file tree children failed', err);
        container.innerHTML = '<div class="dsh-rs-tree-empty">加载失败</div>';
        // 保留 data-lazy，下次点击可重试
      });
  }

  function applyLazyChildren(container, items, expandAll) {
    container.removeAttribute('data-lazy');
    container.removeAttribute('data-loading');
    var depth = parseInt(container.getAttribute('data-depth'), 10) || 0;
    container.innerHTML = items.length ? renderTreeItems(items, depth, !!expandAll) : '<div class="dsh-rs-tree-empty">空文件夹</div>';
  }

  // 批量加载已展开但未加载的目录（过滤模式 expandAll 渲染后调用）
  function loadExpandedLazy(pane, expandAll) {
    var nodes = pane.querySelectorAll('.dsh-rs-tree-children.expanded[data-lazy]');
    Array.prototype.forEach.call(nodes, function (container) {
      var itemEl = container.previousElementSibling; // 对应的 .dsh-rs-tree-item
      if (itemEl && itemEl.classList.contains('dsh-rs-tree-item')) {
        loadLazyChildren(itemEl, container, expandAll);
      }
    });
  }

  // 全局函数：预览文件（弹框模式）
  window.previewFile = function (path) {
    openPreviewModal(path);
  };

  // 打开预览弹框

  // ===== 文件树右键上下文菜单 =====
  var __ctxCloseFns = null;

  function closeTreeContextMenu() {
    var m = document.querySelector('.dsh-rs-ctxmenu');
    if (m) m.remove();
    if (__ctxCloseFns) {
      document.removeEventListener('mousedown', __ctxCloseFns.down);
      document.removeEventListener('keydown', __ctxCloseFns.key);
      window.removeEventListener('resize', __ctxCloseFns.down);
      document.removeEventListener('scroll', __ctxCloseFns.down, true);
      __ctxCloseFns = null;
    }
  }

  function openTreeContextMenu(e, path, isDir) {
    closeTreeContextMenu();
    var menu = document.createElement('div');
    menu.className = 'dsh-rs-ctxmenu';
    menu.setAttribute('role', 'menu');

    var copyItem = document.createElement('div');
    copyItem.className = 'dsh-rs-ctxmenu-item';
    copyItem.setAttribute('role', 'menuitem');
    copyItem.textContent = '复制绝对路径';
    wireCtxItem(copyItem, function () { copyAbsPath(path); });

    var openItem = document.createElement('div');
    openItem.className = 'dsh-rs-ctxmenu-item';
    openItem.setAttribute('role', 'menuitem');
    openItem.textContent = isDir ? '在访达中打开' : '在访达中显示';
    wireCtxItem(openItem, function () {
      console.log('[dsh] reveal_in_finder', path, isDir);
      tauriInvoke('reveal_in_finder', { path: path, isDir: isDir })
        .then(function () { showRsToast('已在访达中打开'); })
        .catch(function (err) {
          console.error('[dsh] reveal_in_finder failed', err);
          showRsToast('打开失败：' + describeErr(err));
        });
    });

    menu.appendChild(copyItem);
    menu.appendChild(openItem);
    document.body.appendChild(menu);

    // 定位并限制在视口内
    var mw = menu.offsetWidth, mh = menu.offsetHeight;
    var x = Math.min(e.clientX, window.innerWidth - mw - 8);
    var y = Math.min(e.clientY, window.innerHeight - mh - 8);
    menu.style.left = Math.max(8, x) + 'px';
    menu.style.top = Math.max(8, y) + 'px';

    // 点击别处 / 滚动 / Esc 关闭（延迟注册，避免本次右键的 mousedown 立即关闭）
    var down = function (ev) {
      if (ev.target && ev.target.closest && ev.target.closest('.dsh-rs-ctxmenu')) return;
      closeTreeContextMenu();
    };
    var key = function (ev) { if (ev.key === 'Escape') closeTreeContextMenu(); };
    __ctxCloseFns = { down: down, key: key };
    setTimeout(function () {
      document.addEventListener('mousedown', down);
      document.addEventListener('keydown', key);
      window.addEventListener('resize', down);
      document.addEventListener('scroll', down, true);
    }, 0);
  }

  // 复制绝对路径：优先用浏览器剪贴板 API（macOS 上比从桌面主进程 spawn pbcopy 更可靠）
  function copyAbsPath(path) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(path).then(function () {
        showRsToast('已复制路径');
      }).catch(function () {
        fallbackCopy(path); // 剪贴板 API 不可用时退回 Rust 命令
      });
      return;
    }
    fallbackCopy(path);
  }
  function fallbackCopy(path) {
    tauriInvoke('copy_to_clipboard', { text: path })
      .then(function () { showRsToast('已复制路径'); })
      .catch(function (err) { showRsToast('复制失败：' + describeErr(err)); });
  }
  function describeErr(err) {
    if (!err) return '未知错误';
    if (typeof err === 'string') return err;
    if (err.message) return err.message;
    try { return JSON.stringify(err); } catch (e) { return String(err); }
  }

  // 菜单项同时响应左键与右键（右键习惯用户直接右键菜单项也能触发）
  function wireCtxItem(item, action) {
    item.addEventListener('click', function (ev) { ev.preventDefault(); closeTreeContextMenu(); action(); });
    item.addEventListener('contextmenu', function (ev) {
      ev.preventDefault();
      ev.stopPropagation();
      closeTreeContextMenu();
      action();
    });
  }

  var __rsToastTimer = null;
  function showRsToast(msg) {
    var t = document.getElementById('dsh-rs-hint');
    if (!t) return;
    var isErr = /失败|错误|无效|not allowed|not found/i.test(msg);
    t.textContent = msg;
    t.classList.toggle('err', isErr);
    t.classList.add('show');
    if (__rsToastTimer) clearTimeout(__rsToastTimer);
    __rsToastTimer = setTimeout(function () { t.classList.remove('show'); }, 2600);
  }

