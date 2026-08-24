// 右侧侧边栏注入脚本：按职责拆分为多个原始串片段，运行时拼接为完整 JS_MODULE。
pub mod constants;
pub mod styles;
pub mod state;
pub mod git;
pub mod file_tree;
pub mod modal;
pub mod markdown;
pub mod highlight;
pub mod sidebar;

// 聚合所有片段为完整注入脚本（保持与单一 JS_MODULE 等价）
pub fn js_module() -> String {
    let mut s = String::with_capacity(32768);
    s.push_str(constants::JS);
    s.push_str(styles::JS);
    s.push_str(state::JS);
    s.push_str(git::JS);
    s.push_str(file_tree::JS);
    s.push_str(modal::JS);
    s.push_str(markdown::JS);
    s.push_str(highlight::JS);
    s.push_str(sidebar::JS);
    s
}
