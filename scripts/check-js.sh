#!/usr/bin/env bash
# 注入 JS 语法守门：逐个片段 + 完整 IIFE 拼装产物均通过 node --check。
# 片段清单必须与 src/inject_js/{archive,right_sidebar}/mod.rs 的 push_str 顺序一致。
set -euo pipefail
cd "$(dirname "$0")/../src-tauri/src/inject_js"

COMMON=(js/common.js)
ARCHIVE=(js/archive/constants.js js/archive/styles.js js/archive/overrides.js js/archive/targets.js \
         js/archive/confirm.js js/archive/execute.js js/archive/panel.js js/archive/danger.js js/archive/init.js)
SIDEBAR=(js/right_sidebar/constants.js js/right_sidebar/styles.js js/right_sidebar/state.js \
         js/right_sidebar/git.js js/right_sidebar/file_tree.js js/right_sidebar/modal.js \
         js/right_sidebar/markdown.js js/right_sidebar/highlight.js js/right_sidebar/sidebar.js)

command -v node >/dev/null || { echo "需要 node 做语法检查" >&2; exit 1; }

for f in "${COMMON[@]}" "${ARCHIVE[@]}" "${SIDEBAR[@]}"; do
  [ -s "$f" ] || { echo "空或缺失片段: $f" >&2; exit 1; }
  node --check "$f" || { echo "语法错误: $f" >&2; exit 1; }
done

# 与 inject_js::assemble_inject_js() 相同结构的完整产物
{
  printf '(function() {\n'
  printf '  // ===== 共享工具函数 =====\n  '; cat "${COMMON[@]}"
  printf '\n\n  // ===== 归档功能 =====\n  ';   cat "${ARCHIVE[@]}"
  printf '\n\n  // ===== 右侧侧边栏 =====\n  '; cat "${SIDEBAR[@]}"
  printf '\n\n  // ===== 初始化 =====\n  initArchive();\n  initRightSidebar();\n})()\n'
} > /tmp/dsh-bundle-check.js

node --check /tmp/dsh-bundle-check.js && echo "OK: $((${#COMMON[@]} + ${#ARCHIVE[@]} + ${#SIDEBAR[@]})) 个片段 + 完整 bundle 语法全部通过"
