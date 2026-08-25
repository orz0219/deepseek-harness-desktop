// ===== 文件链接拦截：用自有弹窗展示，而非 DSH 默认打开 =====
// DSH 工具调用结果中的文件路径以 <button class*="fileLink"> 渲染（见 dsh ui-tool 的 ToolRow.tsx，
// 使用 CSS module 的 css.fileLink；编译后类名含 "fileLink" 子串，hash 前缀每次构建会变）。
// 点击该按钮会触发 DSH 默认的 onOpenFile（在 React onClick 的 bubble 阶段 stopPropagation 后调用）。
// 这里在 document 捕获阶段拦截，阻止 DSH 默认行为，改用已注入的 window.previewFile 弹窗展示文件内容。

// 当前激活预览事务：把对话里同一文件的多个 DOM 按钮归一为「一个预览意图」。
// 仅约束当前一次预览会话（打开期间忽略同名重复点击），关闭即清空；不记录历史、不永久屏蔽。
let activePreviewPath = null;
function requestPreview(path) {
  if (activePreviewPath !== path) {
    activePreviewPath = path;
    window.previewFile(path);
  }
}
window.resetPreviewDedup = function () { activePreviewPath = null; };

function initFileLinkInterceptor() {
  document.addEventListener('click', function (e) {
    // 方案 C：预览弹框打开期间，落在弹框内/遮罩上的点击不拦截，
    // 避免穿透被误当成文件链接、反复打开/重建弹框（也缓解「关闭后同名文件反复顶开」）
    var modal = document.getElementById('dsh-rs-preview-modal');
    if (modal && modal.contains(e.target)) return;

    // 1) 「产物」按钮：DSH 对话末尾产物说明里的文件按钮。
    //    形如 <button class="P4kPIW_file" title="/abs/path/file.js">file.js</button>，
    //    title 已是绝对路径（mac 以 "/" 开头）。不依赖易变 CSS hash 类名，直接用 title。
    //    【方案 A 收口】不再用 button[title^="/"] 泛匹配（会误拦对话进行中大量非文件按钮），
    //    仅当 title 是「绝对路径下的文件」（以 "/" 开头、无空白、含扩展名）时才拦截。
    var prodBtn = e.target.closest && e.target.closest('button');
    if (prodBtn) {
      var prodTitle = (prodBtn.getAttribute('title') || '').trim();
      var isFilePath = prodTitle.charAt(0) === '/' &&
                       !/\s/.test(prodTitle) &&
                       /\.[A-Za-z0-9]+$/.test(prodTitle);
      if (isFilePath) {
        // 捕获阶段先于 DSH 默认预览行为；阻断冒泡，改用自有弹窗
        e.stopPropagation();
        e.preventDefault();
        requestPreview(prodTitle);
        return;
      }
    }

    // 2) 原有 fileLink 按钮：DSH 工具结果里相对 cwd 的路径按钮
    // 按钮内可能有子节点，用 closest 向上找到 fileLink 按钮
    var link = e.target.closest && e.target.closest('[class*="fileLink"]');
    if (!link) return;

    // 捕获阶段先于 React 的 onClick（bubble）执行；阻断冒泡，DSH 的 onOpenFile 不会运行
    e.stopPropagation();
    e.preventDefault();

    // 按钮文本是相对当前会话 cwd 的路径（如 crates/.../agent.rs）
    var relPath = (link.textContent || '').trim();
    if (!relPath) return;

    getCurrentCwd().then(function (cwd) {
      // 拿不到当前会话 cwd 就静默，不兜底调用系统打开
      if (!cwd) return;
      // 相对路径拼接为绝对路径；若文本本身已是绝对路径（以 / 开头）则直接用
      var abs = relPath.charAt(0) === '/'
        ? relPath
        : cwd.replace(/\/+$/, '') + '/' + relPath.replace(/^\/+/, '');
      requestPreview(abs);
    });
  }, true); // true = capture 阶段
}
