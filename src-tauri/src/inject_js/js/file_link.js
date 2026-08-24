// ===== 文件链接拦截：用自有弹窗展示，而非 DSH 默认打开 =====
// DSH 工具调用结果中的文件路径以 <button class*="fileLink"> 渲染（见 dsh ui-tool 的 ToolRow.tsx，
// 使用 CSS module 的 css.fileLink；编译后类名含 "fileLink" 子串，hash 前缀每次构建会变）。
// 点击该按钮会触发 DSH 默认的 onOpenFile（在 React onClick 的 bubble 阶段 stopPropagation 后调用）。
// 这里在 document 捕获阶段拦截，阻止 DSH 默认行为，改用已注入的 window.previewFile 弹窗展示文件内容。
function initFileLinkInterceptor() {
  document.addEventListener('click', function (e) {
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
      window.previewFile(abs);
    });
  }, true); // true = capture 阶段
}
