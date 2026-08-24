  function openPreviewModal(path) {
    // 关闭已存在的弹框
    var existing = document.getElementById('dsh-rs-preview-modal');
    if (existing) existing.remove();

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
          '<div class="dsh-rs-loading"><span class="spinner"></span>加载中</div>' +
        '</div>' +
        '<div class="dsh-rs-modal-resize-handle"></div>' +
      '</div>';

    document.body.appendChild(modal);

    // 更新路径显示
    var parts = path.split('/');
    var fileName = parts[parts.length - 1] || path;
    var dirPath = parts.slice(0, -1).join('/') || '.';
    document.getElementById('modal-file-path').innerHTML =
      '<span style="color:#9ca3af">' + escapeHtml(dirPath) + '/</span>' + escapeHtml(fileName);

    // 加载文件内容
    var fileName0 = path.split('/').pop() || path;
    var ext0 = fileName0.split('.').pop().toLowerCase();
    var isImage0 = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp'].indexOf(ext0) !== -1;

    function showLoadError(msg) {
      var el = document.getElementById('modal-file-content');
      if (!el) return;
      el.innerHTML =
        '<div class="dsh-rs-empty">' +
          '<div class="title">无法读取文件</div>' +
          '<div class="desc">' + msg + '</div>' +
        '</div>';
    }

    if (isImage0) {
      // 图片：读取原始字节并编码为 base64，交由 renderModalContent 渲染
      tauriInvoke('read_file_base64', { path: path })
        .then(function (b64) {
          renderModalContent(path, b64, true);
        })
        .catch(function (e) {
          showLoadError(e && e.message ? e.message : '读取文件失败');
        });
    } else {
      tauriInvoke('read_file_content', { path: path })
        .then(function (content) {
          renderModalContent(path, content, false);
        })
        .catch(function (e) {
          showLoadError(e && e.message ? e.message : '读取文件失败');
        });
    }

    // 支持拖拽调整大小
    initModalResize(modal);

    // ESC 关闭
    document.addEventListener('keydown', function escHandler(e) {
      if (e.key === 'Escape') {
        closePreviewModal();
        document.removeEventListener('keydown', escHandler);
      }
    });
  }

  // 渲染弹框内容
  function renderModalContent(path, content, isImageData) {
    var body = document.getElementById('modal-file-content');
    if (!body) return;

    var parts = path.split('/');
    var fileName = parts[parts.length - 1] || path;
    var ext = fileName.split('.').pop().toLowerCase();

    // 图片：直接以 base64 data URL 渲染
    if (isImageData) {
      var mime = imageMime(ext);
      body.innerHTML =
        '<div class="dsh-rs-modal-image-wrap">' +
          '<img class="dsh-rs-modal-image" src="data:' + mime + ';base64,' + content + '" alt="' + escapeHtml(fileName) + '">' +
        '</div>';
      return;
    }

    if (!content) {
      body.innerHTML = '<div class="dsh-rs-empty"><div class="desc">空文件</div></div>';
      return;
    }

    var html = '';

    // Markdown 文件添加切换按钮
    if (ext === 'md' || ext === 'markdown') {
      html += '<div class="dsh-rs-modal-toolbar">';
      html += '<button class="dsh-rs-preview-btn active" onclick="toggleModalMarkdownView(\'rendered\')">预览</button>';
      html += '<button class="dsh-rs-preview-btn" onclick="toggleModalMarkdownView(\'source\')">源码</button>';
      html += '</div>';
      html += '<div class="dsh-rs-modal-markdown" id="modal-md-preview">' + renderMarkdown(content) + '</div>';
      html += '<div class="dsh-rs-modal-code" id="modal-md-source" style="display:none">' + renderSourceCode(content, ext) + '</div>';
    } else {
      // JSON 文件：先格式化（美化缩进）再高亮
      var displayContent = (ext === 'json') ? formatJson(content) : content;
      html += '<div class="dsh-rs-modal-code">' + renderSourceCode(displayContent, ext) + '</div>';
    }

    body.innerHTML = html;
  }

  // JSON 格式化：尝试解析并重新缩进，失败则原样返回（兼容 JSONC 等）
  function formatJson(content) {
    try {
      return JSON.stringify(JSON.parse(content), null, 2);
    } catch (e) {
      return content;
    }
  }

  // 根据扩展名推断图片 MIME 类型
  function imageMime(ext) {
    var map = {
      'png': 'image/png',
      'jpg': 'image/jpeg',
      'jpeg': 'image/jpeg',
      'gif': 'image/gif',
      'svg': 'image/svg+xml',
      'webp': 'image/webp',
      'ico': 'image/x-icon',
      'bmp': 'image/bmp'
    };
    return map[ext] || 'application/octet-stream';
  }

  // 切换弹框内 Markdown 视图
  window.toggleModalMarkdownView = function (view) {
    var preview = document.getElementById('modal-md-preview');
    var source = document.getElementById('modal-md-source');
    var btns = document.querySelectorAll('.dsh-rs-modal-toolbar .dsh-rs-preview-btn');

    if (view === 'rendered') {
      if (preview) preview.style.display = 'block';
      if (source) source.style.display = 'none';
      btns[0].classList.add('active');
      btns[1].classList.remove('active');
    } else {
      if (preview) preview.style.display = 'none';
      if (source) source.style.display = 'block';
      btns[0].classList.remove('active');
      btns[1].classList.add('active');
    }
  };

  // 关闭预览弹框
  window.closePreviewModal = function () {
    var modal = document.getElementById('dsh-rs-preview-modal');
    if (modal) modal.remove();
  };

  // 调整弹框大小
  window.resizePreviewModal = function (mode) {
    var modal = document.querySelector('.dsh-rs-modal-content');
    if (!modal) return;

    if (mode === 'max') {
      modal.style.width = '90vw';
      modal.style.height = '90vh';
      modal.style.left = '5vw';
      modal.style.top = '5vh';
    } else {
      modal.style.width = '70vw';
      modal.style.height = '70vh';
      modal.style.left = '15vw';
      modal.style.top = '15vh';
    }
  };

  // 初始化弹框拖拽调整大小
  function initModalResize(modal) {
    var handle = modal.querySelector('.dsh-rs-modal-resize-handle');
    var content = modal.querySelector('.dsh-rs-modal-content');
    if (!handle || !content) return;

    var startX, startY, startW, startH;

    handle.addEventListener('mousedown', function (e) {
      e.preventDefault();
      startX = e.clientX;
      startY = e.clientY;
      startW = content.offsetWidth;
      startH = content.offsetHeight;

      function onMouseMove(e) {
        content.style.width = (startW + e.clientX - startX) + 'px';
        content.style.height = (startH + e.clientY - startY) + 'px';
      }

      function onMouseUp() {
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);
      }

      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    });
  }

  // 简单的 Markdown 渲染器

