//! ClipDedup —— Windows 剪贴板去重托盘小工具。
//!
//! 后台线程轮询剪贴板序号感知复制动作，重复内容立即清空剪贴板；
//! 主线程用 winit 事件循环驱动 tray-icon 托盘与菜单。

#[cfg(not(windows))]
compile_error!("ClipDedup 目前仅支持 Windows。");

mod autostart;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use arboard::Clipboard;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");
const POLL_INTERVAL: Duration = Duration::from_millis(120);
const TOOLTIP_ENABLED: &str = "ClipDedup · 剪贴板去重运行中";
const TOOLTIP_PAUSED: &str = "ClipDedup · 已暂停去重";

#[derive(Debug)]
enum UiMsg {
    /// 拦截了一次重复复制，携带累计拦截次数
    Intercepted(u64),
    /// 放行了一次新内容
    Accepted,
    /// 无法访问剪贴板
    ClipboardUnavailable,
}

/// 主线程与剪贴板监视线程之间的共享状态
struct Shared {
    /// 去重是否启用
    enabled: AtomicBool,
    /// 请求监视线程把「当前剪贴板内容」设为比对基线（重新启用时使用）
    rebase: AtomicBool,
    intercept_count: AtomicU64,
}

struct App {
    shared: Arc<Shared>,
    ui_rx: mpsc::Receiver<UiMsg>,
    tray: TrayIcon,
    check_dedup: CheckMenuItem,
    check_autostart: CheckMenuItem,
    id_dedup: MenuId,
    id_autostart: MenuId,
    id_quit: MenuId,
}

impl App {
    fn set_tooltip(&self, text: &str) {
        let _ = self.tray.set_tooltip(Some(text));
    }

    fn handle_menu(&mut self, id: &MenuId, el: &ActiveEventLoop) {
        if *id == self.id_quit {
            el.exit();
        } else if *id == self.id_dedup {
            let on = self.check_dedup.is_checked();
            self.shared.enabled.store(on, Ordering::Relaxed);
            if on {
                // 重新启用时以当前剪贴板内容为新基线，避免误拦
                self.shared.rebase.store(true, Ordering::Relaxed);
                self.set_tooltip(TOOLTIP_ENABLED);
            } else {
                self.set_tooltip(TOOLTIP_PAUSED);
            }
        } else if *id == self.id_autostart {
            let on = self.check_autostart.is_checked();
            match autostart::set_enabled(on) {
                Ok(()) => {}
                Err(e) => {
                    // 写注册表失败则回滚勾选状态
                    self.check_autostart.set_checked(!on);
                    self.set_tooltip(&format!("ClipDedup · 修改自启动失败: {e}"));
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _el: &ActiveEventLoop) {}

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            self.handle_menu(&ev.id, el);
        }
        while let Ok(msg) = self.ui_rx.try_recv() {
            match msg {
                UiMsg::Intercepted(n) => {
                    self.set_tooltip(&format!("ClipDedup · 已拦截 {n} 次重复复制"))
                }
                UiMsg::Accepted => self.set_tooltip(TOOLTIP_ENABLED),
                UiMsg::ClipboardUnavailable => self.set_tooltip("ClipDedup · 无法访问剪贴板"),
            }
        }
        el.set_control_flow(ControlFlow::Poll);
    }
}

fn read_text(clip: &mut Clipboard) -> Option<String> {
    for _ in 0..5 {
        match clip.get_text() {
            Ok(t) => return Some(t),
            // 剪贴板可能正被其他进程短暂占用，稍候重试
            Err(_) => std::thread::sleep(Duration::from_millis(60)),
        }
    }
    None
}

fn spawn_clipboard_monitor(shared: Arc<Shared>, tx: mpsc::Sender<UiMsg>) {
    std::thread::spawn(move || {
        let mut clip = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                tx.send(UiMsg::ClipboardUnavailable).ok();
                eprintln!("无法打开剪贴板: {e:?}");
                return;
            }
        };
        // 上一条已放行的内容（比对基线）
        let mut last: Option<String> = None;
        let mut enabled = shared.enabled.load(Ordering::Relaxed);
        let mut seq = unsafe { GetClipboardSequenceNumber() };

        loop {
            std::thread::sleep(POLL_INTERVAL);
            #[cfg(debug_assertions)]
            eprintln!("[clipdedup] tick, seq={seq}, last={last:?}");

            // 同步启停开关
            let now_enabled = shared.enabled.load(Ordering::Relaxed);
            if now_enabled != enabled {
                enabled = now_enabled;
                if enabled {
                    shared.rebase.store(true, Ordering::Relaxed);
                } else {
                    last = None;
                }
            }

            // 以当前剪贴板内容为基线，不计为一次复制
            if shared.rebase.swap(false, Ordering::Relaxed) {
                last = read_text(&mut clip);
                seq = unsafe { GetClipboardSequenceNumber() };
                continue;
            }

            if !enabled {
                continue;
            }

            let new_seq = unsafe { GetClipboardSequenceNumber() };
            if new_seq == seq {
                continue;
            }

            let text = read_text(&mut clip);
            // 读取（打开剪贴板）本身也会推进序号，刷新基线避免自触发
            seq = unsafe { GetClipboardSequenceNumber() };

            match text {
                Some(t) if last.as_deref() == Some(t.as_str()) => {
                    #[cfg(debug_assertions)]
                    eprintln!("[clipdedup] duplicate detected, clearing...");
                    // 与上一条重复：清空剪贴板，使本次复制不生效
                    match clip.clear() {
                        Ok(()) => {
                            seq = unsafe { GetClipboardSequenceNumber() };
                            let n = shared.intercept_count.fetch_add(1, Ordering::Relaxed) + 1;
                            tx.send(UiMsg::Intercepted(n)).ok();
                        }
                        Err(e) => {
                            #[cfg(debug_assertions)]
                            eprintln!("[clipdedup] clear failed: {e:?}");
                        }
                    }
                }
                Some(t) => {
                    last = Some(t);
                    tx.send(UiMsg::Accepted).ok();
                }
                // 非文本内容（图片/文件等）或空内容：放行，不做比对
                None => last = None,
            }
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, ui_rx) = mpsc::channel();
    let shared = Arc::new(Shared {
        enabled: AtomicBool::new(true),
        rebase: AtomicBool::new(false),
        intercept_count: AtomicU64::new(0),
    });
    spawn_clipboard_monitor(shared.clone(), tx);

    let event_loop = EventLoop::new()?;

    let menu = Menu::new();
    let check_dedup = CheckMenuItem::new("启用去重", true, true, None);
    let check_autostart =
        CheckMenuItem::new("开机自启动", true, autostart::is_enabled(), None);
    let quit = MenuItem::new("退出", true, None);
    let sep = PredefinedMenuItem::separator();
    menu.append(&check_dedup)?;
    menu.append(&check_autostart)?;
    menu.append(&sep)?;
    menu.append(&quit)?;

    let img = image::load_from_memory(LOGO_BYTES)?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let icon = Icon::from_rgba(rgba.into_raw(), w, h)?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(TOOLTIP_ENABLED)
        .with_icon(icon)
        .build()?;

    let mut app = App {
        shared,
        ui_rx,
        tray,
        id_dedup: check_dedup.id().clone(),
        id_autostart: check_autostart.id().clone(),
        id_quit: quit.id().clone(),
        check_dedup,
        check_autostart,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
