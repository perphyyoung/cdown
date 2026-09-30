# lessons.md

踩坑记录与能力边界。每条含：现象 → 结论（带依据）→ 本项目的做法。

## 2026-10-01 系统托盘（tauri 2.12 / muda 0.20 / tray-icon 0.25，Windows）

### 1. dev 下应用内 `app.restart()` 会连带杀死 vite，禁止在 debug 构建暴露

- **现象**：托盘「重启」后，新进程主窗口正常，但运行中再打开设置窗口白屏（ERR_CONNECTION_REFUSED）。
- **机制**（日志 + tauri 源码双证）：
  1. 托盘菜单回调在**主线程**，`AppHandle::restart()` 主线程分支 = spawn 新 exe + `exit(0)`（tauri 2.12 `app.rs`）；
  2. 新 exe 是孤儿进程；`tauri dev` CLI 才是 vite（beforeDevCommand）与 app 的父进程；
  3. CLI 监控线程见 app 子进程退出码 0 → `ExitReason::NormalExit` → 杀 vite 并让 CLI 退场（tauri-cli `dev.rs` 的 `on_app_exit` / `interface/rust/desktop.rs`）；
  4. 主窗口在 app 启动瞬间（vite 被杀前约 1 秒）已加载完，所以看着正常；此后新建/刷新窗口全部连不上 localhost:1420。
- **试过无效**：`app.cleanup_before_exit()` + `request_restart()`（退出码 `i32::MAX`）也救不了——非 0/101/手动 kill 时 CLI 同样归入 NormalExit。CLI 只对**源码 watcher 变更**做「保 vite 重编译重启」，没有给应用自重启留协议。
- **与窗口数量无关**：单/多窗口同此机制，区别只是页面在「死前 1 秒」还是「死后」加载。
- **release 正常**：前端走 tauri 内嵌协议，无 dev server；且 2.4+ 的 `restart()` 等旧实例退出事件循环后再拉起，single-instance 锁不冲突。
- **本项目做法**：托盘菜单按 `cfg!(debug_assertions)` 构建——dev 只有「显示 / 设置 / 退出」；release 才插入「重启」。dev 下曾试过「重新加载界面」（遍历窗口 `webview.reload()`），但会重复弹出日期选择器且价值低，已删除。dev 想冷启动 Rust 侧：终端重跑 `pnpm dev`；Rust 代码改动保存后 watcher 会自动重编译重启。

### 2. 托盘右键菜单是 Win32 原生菜单，宽度/外观不可自定义（官方能力边界）

- 菜单由 muda 经 `AppendMenuW`/`InsertMenuW` 构建为原生 HMENU，托盘点击时 tray-icon 直接调 `TrackPopupMenu` 弹出；**菜单宽度由 Windows 按最长项文本 + 系统 MenuFont 自行测量，Tauri 2 / muda 没有任何设置宽度、内边距、字号的 API**。
- muda 在 Windows 的 owner-draw 子类化只用于**窗口菜单栏**的暗黑模式（`WM_UAHDRAWMENUITEM`），托盘弹出菜单不经过它。
- 要做「自定义样式菜单」只有一条路：不用原生菜单，自己建无边框透明窗口模拟菜单（可用托盘图标 rect 定位，positioner 插件思路），代价是失去原生菜单的焦点/点击外部关闭/键盘导航行为。本项目不做。
- **未定位的异常**：某次 debug 构建下出现过菜单异常宽（约 1300px、文字像居中），但系统侧指标正常（1920×1080、96 DPI、MenuFont 雅黑 UI 12pt、WindowMetrics 默认），原生 `CreatePopupMenu`+三项两字中文实测基准为 **122×72px**；进程 DPI 感知为 per-monitor(2)；muda 未给菜单项设位图/快捷键列。根因未复现确认，若再现，先枚举 `#32768` 弹出窗口的真实 `GetWindowRect` 对比 122px 基准，再做判断，不要先改代码。

### 3. 托盘路径的日志必须用自研宏，不写控制台

- `log::info!`/`log::warn!`（log crate）在 debug 构建只进 tauri-plugin-log 的终端输出，**不写 cdown.log**。
- 托盘回调等用户操作路径要落文件，必须用 `infra/logging.rs` 的 `log_info!`/`log_warn!`（见日志使用说明.md）。

### 4. 托盘菜单无法 e2e

- 托盘图标与原生菜单不经过 webview，Playwright/CDP 触达不了；托盘相关改动（显示/设置/重启/退出、左键 toggle）只跑 `pnpm check` + 单测，**手动验证**，不写 e2e。
