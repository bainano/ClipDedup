# ClipDedup 发布指南

本文件是项目唯一的**发布规范与流程**，适用于所有版本。发布前请完整阅读，发布后按本文件核对。

---

## 1. 版本管理

- 版本号遵循[语义化版本](https://semver.org/lang/zh-CN/) `MAJOR.MINOR.PATCH`（如 `0.0.1`）。
- **唯一版本来源**是 `Cargo.toml` 的 `version` 字段，Git tag 必须写成 `v<版本>` 且与之完全一致。
- 每个 **minor / major 变更**必须新增一份发布说明；同一个 patch 版本内的小改动追加到既有发布说明的「修复」小节，文件 wording 保持不变。
- 仓库中**只保留当前有效的发布版本**：历史版本文档一旦对应 Release 被删除，必须同步删除，避免误导。

### 清理多余版本（异常发布 / 误发版本）

按用户要求保留的版本之外的 Release、tag、发布说明都应清理：

```powershell
# 1) 删除远端 Release（连同其资产）
curl.exe --ssl-no-revoke -X DELETE `
  -H "Authorization: Bearer $tok" `
  https://api.github.com/repos/<owner>/<repo>/releases/<release_id>

# 2) 删除远端 tag
curl.exe --ssl-no-revoke -X DELETE `
  -H "Authorization: Bearer $tok" `
  https://api.github.com/repos/<owner>/<repo>/git/refs/tags/v0.1.0

# 3) 同步删除本地 tag 与仓库内对应的 docs/releases/*.md
git tag -d v0.1.0 ; git push origin --delete v0.1.0
```

> 注意：删除已有 tag 后，同名 tag 再推送不会触发 CI/CD；重新发布请务必确认版本号或 tag 名未与已删除版本冲突。

---

## 2. 构建

| 项目 | 值 |
| --- | --- |
| 语言/工具链 | Rust，`x86_64-pc-windows-msvc` |
| 构建命令 | `cargo build --release` |
| 产物 | `target/release/ClipDedup.exe`（单文件，无需安装） |
| 构建期产物 | `assets/icon.rc` 通过构建脚本 `build.rs` 把图标嵌入 exe |

```powershell
cargo build --release
```

- 发布前必须确认：`cargo build --release` **无 error、无 warning**（新增 warning 要在发布前修掉）。
- ARM64 版本需先安装目标（`rustup target add aarch64-pc-windows-msvc`）并保证 MSVC ARM64 链接工具链可用；未提供该工具链时，发布说明中必须注明「ARM64 未提供预编译包」。

---

## 3. 应用图标要求

图标是本项目对外唯一的视觉标识，**托盘、exe 文件图标、未来安装包必须来自同一套图标**。

| 文件 | 位置 | 必备要求 |
| --- | --- | --- |
| `icon.png` | `assets/icon.png` | 512×512 像素、PNG、RGBA 透明背景；构建期通过 `include_bytes!` 嵌入，作为**托盘图标** |
| `icon.ico` | `assets/icon.ico` | 标准 ICO（推荐内含 256/128/64/48/32/16 多尺寸），作为 **exe 文件图标**与安装器/快捷方式图标 |
| `icon.rc` | `assets/icon.rc` | 资源脚本，引用 `assets/icon.ico`，由 `build.rs` 调用资源编译器嵌入 |

规范要求：

1. **文件名必须保持 `icon.png` / `icon.ico` 且路径固定**（`src/main.rs` 与 `assets/icon.rc` 中硬编码），换图标只替换文件内容，不改文件名。
2. 主体图形居中并留 **8%~10% 内边距**；扁平、单色或低渐变；避免细于 2px 的线条——缩到 16×16 时必须仍可辨识。
3. 背景透明（PNG），不要带圆角底板；保证在深色与浅色托盘背景上都清晰。
4. ICO 与 PNG 必须是**同一视觉内容**；更新 PNG 后必须同步重新导出 ICO，两者不一致视为发布缺陷。
5. 替换图标后必须**重新完整构建**并肉眼确认：exe 在资源管理器中的图标、托盘区图标均已更新。
6. 不要删除 `assets/` 下的图标文件，也不要把临时/旧图标提交到仓库根目录。

---

## 4. 发布资产命名

- **资产文件名只允许 ASCII**（GitHub Release 上传接口会剥离非 ASCII 字符，中文名会被截断）。
- 命名规则：`ClipDedup-<版本>-win-<架构>.exe`，例如：

| 场景 | 文件名 |
| --- | --- |
| Windows 10/11 x64 | `ClipDedup-0.0.1-win-x64.exe` |
| Windows 10/11 ARM64 | `ClipDedup-0.0.1-win-arm64.exe` |

- 每个 Release 上传的资产清单必须与发布说明「下载」表格逐条对应。

---

## 5. 发布步骤

1. **更新版本号**：修改 `Cargo.toml` 的 `version`。
2. **编写发布说明**：新建 `docs/releases/v<版本>.md`，结构严格照 `docs/releases/TEMPLATE.md`；正文即 Release 正文。
3. **自检**：`cargo build --release` 通过；按发布说明「下载」清单准备好资产文件。
4. **提交代码**：`git add -A && git commit -m "v<版本>: <一句话摘要>"`。
5. **打 tag 并推送**：`git tag v<版本>`、`git push origin main`、`git push origin v<版本>`。
6. **创建 GitHub Release**：名称 `ClipDedup v<版本>`，tag 选 `v<版本>`，正文取自步骤 2 的发布说明文件，勾选发布为最新版本。
7. **上传资产**：按第 4 节命名上传 exe。
8. **发版验证**：① 资产可正常下载；② 双击 exe 能启动且**不弹控制台窗口**；③ 托盘菜单「启用去重 / 系统通知 / 开机自启动 / 清除去重历史 / 退出」响应正常；④ 复制 A → 复制 B → 再复制 A 被拦截且弹出系统通知。
9. **收尾**：更新 README 中的功能与版本信息；确认仓库内不存在多余的历史发布说明文件。

---

## 6. 自动化 / 凭证说明

- 本机 GitHub 凭证通过 OAuth 工具 `hg-oauth` 获取：`$tok = (hg-oauth token | Select-String '^gho_').Line`。
- 调用 GitHub REST API 时，PowerShell 发送中文 JSON 必须使用 `-ContentType 'application/json; charset=utf-8'` 并以 UTF-8 字节提交；用 `Get-Content -Raw` 读取 Markdown 后要显式转成 `[string]`，否则会被当成对象序列化导致 422。
- 本机访问 `api.github.com` 的 curl.exe 需要加 `--ssl-no-revoke`。
