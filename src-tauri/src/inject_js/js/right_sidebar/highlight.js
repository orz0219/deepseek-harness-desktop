  // ===== 代码高亮：关键字表（按扩展名索引，数组可被多个扩展名共享）=====
  // 仅覆盖「真正的关键字」；注释/字符串/数字由 highlightLiterals 统一处理。
  var KW = {};
  KW.js = KW.jsx = KW.ts = KW.tsx = ['const','let','var','function','return','if','else','for','while','do','switch','case','break','continue','new','this','class','extends','import','export','default','from','async','await','try','catch','finally','throw','typeof','instanceof','in','of','null','undefined','true','false','void','delete','yield','static','super','with','debugger','interface','type','enum','implements','package','private','protected','public','abstract','as','any','never','unknown','readonly','keyof','infer'];
  KW.py = ['def','class','if','elif','else','for','while','return','import','from','as','try','except','finally','raise','with','yield','lambda','pass','break','continue','and','or','not','in','is','None','True','False','global','nonlocal','assert','del','async','await'];
  KW.rs = ['fn','let','mut','const','static','struct','enum','trait','impl','pub','use','mod','crate','super','self','Self','match','if','else','for','while','loop','return','break','continue','in','as','where','async','await','move','ref','dyn','type','unsafe','extern','true','false'];
  KW.go = ['func','package','import','var','const','type','struct','interface','map','chan','go','defer','return','if','else','for','range','switch','case','default','break','continue','select','fallthrough','goto','nil','true','false'];
  KW.java = ['public','private','protected','class','interface','enum','implements','extends','import','package','static','final','void','int','long','double','float','boolean','char','byte','short','if','else','for','while','do','switch','case','default','break','continue','return','new','this','super','try','catch','finally','throw','throws','abstract','synchronized','volatile','transient','native','instanceof','null','true','false'];
  KW.cs = ['using','namespace','class','interface','struct','enum','public','private','protected','internal','static','readonly','const','void','int','long','double','float','bool','char','byte','string','var','if','else','for','foreach','while','do','switch','case','default','break','continue','return','new','this','base','try','catch','finally','throw','abstract','virtual','override','sealed','async','await','null','true','false','get','set'];
  KW.c = KW.cc = KW.cxx = KW.cpp = KW.h = KW.hpp = KW.hxx = ['int','long','short','char','float','double','void','unsigned','signed','const','static','struct','union','enum','typedef','sizeof','if','else','for','while','do','switch','case','default','break','continue','return','goto','auto','register','volatile','extern','inline','class','public','private','protected','virtual','template','typename','namespace','using','new','delete','this','try','catch','throw','true','false','nullptr','bool'];
  KW.rb = ['def','class','module','if','elsif','else','unless','for','while','until','do','begin','end','return','yield','break','next','redo','retry','require','require_relative','attr_accessor','attr_reader','attr_writer','self','nil','true','false','and','or','not','in','then','when','case','rescue','ensure','raise','lambda','proc'];
  KW.php = ['function','class','interface','trait','public','private','protected','static','final','abstract','const','var','echo','print','if','else','elseif','for','foreach','while','do','switch','case','default','break','continue','return','new','try','catch','finally','throw','isset','empty','include','require','namespace','use','extends','implements','true','false','null','array'];
  KW.swift = ['func','let','var','class','struct','enum','protocol','extension','import','if','else','for','while','do','switch','case','default','break','continue','return','guard','defer','where','in','as','is','try','catch','throw','throws','init','deinit','self','static','public','private','internal','override','mutating','nonmutating','true','false','nil','typealias','associatedtype'];
  KW.kt = KW.kts = ['fun','val','var','class','interface','object','companion','import','package','if','else','when','for','while','do','return','break','continue','throw','try','catch','finally','is','as','in','get','set','constructor','init','override','public','private','protected','internal','abstract','final','open','sealed','data','enum','suspend','true','false','null','Unit','this','super'];
  KW.sh = KW.bash = KW.zsh = ['if','then','else','elif','fi','for','while','do','done','case','esac','in','function','return','exit','export','local','set','unset','source','echo','cd','read','true','false','select','until'];
  KW.sql = ['select','from','where','insert','into','values','update','set','delete','create','table','drop','alter','index','view','join','inner','left','right','outer','on','group','by','order','having','limit','offset','distinct','as','and','or','not','null','is','in','like','between','union','all','case','when','then','else','end','begin','commit','rollback','primary','key','foreign','references','default','true','false'];
  KW.lua = ['function','end','if','then','else','elseif','for','while','do','repeat','until','return','break','local','nil','true','false','and','or','not','in','pairs','ipairs','require'];
  KW.dart = ['import','library','part','class','mixin','enum','extension','interface','abstract','implements','with','static','final','const','var','void','int','double','String','bool','dynamic','if','else','for','while','do','switch','case','default','break','continue','return','new','this','super','try','catch','finally','throw','async','await','get','set','true','false','null'];
  KW.yaml = KW.yml = KW.toml = ['true','false','null','yes','no','on','off'];
  KW.json = ['true','false','null'];

  // 单行注释符号（html/css/md 走专用分支，不在此列）
  var COMMENT = {
    js:'//', jsx:'//', ts:'//', tsx:'//',
    rs:'//', go:'//', java:'//', cs:'//',
    c:'//', cc:'//', cxx:'//', cpp:'//', h:'//', hpp:'//', hxx:'//',
    swift:'//', kt:'//', kts:'//', php:'//', dart:'//',
    py:'#', sh:'#', bash:'#', zsh:'#', rb:'#',
    yaml:'#', yml:'#', toml:'#',
    sql:'--', lua:'--'
  };

  function escapeRegex(s) { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

  // 通用后处理：注释 + 字符串 + 数字
  function highlightLiterals(escaped, commentToken) {
    if (commentToken) {
      escaped = escaped.replace(new RegExp(escapeRegex(commentToken) + '.*$'), '<span class="hl-comment">$&</span>');
    }
    // 字符串：双引号（escapeHtml 已转义为 &quot;）与单引号（真实 escapeHtml 不转义单引号，故裸 ' 也要匹配）
    escaped = escaped.replace(/(&quot;[^&]*?&quot;)/g, '<span class="hl-string">$1</span>');
    escaped = escaped.replace(/(&#x27;[^&]*?&#x27;|'[^']*?')/g, '<span class="hl-string">$1</span>');
    // 数字
    escaped = escaped.replace(/\b(\d+\.?\d*)\b/g, '<span class="hl-number">$1</span>');
    return escaped;
  }

  function highlightCode(line, lang) {
    if (!line) return '';
    var escaped = escapeHtml(line);

    // 1) 专用分支：结构特殊的标记 / 样式语言
    if (lang === 'html' || lang === 'htm' || lang === 'xml') {
      // HTML/XML：单次合并正则 + 回调，避免属性正则二次匹配到标签高亮已插入的
      // <span class="hl-tag"> 里的 class=，从而生成嵌套错乱的标签。
      escaped = escaped.replace(/(&lt;\/?)([\w-]+)|([\w-]+)(=)/g, function (m, lt, tag, attr, eq) {
        if (tag !== undefined) return lt + '<span class="hl-tag">' + tag + '</span>';
        return '<span class="hl-attr">' + attr + '</span>' + eq;
      });
      return escaped;
    }
    if (lang === 'vue') {
      // Vue 单文件组件：template 用 HTML、script 用 JS、style 用 CSS。
      // 逐行高亮无法切分区块，这里组合 HTML 标签 + JS 关键字 + Vue 指令 + 注释。
      // Vue 绑定指令：v-xxx、@event、:prop（先于标签/属性处理，避免被当成普通属性）
      escaped = escaped.replace(/(v-[\w-]+|@[\w-]+|:[\w-]+)/g, '<span class="hl-keyword">$1</span>');
      // 标签 + 属性（template 部分）
      escaped = escaped.replace(/(&lt;\/?)([\w-]+)|([\w-]+)(=)/g, function (m, lt, tag, attr, eq) {
        if (tag !== undefined) return lt + '<span class="hl-tag">' + tag + '</span>';
        return '<span class="hl-attr">' + attr + '</span>' + eq;
      });
      // JS 关键字（script 部分）
      var vk = KW.js;
      if (vk && vk.length) escaped = escaped.replace(new RegExp('\\b(' + vk.join('|') + ')\\b', 'g'), '<span class="hl-keyword">$1</span>');
      // 注释：JS // 与 HTML <!-- -->
      escaped = escaped.replace(/(\/\/.*$|&lt;!--.*--&gt;)/g, '<span class="hl-comment">$1</span>');
      // 字符串 + 数字
      escaped = escaped.replace(/(&quot;[^&]*?&quot;)/g, '<span class="hl-string">$1</span>');
      escaped = escaped.replace(/(&#x27;[^&]*?&#x27;|'[^']*?')/g, '<span class="hl-string">$1</span>');
      escaped = escaped.replace(/\b(\d+\.?\d*)\b/g, '<span class="hl-number">$1</span>');
      return escaped;
    }
    if (lang === 'css' || lang === 'scss' || lang === 'less') {
      escaped = escaped.replace(/([.#][\w-]+)/g, '<span class="hl-selector">$1</span>');
      escaped = escaped.replace(/([\w-]+)(\s*:)/g, '<span class="hl-property">$1</span>$2');
      return escaped;
    }
    if (lang === 'json') {
      var jk = KW.json;
      if (jk && jk.length) escaped = escaped.replace(new RegExp('\\b(' + jk.join('|') + ')\\b', 'g'), '<span class="hl-keyword">$1</span>');
      escaped = escaped.replace(/(&quot;[\w]+&quot;)(\s*:)/g, '<span class="hl-key">$1</span>$2');
      return highlightLiterals(escaped, null);
    }
    if (lang === 'md' || lang === 'markdown') {
      escaped = escaped.replace(/^(#{1,6}\s.*)$/gm, '<span class="hl-heading">$1</span>');
      escaped = escaped.replace(/(\*\*|__)(.*?)\1/g, '<span class="hl-bold">$1$2$1</span>');
      escaped = escaped.replace(/(\*|_)(.*?)\1/g, '<span class="hl-italic">$1$2$1</span>');
      escaped = escaped.replace(/(`[^`]+`)/g, '<span class="hl-code">$1</span>');
      escaped = escaped.replace(/(\[.*?\]\(.*?\))/g, '<span class="hl-link">$1</span>');
      return escaped;
    }

    // 2) 关键字高亮（覆盖主流编程语言；缺失的关键字不影响其他规则）
    // 扩展名以 sql 结尾的文件（如 .psql / .mysql / .pgsql）一并按 SQL 高亮
    var kws = KW[lang];
    if (!kws && lang && lang.endsWith('sql')) kws = KW.sql;
    if (kws && kws.length) {
      // SQL 关键字常写作大写（SELECT/FROM），故 SQL 类扩展名用大小写不敏感匹配
      var reFlags = (lang && lang.endsWith('sql')) ? 'gi' : 'g';
      escaped = escaped.replace(new RegExp('\\b(' + kws.join('|') + ')\\b', reFlags), '<span class="hl-keyword">$1</span>');
    }

    // 3) 语言特定附加结构
    if (lang === 'py') {
      // Python 装饰器
      escaped = escaped.replace(/(@\w+)/g, '<span class="hl-decorator">$1</span>');
    } else if (lang === 'js' || lang === 'jsx' || lang === 'ts' || lang === 'tsx') {
      // 函数调用
      escaped = escaped.replace(/\b([a-zA-Z_$][\w$]*)\s*(?=\()/g, '<span class="hl-function">$1</span>');
    }

    // 4) 通用：注释 + 字符串 + 数字
    // 扩展名以 sql 结尾的文件统一用 -- 注释
    var cm = COMMENT[lang];
    if (cm === undefined && lang && lang.endsWith('sql')) cm = '--';
    return highlightLiterals(escaped, cm);
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
