  // ===== Git 数据加载 =====
  function loadGitData() {
    var pane = document.querySelector('.dsh-rs-pane[data-pane="git"]');
    if (!pane) return;

    pane.innerHTML = '<div class="dsh-rs-loading"><span class="spinner"></span>加载中</div>';

    getCurrentCwd().then(function (cwd) {
      var args = cwd ? { cwd: cwd } : {};
      return tauriInvoke('get_git_diff', args);
    })
      .then(function (result) {
        rsState.gitData = result;
        renderGitDiff(result);
      })
      .catch(function (e) {
        pane.innerHTML =
          '<div class="dsh-rs-empty">' +
            ICON_GIT +
            '<div class="title">无法加载 Git 改动</div>' +
            '<div class="desc">' + (e && e.message ? e.message : '请确保当前目录是 Git 仓库') + '</div>' +
          '</div>';
      });
  }

  function renderGitDiff(data) {
    var pane = document.querySelector('.dsh-rs-pane[data-pane="git"]');
    if (!pane) return;

    if (!data || !data.files || !data.files.length) {
      pane.innerHTML =
        '<div class="dsh-rs-empty">' +
          ICON_GIT +
          '<div class="title">没有改动</div>' +
          '<div class="desc">工作区干净，没有未提交的更改</div>' +
        '</div>';
      return;
    }

    // 缓存文件数据，供点击后在弹框中显示详情
    window.__gitFiles = data.files;

    var html = '<div class="dsh-rs-git-list">';
    data.files.forEach(function (file, index) {
      var statusClass = { M: 'modified', A: 'added', D: 'deleted', R: 'renamed' }[file.status] || 'modified';
      var statusText = { M: '修改', A: '新增', D: '删除', R: '重命名' }[file.status] || file.status;

      html += '<div class="dsh-rs-git-item" onclick="openGitDiffModal(window.__gitFiles[' + index + '])">';
      html += '<span class="dsh-rs-git-status ' + statusClass + '">' + statusText + '</span>';
      html += '<span class="dsh-rs-git-path">' + escapeHtml(file.path) + '</span>';
      html += '<span class="dsh-rs-git-stats"><span class="add">+' + file.additions + '</span> <span class="del">-' + file.deletions + '</span></span>';
      html += '</div>';
    });
    html += '</div>';

    pane.innerHTML = html;
  }

  function renderDiffLines(diff) {
    if (!diff) return '';
    var lines = diff.split('\n');
    var html = '';
    var lineNum = 0;

    lines.forEach(function (line) {
      lineNum++;
      var cls = 'context';
      if (line.startsWith('+') && !line.startsWith('+++')) {
        cls = 'addition';
      } else if (line.startsWith('-') && !line.startsWith('---')) {
        cls = 'deletion';
      }
      html += '<div class="dsh-rs-diff-line ' + cls + '">';
      html += '<span class="line-num">' + lineNum + '</span>';
      html += escapeHtml(line);
      html += '</div>';
    });

    return html;
  }

  // 在预览弹框中显示单个文件的 Git diff 详情
  function openGitDiffModal(file) {
    if (!file) return;
    var existing = document.getElementById('dsh-rs-preview-modal');
    if (existing) existing.remove();

    var statusClass = { M: 'modified', A: 'added', D: 'deleted', R: 'renamed' }[file.status] || 'modified';
    var statusText = { M: '修改', A: '新增', D: '删除', R: '重命名' }[file.status] || file.status;

    var modal = document.createElement('div');
    modal.id = 'dsh-rs-preview-modal';
    modal.className = 'dsh-rs-modal';
    modal.innerHTML =
      '<div class="dsh-rs-modal-content">' +
        '<div class="dsh-rs-modal-header">' +
          '<span class="dsh-rs-modal-path" id="modal-file-path"></span>' +
          '<div class="dsh-rs-modal-actions">' +
            '<button class="dsh-rs-modal-btn" onclick="resizePreviewModal(\'min\')" title="最小化">─</button>' +
            '<button class="dsh-rs-modal-btn" onclick="resizePreviewModal(\'max\')" title="最大化">□</button>' +
            '<button class="dsh-rs-modal-btn dsh-rs-modal-close" onclick="closePreviewModal()" title="关闭">✕</button>' +
          '</div>' +
        '</div>' +
        '<div class="dsh-rs-modal-body" id="modal-file-content">' +
          '<div class="dsh-rs-git-diff-meta">' +
            '<span class="dsh-rs-git-status ' + statusClass + '">' + statusText + '</span>' +
            '<span class="dsh-rs-git-diff-stats"><span class="add">+' + file.additions + '</span> <span class="del">-' + file.deletions + '</span></span>' +
          '</div>' +
          '<div class="dsh-rs-git-diff-content">' + renderDiffLines(file.diff) + '</div>' +
        '</div>' +
        '<div class="dsh-rs-modal-resize-handle"></div>' +
      '</div>';

    document.body.appendChild(modal);

    // 路径展示（目录灰 + 文件名）
    var parts = (file.path || '').split('/');
    var fileName = parts[parts.length - 1] || file.path || '';
    var dirPath = parts.slice(0, -1).join('/') || '.';
    document.getElementById('modal-file-path').innerHTML =
      '<span style="color:#9ca3af">' + escapeHtml(dirPath) + '/</span>' + escapeHtml(fileName);

    initModalResize(modal);

    document.addEventListener('keydown', function escHandler(e) {
      if (e.key === 'Escape') {
        closePreviewModal();
        document.removeEventListener('keydown', escHandler);
      }
    });
  }

  // 暴露到全局，供内联 onclick 调用（git 列表项点击打开 diff 弹框）
  window.openGitDiffModal = openGitDiffModal;


