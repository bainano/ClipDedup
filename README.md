# ClipDedup

Windows 剪贴板完全去重托盘小工具（Rust）。

## 功能

- **持续监听剪贴板**：通过轮询系统剪贴板序号（`GetClipboardSequenceNumber`）感知每一次复制动作。
- **完全去重**：新复制的内容只要与最近 **10 条**复制历史中**任意一条**重复（中间复制过别的内容同样会被拦），立即清空剪贴板并给出提示。
- **系统通知**：拦截到重复复制时弹出系统通知「已帮你拦截一条重复复制」，可在菜单中开关。
- **系统托盘驻留**：无图形界面、无控制台窗口，仅托盘图标 + 右键菜单。
- **托盘右键菜单**：
  - 启用去重（勾选开关，暂停/恢复拦截）
  - 系统通知（勾选开关）
  - 清除去重历史
  - 开机自启动（写入/删除 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`）
  - 退出

## 使用

双击 `ClipDedup-<版本>-win-x64.exe` 即可，程序在系统托盘中驻留，不弹出终端窗口。右键托盘图标选择菜单项即可操作。

> 说明：去重只针对文本内容；图片、文件等非文本复制不做比对，直接放行。

## 从源码构建

需要 Rust 1.75+（msvc 工具链）：

```powershell
cargo build --release
# 产物：target/release/ClipDedup.exe
```

`build.rs` 会在构建时把 `assets/icon.ico` 嵌入为 exe 图标（需要 Windows SDK 的资源编译器 `rc.exe`；缺失时会给出 warning 但构建继续）。

## 图标

托盘图标取自 `assets/icon.png`，exe 文件图标取自 `assets/icon.ico`。两个文件的命名、路径与规格要求是发布规范的一部分，详见 [docs/RELEASE_GUIDE.md](docs/RELEASE_GUIDE.md) 第 3 节。

## 工作原理

1. 后台线程以 120ms 间隔轮询剪贴板序号，序号变化即视为发生了一次复制。
2. 读取剪贴板文本，与最近 10 条已放行内容比对：
   - 历史中不存在 → 放行并记入历史（FIFO，超容量淘汰最旧一条）；
   - 已存在 → 清空剪贴板（本次复制不生效）并按开关弹出系统通知。
3. 主线程使用 `winit` 事件循环驱动 `tray-icon` 托盘图标与菜单，菜单项状态与注册表自启动配置实时同步。

## 目录结构

```
src/main.rs        托盘、菜单、事件循环与剪贴板监视线程
src/autostart.rs   开机自启动（注册表）读写
src/notify.rs      拦截时的系统通知
assets/icon.png    托盘图标
assets/icon.ico    exe 文件图标（构建时嵌入）
docs/              发布指南与版本发布说明
```

## 发布

发布流程、资产命名与图标规范见 [docs/RELEASE_GUIDE.md](docs/RELEASE_GUIDE.md)。

## License

MIT
