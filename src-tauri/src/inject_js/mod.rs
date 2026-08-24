//! 注入到 dsh webview 的脚本模块
//!
//! 该模块包含所有注入到 dsh webview 的 JavaScript 代码，包括：
//! - `common`: 共享工具函数（RPC、DOM 操作、样式注入等）
//! - `archive`: 归档功能（归档按钮、归档管理器面板）
//! - `right_sidebar`: 右侧侧边栏（Git 改动、文件树、文件预览）
//! - `file_link`: 文件链接拦截（拦截 DSH 工具结果路径按钮与对话「产物」文件按钮，改用自有弹窗展示）

pub mod archive;
pub mod common;
pub mod file_link;
pub mod right_sidebar;

/// 组装所有注入脚本为一个完整的 IIFE
///
/// 各模块的 JS 代码会被拼接成一个立即执行函数，通过 `spawn_injection()` 注入到 webview。
pub fn assemble_inject_js() -> String {
    format!(
        r##"(function() {{
  // ===== 共享工具函数 =====
  {common}

  // ===== 归档功能 =====
  {archive}

  // ===== 右侧侧边栏 =====
  {right_sidebar}

  // ===== 文件链接拦截 =====
  {file_link}

  // ===== 初始化 =====
  initArchive();
  initRightSidebar();
  initFileLinkInterceptor();
}})()"##,
        common = common::JS_MODULE,
        archive = archive::js_module(),
        right_sidebar = right_sidebar::js_module(),
        file_link = file_link::JS_MODULE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 完整注入脚本必须是一个语法自洽的 IIFE 骨架，且两个初始化入口有定义。
    #[test]
    fn assembled_bundle_is_an_iife() {
        let js = assemble_inject_js();
        assert!(js.starts_with("(function() {"));
        assert!(js.ends_with("})()"));
        assert!(js.contains("function initArchive()"));
        assert!(js.contains("function initRightSidebar()"));
    }
}
