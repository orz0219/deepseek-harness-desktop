  function highlightCode(line, lang) {
    if (!line) return '';
    var escaped = escapeHtml(line);

    // 语言特定高亮（优先处理，避免通用规则干扰）
    if (['js', 'jsx', 'ts', 'tsx'].includes(lang)) {
      // JavaScript/TypeScript 关键字
      var keywords = ['const', 'let', 'var', 'function', 'return', 'if', 'else', 'for', 'while', 'do', 'switch', 'case', 'break', 'continue', 'new', 'this', 'class', 'extends', 'import', 'export', 'default', 'from', 'async', 'await', 'try', 'catch', 'finally', 'throw', 'typeof', 'instanceof', 'in', 'of', 'null', 'undefined', 'true', 'false', 'void', 'delete', 'yield', 'static', 'super', 'with', 'debugger', 'interface', 'type', 'enum', 'implements', 'package', 'private', 'protected', 'public', 'abstract', 'as', 'any', 'never', 'unknown', 'readonly', 'keyof', 'infer'];
      var keywordRegex = new RegExp('\\b(' + keywords.join('|') + ')\\b', 'g');
      escaped = escaped.replace(keywordRegex, '<span class="hl-keyword">$1</span>');
      // 函数调用
      escaped = escaped.replace(/\b([a-zA-Z_$][\w$]*)\s*(?=\()/g, '<span class="hl-function">$1</span>');
      // 单行注释
      escaped = escaped.replace(/(\/\/.*)$/g, '<span class="hl-comment">$1</span>');
    } else if (lang === 'py') {
      // Python 关键字
      var pyKeywords = ['def', 'class', 'if', 'elif', 'else', 'for', 'while', 'return', 'import', 'from', 'as', 'try', 'except', 'finally', 'raise', 'with', 'yield', 'lambda', 'pass', 'break', 'continue', 'and', 'or', 'not', 'in', 'is', 'None', 'True', 'False', 'global', 'nonlocal', 'assert', 'del', 'async', 'await'];
      var pyKeywordRegex = new RegExp('\\b(' + pyKeywords.join('|') + ')\\b', 'g');
      escaped = escaped.replace(pyKeywordRegex, '<span class="hl-keyword">$1</span>');
      // Python 注释
      escaped = escaped.replace(/(#.*$)/g, '<span class="hl-comment">$1</span>');
      // Python 装饰器
      escaped = escaped.replace(/(@\w+)/g, '<span class="hl-decorator">$1</span>');
    } else if (['html', 'htm', 'xml'].includes(lang)) {
      // HTML/XML：单次合并正则 + 回调，避免属性正则二次匹配到标签高亮已插入的
      // <span class="hl-tag"> 里的 class=，从而生成嵌套错乱的标签。
      escaped = escaped.replace(/(&lt;\/?)([\w-]+)|([\w-]+)(=)/g, function (m, lt, tag, attr, eq) {
        if (tag !== undefined) return lt + '<span class="hl-tag">' + tag + '</span>';
        return '<span class="hl-attr">' + attr + '</span>' + eq;
      });
    } else if (['css', 'scss', 'less'].includes(lang)) {
      // CSS 选择器
      escaped = escaped.replace(/([.#][\w-]+)/g, '<span class="hl-selector">$1</span>');
      // CSS 属性
      escaped = escaped.replace(/([\w-]+)(\s*:)/g, '<span class="hl-property">$1</span>$2');
    } else if (lang === 'json') {
      // JSON 键
      escaped = escaped.replace(/(&quot;[\w]+&quot;)(\s*:)/g, '<span class="hl-key">$1</span>$2');
    } else if (lang === 'md') {
      // Markdown 标题
      escaped = escaped.replace(/^(#{1,6}\s.*)$/gm, '<span class="hl-heading">$1</span>');
      // Markdown 粗体
      escaped = escaped.replace(/(\*\*|__)(.*?)\1/g, '<span class="hl-bold">$1$2$1</span>');
      // Markdown 斜体
      escaped = escaped.replace(/(\*|_)(.*?)\1/g, '<span class="hl-italic">$1$2$1</span>');
      // Markdown 代码
      escaped = escaped.replace(/(`[^`]+`)/g, '<span class="hl-code">$1</span>');
      // Markdown 链接
      escaped = escaped.replace(/(\[.*?\]\(.*?\))/g, '<span class="hl-link">$1</span>');
    } else {
      // 通用高亮（其他语言）
      // 字符串（单引号和双引号）
      escaped = escaped.replace(/(&quot;[^&]*?&quot;)/g, '<span class="hl-string">$1</span>');
      escaped = escaped.replace(/(&#x27;[^&]*?&#x27;)/g, '<span class="hl-string">$1</span>');
      // 数字
      escaped = escaped.replace(/\b(\d+\.?\d*)\b/g, '<span class="hl-number">$1</span>');
    }

    return escaped;
  }

  // 渲染源代码（带行号和语法高亮）
  function renderSourceCode(content, ext) {
    var html = '';
    var lines = content.split('\n');
    lines.forEach(function (line, i) {
      html += '<div class="dsh-rs-preview-line">';
      html += '<span class="dsh-rs-preview-line-num">' + (i + 1) + '</span>';
      html += '<span class="dsh-rs-preview-line-content">' + highlightCode(line, ext) + '</span>';
      html += '</div>';
    });
    return html;
  }


