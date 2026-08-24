  function renderMarkdown(content) {
    if (!content) return '';

    var html = '';
    var lines = content.split('\n');
    var inCodeBlock = false;
    var codeBlockContent = '';
    var codeBlockLang = '';
    var inList = false;
    var listType = '';
    var inBlockquote = false;
    var inTable = false;
    var tableRows = [];

    for (var i = 0; i < lines.length; i++) {
      var line = lines[i];

      // 代码块处理
      if (line.trim().startsWith('```')) {
        if (inCodeBlock) {
          html += '<pre class="md-code-block"><code class="language-' + codeBlockLang + '">' + escapeHtml(codeBlockContent.trim()) + '</code></pre>';
          inCodeBlock = false;
          codeBlockContent = '';
          codeBlockLang = '';
        } else {
          inCodeBlock = true;
          codeBlockLang = line.trim().slice(3).trim();
        }
        continue;
      }

      if (inCodeBlock) {
        codeBlockContent += line + '\n';
        continue;
      }

      // 空行
      if (line.trim() === '') {
        if (inList) {
          html += '</' + listType + '>';
          inList = false;
        }
        if (inBlockquote) {
          html += '</blockquote>';
          inBlockquote = false;
        }
        if (inTable) {
          html += renderTable(tableRows);
          tableRows = [];
          inTable = false;
        }
        html += '<br>';
        continue;
      }

      // 标题
      var headingMatch = line.match(/^(#{1,6})\s+(.+)/);
      if (headingMatch) {
        var level = headingMatch[1].length;
        html += '<h' + level + ' class="md-heading">' + renderInlineMarkdown(headingMatch[2]) + '</h' + level + '>';
        continue;
      }

      // 水平线
      if (/^(-{3,}|\*{3,}|_{3,})$/.test(line.trim())) {
        html += '<hr class="md-hr">';
        continue;
      }

      // 引用
      if (line.trim().startsWith('>')) {
        if (!inBlockquote) {
          html += '<blockquote class="md-blockquote">';
          inBlockquote = true;
        }
        html += renderInlineMarkdown(line.trim().slice(1).trim()) + '<br>';
        continue;
      }

      // 无序列表
      if (/^[\s]*[-*+]\s+/.test(line)) {
        if (!inList || listType !== 'ul') {
          if (inList) html += '</' + listType + '>';
          html += '<ul class="md-list">';
          inList = true;
          listType = 'ul';
        }
        html += '<li>' + renderInlineMarkdown(line.replace(/^[\s]*[-*+]\s+/, '')) + '</li>';
        continue;
      }

      // 有序列表
      if (/^[\s]*\d+\.\s+/.test(line)) {
        if (!inList || listType !== 'ol') {
          if (inList) html += '</' + listType + '>';
          html += '<ol class="md-list">';
          inList = true;
          listType = 'ol';
        }
        html += '<li>' + renderInlineMarkdown(line.replace(/^[\s]*\d+\.\s+/, '')) + '</li>';
        continue;
      }

      // 表格
      if (line.includes('|') && line.trim().startsWith('|')) {
        inTable = true;
        tableRows.push(line);
        continue;
      }

      // 普通段落
      html += '<p class="md-paragraph">' + renderInlineMarkdown(line) + '</p>';
    }

    // 关闭未关闭的标签
    if (inList) html += '</' + listType + '>';
    if (inBlockquote) html += '</blockquote>';
    if (inTable) html += renderTable(tableRows);

    return html;
  }

  // 渲染表格
  function renderTable(rows) {
    if (rows.length < 2) return '';

    var html = '<table class="md-table">';

    rows.forEach(function (row, index) {
      // 跳过分隔行（第二行，通常是 |---|---|）
      if (index === 1 && /^\|[\s-:|]+\|$/.test(row.trim())) return;

      var cells = row.split('|').filter(function (cell) { return cell.trim() !== ''; });
      var tag = index === 0 ? 'th' : 'td';

      if (index === 0) html += '<thead>';
      if (index === 1) html += '<tbody>';

      html += '<tr>';
      cells.forEach(function (cell) {
        html += '<' + tag + '>' + renderInlineMarkdown(cell.trim()) + '</' + tag + '>';
      });
      html += '</tr>';

      if (index === 0) html += '</thead>';
    });

    html += '</tbody></table>';
    return html;
  }

  // 渲染行内 Markdown
  function renderInlineMarkdown(text) {
    if (!text) return '';

    var result = escapeHtml(text);

    // 图片
    result = result.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img class="md-image" src="$2" alt="$1">');

    // 链接
    result = result.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a class="md-link" href="$2" target="_blank">$1</a>');

    // 粗体
    result = result.replace(/\*\*(.+?)\*\*/g, '<strong class="md-bold">$1</strong>');
    result = result.replace(/__(.+?)__/g, '<strong class="md-bold">$1</strong>');

    // 斜体
    result = result.replace(/\*(.+?)\*/g, '<em class="md-italic">$1</em>');
    result = result.replace(/_(.+?)_/g, '<em class="md-italic">$1</em>');

    // 删除线
    result = result.replace(/~~(.+?)~~/g, '<del class="md-strikethrough">$1</del>');

    // 行内代码
    result = result.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');

    // 高亮
    result = result.replace(/==(.+?)==/g, '<mark class="md-highlight">$1</mark>');

    return result;
  }

  // 简单的语法高亮函数

