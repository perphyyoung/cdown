# cdown

**Countdown Desktop Widget** — 倒计时桌面小组件：表格样式列出倒计时项，剩余天数按可自定义的紧急度分级着色（默认 0 天橙 / 3 天黄 / 7 天紫，最多 6 级），过期红色删除线并置底。

本文件是**使用与上手文档**：这是什么、怎么跑起来、怎么用。方案与依赖取舍见 [cdown起步方案.md](cdown起步方案.md)；UI / 交互硬约定见 [design.md](design.md)；日志的文件位置、级别开关与打点方式见 [日志使用说明.md](日志使用说明.md)。

## 技术栈

- **桌面框架**：Tauri 2
- **前端**：Vue 3 + TypeScript + Tailwind CSS + Vite
- **存储**：JSON 文件（`<app_config_dir>/cdown.json`，临时文件 + rename 原子写）

## 功能

- **表格展示**：倒计时（今天 / 明天 / N 天 / 已过期 N 天）、目标日期、名称、备注四列；非过期行按剩余天数升序，过期行置底；列宽可在表头拖拽调整并持久化。
- **紧急度分级**：剩余天数命中「满足阈值的最紧急级」时整行按该级颜色显示（天数小的优先，含当天）；各级天数与颜色可自定义，最多 6 级；过期行固定红色删除线，清空分级则全部正常色。
- **行内编辑**：右键条目 → 「编辑」直接在行内修改（回车保存、Esc 取消、失焦自动保存）；「删除」走确认对话框。新增通过标题栏「＋」，草稿行固定在最后。
- **日期选择**：编辑行点击日期框弹出**独立日历窗口**（无边框置顶，真正浮于主窗口外），显示与存储统一 `yyyy-MM-dd`，含「今天」快捷定位。
- **设置窗口**：标题栏 ⚙ 或托盘菜单打开（独立置顶窗口，便于对照主面板）；紧急度分级的编辑/删除/重置，倒计时与设置各自独立的导出/导入（JSON，替换语义，均需确认）。
- **全局热键**：默认 `Ctrl+Alt+C`，应用在后台或最小化（含托盘隐藏）时按下即唤起主窗口，窗口正显示在前台时再按则收回托盘；键位可在设置页录制修改，需含 Ctrl/Alt/Shift 至少一个，被其它程序占用时保存会报错并保留旧键，也可关闭。
- **托盘常驻**：无边框、置顶、不进任务栏；标题栏「—」或托盘左键隐藏到托盘，托盘右键「显示 / 设置 / 退出」；单实例，二次启动唤起已有窗口。
- **窗口记忆**：主窗口尺寸与位置在重启后恢复（官方 window-state 插件，与显示器绑定，不参与设置的导入/导出）。

## 快速开始

前置要求：Rust、Node（pnpm）、Windows WebView2。

```bash
# 安装前端依赖
pnpm install

# 开发模式（启动 Vite + Tauri 窗口）
pnpm dev

# 构建（等效 tauri build，NSIS 安装包）
pnpm release
```

## 数据与存储

- 倒计时与设置存于 `<app_config_dir>/cdown.json`（`%APPDATA%\com.cdown.widget`），写入用临时文件 + rename 原子替换，损坏时自动备份为 `.json.bak` 并回落默认值。
- 主窗口尺寸/位置由官方 `tauri-plugin-window-state` 存于应用配置目录，与显示器绑定。
- 时间精度到天（天界在午夜）：每分钟重算一次，窗口隐藏时暂停，不做秒级计时——低资源占用。

## 开发环境

当前**只在 Windows 上开发与验证**（Tauri 2 + WebView2）。macOS / Linux 未测试——不是「不支持」，而是没有验证过。

工具链：

- **Windows + WebView2**（随 Edge 安装，通常已具备）；
- **Rust**：最低版本见 `src-tauri/Cargo.toml` 的 `rust-version`；
- **pnpm 12**：版本已固定在 `package.json` 的 `packageManager`，请勿用其它大版本安装依赖（会改写 `pnpm-lock.yaml`）；
- **`CARGO_TARGET_DIR`**：指向共享目录，与其它 Tauri 项目共用编译产物；共享 target 是全局一把锁，多项目不能并行 build。

## 常用命令

质量门唯一入口是 `pnpm check`（format → build:rs → gen:bindings → typecheck → build）；改完代码先跑它，通过后再按需跑测试。

| 命令 | 内容 |
| --- | --- |
| `pnpm dev` | 开发模式（Vite + Tauri，debug 构建自动导出 `src/bindings.ts`） |
| `pnpm check` | 质量门：format → build:rs → gen:bindings → typecheck → build |
| `pnpm test` | 全部单元测试（vitest 纯逻辑 + cargo test） |
| `pnpm build:rs` / `pnpm gen:bindings` | 手动重编 Rust / 复写 `src/bindings.ts` |
| `pnpm release` | 构建安装包（NSIS） |

`src/bindings.ts` 是 tauri-specta 运行期导出的生成物：不手改、不入格式化，Rust 命令签名变更后由 `pnpm check` / `pnpm dev` 自动复写。

## 许可

GPL-3.0。
