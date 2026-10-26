//! 系统通知：拦截到重复复制时弹出提示。
//!
//! 普通（非商店/未安装）桌面程序要让 Windows Toast 真正显示，必须为使用的
//! AppUserModelID 注册一个“带 System.AppUserModel.ID 属性的开始菜单快捷方式”。
//! 注册成功则通知以 ClipDedup 名义显示；若注册失败，则回退到系统自带的
//! POWERSHELL_APP_ID（保证一定显示，但归属名为 Windows PowerShell）。

use notify_rust::{Notification, Timeout};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

/// 通知使用的 AppUserModelID（Windows Toast 归属标识）
#[cfg(windows)]
const APP_ID: &str = "ClipDedup";

/// 系统自带的 PowerShell AUMID：未安装程序回退使用，确保 Toast 一定能显示
#[cfg(windows)]
const POWERSHELL_APP_ID: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}";

/// 内嵌的 PowerShell 脚本：用 C#/COM 创建带 AppUserModel.ID 属性的 .lnk
const REGISTER_PS1: &str = include_str!("../res/register_aumid.ps1");

/// 标记 AUMID 是否已成功注册（决定用自定义标识还是回退标识）
static AUMID_OK: AtomicBool = AtomicBool::new(false);

/// 为当前进程注册 AUMID，使 Toast 能正常显示。
/// 必须在首次 show 之前（建议在 main 最早期）调用；内部用 Once 保证只执行一次。
pub fn register_aumid() {
    #[cfg(windows)]
    {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            // 1) 在 HKCU 注册该 AUMID（普通桌面程序显示 Toast 的必要身份）
            let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            if let Ok((key, _)) =
                hkcu.create_subkey(r"Software\Classes\AppUserModel\Applications\ClipDedup")
            {
                let _ = key.set_value("DisplayName", &"ClipDedup");
            }
            // 2) 把当前进程的 AUMID 设为同一标识
            let wide: Vec<u16> = "ClipDedup\0".encode_utf16().collect();
            unsafe {
                windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(
                    wide.as_ptr(),
                );
            }
            // 3) 创建“开始菜单程序”里带 AppUserModel.ID 属性的快捷方式（关键一步）
            if let Ok(exe) = std::env::current_exe() {
                AUMID_OK.store(create_aumid_shortcut(&exe), Ordering::Relaxed);
            }
        });
    }
}

/// 创建开始菜单“程序”文件夹中的快捷方式并写入 AppUserModel.ID 属性。
/// 返回是否成功（或该快捷方式已存在）。
#[cfg(windows)]
fn create_aumid_shortcut(exe: &std::path::Path) -> bool {
    let appdata = match std::env::var("APPDATA") {
        Ok(v) => v,
        Err(_) => return false,
    };
    let lnk: PathBuf = PathBuf::from(&appdata)
        .join("Microsoft\\Windows\\Start Menu\\Programs\\ClipDedup.lnk");
    if lnk.exists() {
        return true; // 已经注册过
    }
    if let Some(parent) = lnk.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = std::env::temp_dir().join("clipdedup_register_aumid.ps1");
    if std::fs::write(&tmp, REGISTER_PS1).is_err() {
        return false;
    }
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &tmp.to_string_lossy(),
            "-lnkPath",
            &lnk.to_string_lossy(),
            "-targetPath",
            &exe.to_string_lossy(),
            "-aumid",
            APP_ID,
        ])
        .output();
    let _ = std::fs::remove_file(&tmp);
    match out {
        Ok(o) => o.status.success() && lnk.exists(),
        Err(_) => false,
    }
}

/// 仅在 Windows 下弹出通知
#[cfg(not(windows))]
pub fn show_duplicate_intercepted(text: &str) {
    let _ = text;
}

#[cfg(windows)]
pub fn show_duplicate_intercepted(text: &str) {
    // 双重保险：即使调用方没在启动时注册，这里也会注册一次
    register_aumid();
    let app_id = if AUMID_OK.load(Ordering::Relaxed) {
        APP_ID
    } else {
        POWERSHELL_APP_ID
    };
    let body = preview_string(text);
    let result = Notification::new()
        .app_id(app_id)
        .appname("ClipDedup")
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
