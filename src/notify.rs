//! 系统通知：拦截到重复复制时弹出提示。

use notify_rust::{Notification, Timeout};

/// 通知使用的 AppUserModelID（Windows Toast 归属标识）
#[cfg(windows)]
const APP_ID: &str = "ClipDedup";

/// 仅在 Windows 下弹出通知
#[cfg(not(windows))]
pub fn show_duplicate_intercepted(text: &str) {
    let _ = text;
}

#[cfg(windows)]
pub fn show_duplicate_intercepted(text: &str) {
    let body = preview_string(text);
    let result = Notification::new()
        .app_id(APP_ID)
        .appname(APP_ID)
        .summary("已帮你拦截一条重复复制")
        .body(&body)
        .timeout(Timeout::Milliseconds(4000))
        .show();

    match result {
        Ok(_) => {}
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("[clipdedup] 通知发送失败: {_e:?}");
        }
    }
}

/// 通知正文里的内容预览（换行折叠、超长截断）
fn preview_string(text: &str) -> String {
    const MAX: usize = 40;
    let one_line: String = text.chars().filter(|c| !matches!(c, '\r' | '\n')).collect();
    match one_line.chars().count() {
        n if n > MAX => format!("{}…", one_line.chars().take(MAX).collect::<String>()),
        _ => one_line,
    }
}
