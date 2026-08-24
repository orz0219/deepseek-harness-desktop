
  // ===== 右侧侧边栏常量 =====
  var RS_ID = 'dsh-right-sidebar';
  var RS_TOGGLE_ID = 'dsh-rs-toggle';
  var RS_PANEL_ID = 'dsh-rs-panel';
  var RS_STORAGE_KEY = 'dsh-right-sidebar-state';

  // 切换按钮图标（左箭头/右箭头）
  var ICON_CHEVRON_LEFT =
    '<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<path d="M10 4L6 8l4 4"/>' +
    '</svg>';

  var ICON_CHEVRON_RIGHT =
    '<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<path d="M6 4l4 4-4 4"/>' +
    '</svg>';

  // Git 图标
  var ICON_GIT =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<circle cx="8" cy="8" r="3"/>' +
    '<path d="M8 5v6M5 8h6"/>' +
    '</svg>';

  // 文件夹图标（折叠状态 - 灰色）
  var ICON_FOLDER_CLOSED =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<path d="M2 4h5l2 2h5v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4z" stroke="#6b7280" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>' +
    '</svg>';

  // 文件夹图标（打开状态 - 蓝色）
  var ICON_FOLDER_OPENED =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<path d="M2 4h5l2 2h5v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4z" stroke="#3b6cf6" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>' +
    '<path d="M2 6h12" stroke="#3b6cf6" stroke-width="1.2" stroke-dasharray="2 1"/>' +
    '</svg>';

  // 兼容旧代码
  var ICON_FOLDER = ICON_FOLDER_CLOSED;

  // 文件图标
  var ICON_FILE =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<path d="M4 2h5l4 4v8a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z"/>' +
    '<path d="M9 2v4h4"/>' +
    '</svg>';

  // 预览图标
  var ICON_PREVIEW =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<rect x="2" y="3" width="12" height="10" rx="1"/>' +
    '<path d="M5 7h6M5 9h4"/>' +
    '</svg>';

  // 刷新图标
  var ICON_REFRESH =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<path d="M2 8a6 6 0 0 1 10.3-4.2M14 8a6 6 0 0 1-10.3 4.2"/>' +
    '<path d="M14 2v4h-4M2 14v-4h4"/>' +
    '</svg>';

  // 文件类型图标
  var ICON_FILE_JS =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="#f7df1e"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="7" font-weight="bold" fill="#000">JS</text>' +
    '</svg>';

  var ICON_FILE_TS =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="#3178c6"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="7" font-weight="bold" fill="white">TS</text>' +
    '</svg>';

  var ICON_FILE_PY =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="#3776ab"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="7" font-weight="bold" fill="yellow">PY</text>' +
    '</svg>';

  var ICON_FILE_HTML =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="orange"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="6" font-weight="bold" fill="white">&lt;/&gt;</text>' +
    '</svg>';

  var ICON_FILE_CSS =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="#1572b6"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="6" font-weight="bold" fill="white">{}</text>' +
    '</svg>';

  var ICON_FILE_JSON =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="#5b5b5b"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="6" font-weight="bold" fill="white">{}</text>' +
    '</svg>';

  var ICON_FILE_MD =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">' +
    '<rect x="2" y="2" width="12" height="12" rx="2" fill="#083fa1"/>' +
    '<text x="8" y="11" text-anchor="middle" font-size="7" font-weight="bold" fill="white">M↓</text>' +
    '</svg>';

  var ICON_FILE_IMAGE =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" ' +
    'stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<rect x="2" y="3" width="12" height="10" rx="1"/>' +
    '<circle cx="5.5" cy="6.5" r="1.5"/>' +
    '<path d="M14 10l-3-3-5 5"/>' +
    '</svg>';

  var ICON_FILE_SHELL =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" ' +
    'stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
    '<rect x="2" y="3" width="12" height="10" rx="1"/>' +
    '<path d="M5 7l2 2-2 2M8 10h3"/>' +
    '</svg>';


