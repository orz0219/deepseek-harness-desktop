// 归档功能注入脚本：按职责拆分为多个原始串片段，运行时拼接为完整 JS_MODULE。
pub mod constants;
pub mod styles;
pub mod overrides;
pub mod targets;
pub mod confirm;
pub mod execute;
pub mod panel;
pub mod danger;
pub mod init;

// 聚合所有片段为完整注入脚本（保持与单一 JS_MODULE 等价）
pub fn js_module() -> String {
    let mut s = String::with_capacity(32768);
    s.push_str(constants::JS);
    s.push_str(styles::JS);
    s.push_str(overrides::JS);
    s.push_str(targets::JS);
    s.push_str(confirm::JS);
    s.push_str(execute::JS);
    s.push_str(panel::JS);
    s.push_str(danger::JS);
    s.push_str(init::JS);
    s
}
