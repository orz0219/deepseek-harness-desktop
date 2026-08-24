  // ===== 右侧侧边栏样式 =====
  var RS_CSS =
    // 容器
    '#' + RS_ID + '{position:fixed;top:0;right:0;bottom:0;z-index:2147483645;display:flex;pointer-events:none;}' +
    '#' + RS_ID + ' *{box-sizing:border-box;}' +
    // 宿主（dsh web）主内容区避让：面板展开时通过给 #root 设置内联 margin-right 右移（见 applyHostPush），避免遮挡对话
    // 切换按钮
    '#' + RS_TOGGLE_ID + '{position:absolute;top:50%;right:0;transform:translateY(-50%);width:28px;height:52px;display:flex;align-items:center;justify-content:center;background:#ffffff;border:1px solid #e5e7eb;border-right:none;border-radius:8px 0 0 8px;cursor:pointer;color:#374151;pointer-events:auto;transition:background .15s ease,color .15s ease,box-shadow .15s ease,opacity .2s ease;opacity:1;box-shadow:-3px 0 10px rgba(0,0,0,.12);z-index:1;}' +
    '#' + RS_TOGGLE_ID + ':hover{background:#3b6cf6;color:#fff;border-color:#3b6cf6;box-shadow:-3px 0 14px rgba(59,108,246,.45);}' +
    '#' + RS_TOGGLE_ID + ' svg{display:block;}' +
    // 面板主体
    '#' + RS_PANEL_ID + '{width:360px;height:100%;background:#ffffff;border-left:1px solid #e5e7eb;display:flex;flex-direction:column;transform:translateX(100%);transition:transform .25s ease;pointer-events:auto;box-shadow:-4px 0 20px rgba(0,0,0,0.05);}' +
    '#' + RS_ID + '.expanded #' + RS_PANEL_ID + '{transform:translateX(0);}' +
    '#' + RS_ID + '.expanded #' + RS_TOGGLE_ID + '{right:360px;opacity:1;}' +
    // 标签页导航
    '.dsh-rs-tabs{display:flex;align-items:center;gap:0;padding:0 12px;border-bottom:1px solid #e5e7eb;background:#fafbfc;min-height:40px;}' +
    '.dsh-rs-tab{flex:none;padding:8px 12px;font-size:12px;font-weight:500;color:#6b7280;background:transparent;border:none;border-bottom:2px solid transparent;cursor:pointer;transition:color .15s ease,border-color .15s ease;display:inline-flex;align-items:center;gap:4px;}' +
    '.dsh-rs-tab:hover{color:#374151;}' +
    '.dsh-rs-tab.active{color:#3b6cf6;border-bottom-color:#3b6cf6;}' +
    '.dsh-rs-tab svg{flex:none;}' +
    // 内容区
    '.dsh-rs-content{flex:1;min-height:0;overflow:hidden;display:flex;flex-direction:column;}' +
    '.dsh-rs-pane{flex:1;min-height:0;overflow-y:auto;padding:12px;display:none;}' +
    '.dsh-rs-pane.active{display:block;}' +
    // 工具栏
    '.dsh-rs-toolbar{display:flex;align-items:center;gap:8px;padding:8px 12px;border-bottom:1px solid #eef0f3;background:#fafbfc;}' +
    '.dsh-rs-toolbar-title{font-size:12px;font-weight:400;color:#374151;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;}' +
    '.dsh-rs-toolbar-btn{width:26px;height:26px;display:flex;align-items:center;justify-content:center;border:none;border-radius:6px;background:transparent;color:#6b7280;cursor:pointer;transition:background .15s ease,color .15s ease;}' +
    '.dsh-rs-toolbar-btn:hover{background:#f3f4f6;color:#374151;}' +
    '.dsh-rs-toolbar-btn svg{display:block;}' +
    // 状态栏
    '.dsh-rs-statusbar{display:flex;align-items:center;gap:8px;padding:6px 12px;border-top:1px solid #eef0f3;background:#fafbfc;font-size:11px;color:#9ca3af;}' +
    '.dsh-rs-statusbar-item{display:inline-flex;align-items:center;gap:4px;}' +
    // Git 改动：文件列表（点击后在弹框显示详情）
    '.dsh-rs-git-list{display:flex;flex-direction:column;}' +
    '.dsh-rs-git-item{display:flex;align-items:center;gap:10px;padding:9px 12px;border-bottom:1px solid #f1f3f5;cursor:pointer;user-select:none;transition:background .12s ease;}' +
    '.dsh-rs-git-item:hover{background:#f6f8fb;}' +
    '.dsh-rs-git-item:active{background:#eef2f7;}' +
    '.dsh-rs-git-status{flex:none;font-size:11px;padding:1px 7px;border-radius:4px;font-weight:600;}' +
    '.dsh-rs-git-status.modified{background:#fef3c7;color:#92400e;}' +
    '.dsh-rs-git-status.added{background:#dcfce7;color:#166534;}' +
    '.dsh-rs-git-status.deleted{background:#fee2e2;color:#b91c1c;}' +
    '.dsh-rs-git-status.renamed{background:#e5edff;color:#1e40af;}' +
    '.dsh-rs-git-path{flex:1;min-width:0;font-size:13px;color:#374151;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
    '.dsh-rs-git-stats{flex:none;font-size:11px;color:#6b7280;font-variant-numeric:tabular-nums;white-space:nowrap;}' +
    '.dsh-rs-git-stats .add{color:#16a34a;}' +
    '.dsh-rs-git-stats .del{color:#dc2626;}' +
    // 弹框内 diff 详情
    '.dsh-rs-git-diff-meta{display:flex;align-items:center;gap:10px;padding:10px 16px;border-bottom:1px solid #eef0f3;background:#fafbfc;}' +
    '.dsh-rs-git-diff-stats{font-size:12px;color:#6b7280;font-variant-numeric:tabular-nums;}' +
    '.dsh-rs-git-diff-stats .add{color:#16a34a;}' +
    '.dsh-rs-git-diff-stats .del{color:#dc2626;}' +
    '.dsh-rs-git-diff-content{flex:1;min-height:0;overflow:auto;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:13px;line-height:1.6;}' +
    // diff 行高亮（弹框详情共用）
    '.dsh-rs-diff-line{padding:0 14px;white-space:pre;}' +
    '.dsh-rs-diff-line.addition{background:#dcfce7;color:#166534;}' +
    '.dsh-rs-diff-line.deletion{background:#fee2e2;color:#b91c1c;}' +
    '.dsh-rs-diff-line.context{color:#6b7280;}' +
    '.dsh-rs-diff-line .line-num{display:inline-block;width:40px;color:#9ca3af;text-align:right;margin-right:12px;user-select:none;}' +
    // 文件树样式
    '.dsh-rs-tree{font-size:13px;position:relative;}' +
    '.dsh-rs-tree-item{display:flex;align-items:center;gap:6px;padding:4px 8px;border-radius:6px;cursor:pointer;user-select:none;transition:background .15s ease;position:relative;}' +
    '.dsh-rs-tree-item:hover{background:#f3f4f6;}' +
    '.dsh-rs-tree-item.selected{background:#e5edff;}' +
    '.dsh-rs-tree-item svg{flex:none;color:#6b7280;}' +
    '.dsh-rs-tree-item .name{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}' +
    '.dsh-rs-tree-item .chevron{flex:none;width:16px;height:16px;transition:transform .15s ease;}' +
    '.dsh-rs-tree-item .chevron.expanded{transform:rotate(90deg);}' +
    '.dsh-rs-tree-children{display:none;}' +
    '.dsh-rs-tree-children.expanded{display:block;}' +
    // 懒加载占位 / 空目录提示
    '.dsh-rs-tree-loading,.dsh-rs-tree-empty{padding:4px 8px 4px 24px;font-size:12px;color:#9ca3af;}' +
    '.dsh-rs-tree-loading{color:#6b7280;}' +
    // 文件树过滤框
    '.dsh-rs-tree-filter{position:sticky;top:0;z-index:2;margin:-12px -12px 8px;padding:8px 12px;background:#fff;border-bottom:1px solid #eef0f3;}' +
    '.dsh-rs-tree-filter-input{width:100%;box-sizing:border-box;padding:6px 10px;font-size:12px;font-family:inherit;color:#374151;background:#f8fafc;border:1px solid #e5e7eb;border-radius:6px;outline:none;transition:border-color .15s ease,box-shadow .15s ease;}' +
    '.dsh-rs-tree-filter-input::placeholder{color:#9ca3af;}' +
    '.dsh-rs-tree-filter-input:focus{border-color:#3b6cf6;box-shadow:0 0 0 3px rgba(59,108,246,.12);background:#fff;}' +
    '.dsh-rs-tree-wrap{padding:0 12px 12px;}' +
    // 树状连接线
    '.tree-line{position:absolute;top:0;bottom:0;width:1px;background:#e5e7eb;}' +
    '.tree-branch{position:absolute;top:12px;width:12px;height:1px;background:#e5e7eb;}' +
    '.tree-branch-last{height:calc(100% - 12px);background:transparent;}' +
    // 文件夹图标样式
    '.dsh-rs-tree-item .folder-icon{flex:none;display:inline-flex;align-items:center;}' +
    '.dsh-rs-tree-item .folder-icon svg{transition:color .15s ease;}' +
    '.dsh-rs-tree-item .folder-icon.opened svg{color:#3b6cf6;}' +
    // 文件类型颜色
    '.dsh-rs-tree-item.file-dir .name{color:#5b9bd5;}' +
    '.dsh-rs-tree-item.file-js svg,.dsh-rs-tree-item.file-js .name{color:#f7df1e;}' +
    '.dsh-rs-tree-item.file-ts svg,.dsh-rs-tree-item.file-ts .name{color:#3178c6;}' +
    '.dsh-rs-tree-item.file-py svg,.dsh-rs-tree-item.file-py .name{color:#3776ab;}' +
    '.dsh-rs-tree-item.file-html svg,.dsh-rs-tree-item.file-html .name{color:#e34f26;}' +
    '.dsh-rs-tree-item.file-css svg,.dsh-rs-tree-item.file-css .name{color:#1572b6;}' +
    '.dsh-rs-tree-item.file-json svg,.dsh-rs-tree-item.file-json .name{color:#5b5b5b;}' +
    '.dsh-rs-tree-item.file-yaml svg,.dsh-rs-tree-item.file-yaml .name{color:#cb171e;}' +
    '.dsh-rs-tree-item.file-md svg,.dsh-rs-tree-item.file-md .name{color:#083fa1;}' +
    '.dsh-rs-tree-item.file-image svg{color:#8b5cf6;}' +
    '.dsh-rs-tree-item.file-shell svg,.dsh-rs-tree-item.file-shell .name{color:#4eaa25;}' +
    '.dsh-rs-tree-item.file-sql svg,.dsh-rs-tree-item.file-sql .name{color:#f29111;}' +
    '.dsh-rs-tree-item.file-rs svg,.dsh-rs-tree-item.file-rs .name{color:#dea584;}' +
    '.dsh-rs-tree-item.file-go svg,.dsh-rs-tree-item.file-go .name{color:#00add8;}' +
    '.dsh-rs-tree-item.file-java svg,.dsh-rs-tree-item.file-java .name{color:#ed8b00;}' +
    '.dsh-rs-tree-item.file-c svg,.dsh-rs-tree-item.file-c .name{color:#555555;}' +
    '.dsh-rs-tree-item.file-rb svg,.dsh-rs-tree-item.file-rb .name{color:#cc342d;}' +
    '.dsh-rs-tree-item.file-php svg,.dsh-rs-tree-item.file-php .name{color:#777bb4;}' +
    '.dsh-rs-tree-item.file-swift svg,.dsh-rs-tree-item.file-swift .name{color:#f05138;}' +
    '.dsh-rs-tree-item.file-kt svg,.dsh-rs-tree-item.file-kt .name{color:#7f52ff;}' +
    '.dsh-rs-tree-item.file-cs svg,.dsh-rs-tree-item.file-cs .name{color:#239120;}' +
    '.dsh-rs-tree-item.file-xml svg,.dsh-rs-tree-item.file-xml .name{color:#f16529;}' +
    '.dsh-rs-tree-item.file-config svg{color:#6b7280;}' +
    '.dsh-rs-tree-item.file-text svg{color:#6b7280;}' +
    '.dsh-rs-tree-item.file-pdf svg{color:#ff0000;}' +
    '.dsh-rs-tree-item.file-doc svg{color:#2b579a;}' +
    '.dsh-rs-tree-item.file-archive svg{color:#ffa500;}' +
    '.dsh-rs-tree-item.file-graphql svg{color:#e535ab;}' +
    '.dsh-rs-tree-item.file-default svg{color:#6b7280;}' +
    // 预览样式
    '.dsh-rs-preview-header{display:flex;align-items:center;gap:8px;padding:8px 12px;background:#f9fafb;border-bottom:1px solid #e5e7eb;font-size:12px;}' +
    '.dsh-rs-preview-path{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:#374151;}' +
    '.dsh-rs-preview-path span{color:#9ca3af;}' +
    '.dsh-rs-preview-content{flex:1;min-height:0;overflow:auto;padding:12px;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:12px;line-height:1.6;white-space:pre;tab-size:2;}' +
    '.dsh-rs-preview-line{display:flex;}' +
    '.dsh-rs-preview-line-num{flex:none;width:40px;color:#9ca3af;text-align:right;margin-right:16px;user-select:none;}' +
    '.dsh-rs-preview-line-content{flex:1;min-width:0;white-space:pre;tab-size:2;}' +
    // 语法高亮样式
    '.hl-keyword{color:#d73a49;font-weight:500;}' +
    '.hl-string{color:#032f62;}' +
    '.hl-number{color:#005cc5;}' +
    '.hl-comment{color:#6a737d;font-style:italic;}' +
    '.hl-function{color:#6f42c1;}' +
    '.hl-tag{color:#22863a;}' +
    '.hl-attr{color:#6f42c1;}' +
    '.hl-selector{color:#6f42c1;}' +
    '.hl-property{color:#005cc5;}' +
    '.hl-key{color:#005cc5;}' +
    '.hl-decorator{color:#e36209;}' +
    '.hl-heading{color:#d73a49;font-weight:600;}' +
    '.hl-bold{font-weight:600;}' +
    '.hl-italic{font-style:italic;}' +
    '.hl-code{background:#f6f8fa;padding:2px 4px;border-radius:3px;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;}' +
    '.hl-link{color:#0366d6;text-decoration:underline;}' +
    // 预览切换按钮
    '.dsh-rs-preview-toggle{display:flex;gap:4px;margin-left:auto;}' +
    '.dsh-rs-preview-btn{padding:4px 10px;font-size:11px;border:1px solid #e5e7eb;background:#fff;color:#6b7280;border-radius:4px;cursor:pointer;transition:all .15s ease;}' +
    '.dsh-rs-preview-btn:hover{background:#f9fafb;color:#374151;}' +
    '.dsh-rs-preview-btn.active{background:#3b6cf6;color:#fff;border-color:#3b6cf6;}' +
    // Markdown 渲染样式（作用于弹框内的 .dsh-rs-modal-markdown 容器）
    '.dsh-rs-modal-markdown{flex:1;min-height:0;overflow:auto;padding:28px 32px;background:#fff;font-family:-apple-system,BlinkMacSystemFont,system-ui,"Segoe UI",Roboto,"PingFang SC","Microsoft YaHei",sans-serif;font-size:15px;line-height:1.75;color:#374151;}' +
    '.dsh-rs-modal-markdown>:first-child{margin-top:0;}' +
    '.dsh-rs-modal-markdown h1,.dsh-rs-modal-markdown h2,.dsh-rs-modal-markdown h3,.dsh-rs-modal-markdown h4,.dsh-rs-modal-markdown h5,.dsh-rs-modal-markdown h6{margin:1.5em 0 .55em;font-weight:600;line-height:1.3;color:#111827;}' +
    '.dsh-rs-modal-markdown h1{font-size:1.7em;padding-bottom:.3em;border-bottom:1px solid #eceef1;}' +
    '.dsh-rs-modal-markdown h2{font-size:1.4em;padding-bottom:.3em;border-bottom:1px solid #eceef1;}' +
    '.dsh-rs-modal-markdown h3{font-size:1.2em;}' +
    '.dsh-rs-modal-markdown h4{font-size:1.05em;}' +
    '.dsh-rs-modal-markdown h5,.dsh-rs-modal-markdown h6{font-size:1em;color:#6b7280;}' +
    '.dsh-rs-modal-markdown p{margin:.85em 0;}' +
    '.dsh-rs-modal-markdown a{color:#3b6cf6;text-decoration:none;border-bottom:1px solid rgba(59,108,246,.35);transition:border-color .15s ease;}' +
    '.dsh-rs-modal-markdown a:hover{border-bottom-color:#3b6cf6;}' +
    '.dsh-rs-modal-markdown strong{font-weight:600;color:#1f2937;}' +
    '.dsh-rs-modal-markdown em{font-style:italic;}' +
    '.dsh-rs-modal-markdown del{text-decoration:line-through;color:#9ca3af;}' +
    '.dsh-rs-modal-markdown code{background:#f1f3f5;color:#c2185b;padding:.15em .4em;border-radius:5px;font-size:.88em;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;}' +
    '.dsh-rs-modal-markdown pre{background:#0f172a;color:#e2e8f0;padding:16px 18px;border:1px solid #1e293b;border-radius:10px;overflow-x:auto;margin:1em 0;font-size:13.5px;line-height:1.6;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;}' +
    '.dsh-rs-modal-markdown pre code{background:transparent;color:inherit;padding:0;font-size:inherit;border-radius:0;}' +
    '.dsh-rs-modal-markdown blockquote{margin:1em 0;padding:.6em 1em;color:#475569;border-left:4px solid #c7d2fe;background:#f5f7ff;border-radius:0 8px 8px 0;}' +
    '.dsh-rs-modal-markdown ul,.dsh-rs-modal-markdown ol{margin:.85em 0;padding-left:1.6em;}' +
    '.dsh-rs-modal-markdown li{margin:.3em 0;}' +
    '.dsh-rs-modal-markdown li::marker{color:#9ca3af;}' +
    '.dsh-rs-modal-markdown hr{border:none;height:1px;background:#eceef1;margin:1.6em 0;}' +
    '.dsh-rs-modal-markdown table{border-collapse:collapse;width:100%;margin:1em 0;font-size:.95em;border:1px solid #e5e7eb;border-radius:8px;overflow:hidden;}' +
    '.dsh-rs-modal-markdown th,.dsh-rs-modal-markdown td{border:1px solid #e5e7eb;padding:9px 13px;text-align:left;}' +
    '.dsh-rs-modal-markdown th{background:#f8fafc;font-weight:600;color:#111827;}' +
    '.dsh-rs-modal-markdown tr:nth-child(even){background:#fafbfc;}' +
    '.dsh-rs-modal-markdown tr:hover{background:#f1f5ff;}' +
    '.dsh-rs-modal-markdown img{max-width:100%;height:auto;border-radius:10px;margin:.6em 0;box-shadow:0 1px 4px rgba(0,0,0,.08);}' +
    '.dsh-rs-modal-markdown mark{background:#fef08a;color:#713f12;padding:.1em .35em;border-radius:4px;}' +
    // 预览弹框样式
    '.dsh-rs-modal{position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;background:rgba(17,24,39,.5);}' +
    '.dsh-rs-modal-content{position:absolute;width:70vw;height:70vh;left:15vw;top:15vh;background:#fff;border-radius:12px;box-shadow:0 20px 60px rgba(0,0,0,.3);display:flex;flex-direction:column;overflow:hidden;min-width:400px;min-height:300px;}' +
    '.dsh-rs-modal-header{display:flex;align-items:center;justify-content:space-between;padding:12px 16px;background:#f9fafb;border-bottom:1px solid #e5e7eb;cursor:move;user-select:none;}' +
    '.dsh-rs-modal-path{flex:1;font-size:13px;color:#374151;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;}' +
    '.dsh-rs-modal-actions{display:flex;gap:6px;margin-left:12px;}' +
    '.dsh-rs-modal-btn{width:28px;height:28px;display:flex;align-items:center;justify-content:center;border:none;border-radius:6px;background:transparent;color:#6b7280;cursor:pointer;font-size:14px;transition:all .15s ease;}' +
    '.dsh-rs-modal-btn:hover{background:#e5e7eb;color:#374151;}' +
    '.dsh-rs-modal-close:hover{background:#fee2e2;color:#b91c1c;}' +
    '.dsh-rs-modal-body{flex:1;min-height:0;overflow:auto;display:flex;flex-direction:column;}' +
    '.dsh-rs-modal-toolbar{display:flex;gap:4px;padding:8px 16px;background:#fafbfc;border-bottom:1px solid #eef0f3;}' +
    '.dsh-rs-modal-markdown{flex:1;min-height:0;overflow:auto;}' +
    '.dsh-rs-modal-code{flex:1;overflow:auto;padding:0;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;font-size:13px;line-height:1.6;white-space:pre;tab-size:2;}' +
    '.dsh-rs-modal-resize-handle{position:absolute;right:0;bottom:0;width:16px;height:16px;cursor:nwse-resize;}' +
    '.dsh-rs-modal-resize-handle::after{content:"";position:absolute;right:4px;bottom:4px;width:8px;height:8px;border-right:2px solid #9ca3af;border-bottom:2px solid #9ca3af;}' +
    // 图片预览
    '.dsh-rs-modal-image-wrap{flex:1;min-height:0;overflow:auto;display:flex;align-items:center;justify-content:center;padding:16px;background:#f9fafb;}' +
    '.dsh-rs-modal-image{max-width:100%;max-height:100%;object-fit:contain;border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,.12);}' +
    // 空状态
    '.dsh-rs-empty{display:flex;flex-direction:column;align-items:center;justify-content:center;padding:40px 20px;color:#9ca3af;text-align:center;}' +
    '.dsh-rs-empty svg{margin-bottom:12px;opacity:0.5;}' +
    '.dsh-rs-empty .title{font-size:13px;font-weight:500;margin-bottom:4px;}' +
    '.dsh-rs-empty .desc{font-size:12px;}' +
    // 加载状态
    '.dsh-rs-loading{display:flex;align-items:center;justify-content:center;gap:8px;padding:40px 20px;color:#6b7280;font-size:13px;}' +
    '.dsh-rs-loading .spinner{width:16px;height:16px;border:2px solid #e5e7eb;border-top-color:#3b6cf6;border-radius:50%;animation:dsh-rs-spin .8s linear infinite;}' +
    '@keyframes dsh-rs-spin{to{transform:rotate(360deg);}}' +
    // ---- 文件树右键上下文菜单 ----
    '.dsh-rs-ctxmenu{position:fixed;z-index:2147483647;min-width:168px;background:#ffffff;border:1px solid #e5e7eb;border-radius:10px;box-shadow:0 12px 34px rgba(17,24,39,.18);padding:5px;font-family:inherit;color:#1f2937;}' +
    '.dsh-rs-ctxmenu-item{padding:7px 12px;border-radius:7px;font-size:13px;cursor:pointer;white-space:nowrap;color:#374151;}' +
    '.dsh-rs-ctxmenu-item:hover{background:#f3f4f6;color:#111827;}' +
    // ---- 面板内提示（替代浮动 toast，避免被 dsh web 样式干扰）----
    '.dsh-rs-hint{display:none;align-items:center;gap:6px;padding:6px 12px;border-top:1px solid #eef0f3;background:#fafbfc;font-size:12px;color:#374151;min-height:30px;}' +
    '.dsh-rs-hint.show{display:flex;}' +
    '.dsh-rs-hint.err{color:#b91c1c;background:#fef2f2;border-top-color:#fecaca;}';


