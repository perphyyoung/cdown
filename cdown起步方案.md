# cdown 起步方案（Tauri 2 倒计时桌面小组件）

## 1. 定位

- 常驻桌面的小窗口小组件：以表格样式列出倒计时项（名称、目标日期、剩余天数，备注列在剩余右侧）；四列宽度可单独拖拽调整并持久化（`Settings.column_widths`）。
- 支持多级紧急度着色：各级天数阈值与颜色可自定义（默认 7 天紫 / 3 天黄 / 0 天橙（0 天级仅命中今天，突出当天），最多 6 级），剩余天数命中「满足阈值的最紧急级」时按该级颜色显示；过期固定红色弱化。
- **低资源占用**：时间精度到天即可，不做秒级计时；刷新频率压到最低（见 §6），无路由、单窗口、依赖从简。
- 按《tauri2项目起步指南》起步，依赖按需引入（见 §3 取舍表）。

## 2. 技术栈

| 层 | 选择 | 说明 |
| --- | --- | --- |
| 前端 | Vue 3 + TS + Vite 6 + Tailwind 3 | 与指南/paim 一致；表格样式用 Tailwind 最快 |
| 路由 | 不引入 | 单视图应用 |
| 状态 | 组件内 composable | 数据量小，无需 pinia |
| 后端 | Tauri 2 + tauri-specta rc.25 三件套 | 命令签名单一事实源，指南定为必选 |
| 存储 | JSON 文件（`cdown.json`） | 数据量小，不引 rusqlite |
| 测试 | vitest + cargo test | 倒计时/变红纯函数是单测重点 |
| e2e | 二期再定 | 一期先跑通 `pnpm check` |

## 3. 依赖取舍（相对 paim，按需裁剪）

保留（指南「建议统一」项）：

- pnpm pin（`packageManager`）、版本单一事实源（`tauri.conf.json` 的 `version: "../package.json"`）
- Rust 分层 `commands.rs`+`commands/`、`domain.rs`+`domain/`、`infra.rs`+`infra/`（同名文件放子模块声明，**不用 mod.rs**；测试平铺 `<源文件>.test.rs`）、根 `Cargo.toml` workspace + `[profile.release]` 体积优化
- 共享 `CARGO_TARGET_DIR`（沿用 `D:\cargo-shared-target`），脚本一律读环境变量
- specta_builder + 两条导出路径（`CDOWN_EXPORT_BINDINGS` 导出即退 / debug 启动自动导出）+ `scripts/gen-bindings.mjs`
- `removeUnusedCommands` + capabilities 精确清单、生产严格 CSP + `devCsp` 放宽
- vitest / cargo test / oxfmt + cargo fmt / `pnpm check` 质量门

裁掉：

- `rusqlite` → JSON 文件；`vue-router` → 无此需求（`plugin-dialog` 一期为导出/导入引入，见下）
- asset protocol（`protocol-asset` 特性）→ 无本地图片加载，CSP 相应简化
- `tauri-plugin-global-shortcut` → 二期已做：默认 `Ctrl+Alt+C` 切换主窗口（托盘隐藏/最小化时按下唤起，
  窗口正显示在前台时按下收回托盘；长按自动重复已用「松手后才算新一次」过滤），
  键位存 `Settings.hotkey`（`null` = 关闭），设置页可录键修改；注册/注销全在 Rust 侧（`commands/hotkey.rs`），
  capabilities 不需开权限，键被其它程序占用时保存报错且保留旧键
- `tauri-plugin-log` 仅保留 debug 终端输出；文件日志按 paim 引入自研 `infra/logging.rs`（`cdown.log` + `cdown-config.toml` 分级 + `CDOWN_LOG` 环境变量 + 前端 logger，见 日志使用说明.md）
- Playwright e2e 骨架 → 二期（先抄 paim 的 fixture/CDP 模式）

新增：

- `tauri-plugin-single-instance`（最先注册；小组件必须防多开）
- `tauri-plugin-dialog` + `@tauri-apps/plugin-dialog`（设置页全量导出/导入的保存/选文件对话框）
- `@vuepic/vue-datepicker`（日期选择；原生 date 控件显示格式跟随系统区域无法控制，显示与存储统一 `yyyy-MM-dd`）
- 日期日历跑在**独立 `date-picker` 小窗口**里（inline 模式、无时间选择、失焦即关）：HTML 弹层画不出窗口边界，原生 flyout 的「浮在窗口外」体验只能靠独立窗口实现
- `tauri` 的 `tray-icon` 特性（关到托盘、右键退出/唤起；小组件没有任务栏图标，托盘是唯一出口）
- 二期候选：`tauri-plugin-autostart`（开机自启，小组件类应用大概率需要）

## 4. 窗口形态

- 主窗口 label `main`：约 320×420，`decorations: false`，`alwaysOnTop: true`，`skipTaskbar: true`，可缩放（min/max 收窄）。
- 无边框拖动：标题区放 `data-tauri-drag-region`（需 `core:window:allow-start-dragging` 权限）。
- 自绘右上角小按钮：添加（＋）、图钉（置顶开关）、设置（⚙）、隐藏到托盘（−）。
- 主窗口置顶：偏好存 `Settings.always_on_top`（serde 缺省 true，旧数据自动默认置顶）；启动时 setup 读 Settings 应用，前端图钉按钮只改设置、watch 同步窗口实际状态（设置导入后经 `settings-changed` 同样生效），需 `core:window:allow-set-always-on-top` 权限。
- 托盘菜单：显示/隐藏、设置、退出；「设置」与主面板 ⚙ 按钮都走 `open_settings` 命令（Rust 侧创建/唤起独立设置窗口，label `settings`，原生标题栏 + 置顶，方便对照主面板调样式；失败信息回传前端错误条）。同一段前端按**窗口 label** 分流渲染；设置保存后广播 `settings-changed`，主窗口重拉设置。
- 一期不做透明背景（避免 macOS private api 分歧），用圆角 + 阴影即可。

## 5. 数据模型与命令

```rust
// domain/model.rs
struct CountdownItem { id: String, title: String, target_date: String /* "YYYY-MM-DD" */, note: Option<String>, created_at: String }
struct Settings { levels: Vec<UrgencyLevel>, column_widths: ColumnWidths, always_on_top: bool }
```

命令（specta 登记一次，导出 TS 类型）：

- `list_items() -> Vec<CountdownItem>`
- `add_item(title, target_date, note?) -> CountdownItem`
- `update_item(id, title, target_date, note?)` / `delete_item(id)`
- `get_settings() -> Settings` / `set_settings(Settings)`

存储：`infra/store.rs` 读写 `<app_config_dir>/cdown.json`（serde_json，写入用临时文件+rename 防写坏）。目标日期存 `YYYY-MM-DD` 字符串，Rust 侧用 `chrono::NaiveDate` 校验。

主窗口几何（尺寸/位置）由官方 `tauri-plugin-window-state` 插件持久化与启动自动恢复（状态标志排除 VISIBLE——主窗口常隐藏到托盘，可见性不参与持久化；托盘退出前显式保存一次兜底）。几何与显示器绑定，**不参与设置的导入/导出**；dev/release 状态文件分离（release 用插件默认 `.window-state.json`，dev 为 `window-state.dev.json`），避免两种构建共享同一份几何。

## 6. 倒计时与变红逻辑（前端纯函数，vitest 覆盖）

- `src/features/countdown/logic.ts`（以「本地日期字符串 YYYY-MM-DD」为入参，可测、无时钟依赖）：
  - `daysUntil(targetDate, today) -> number`：目标日期 − 今天的天数差。
  - `rowState(days, levels) -> { state: normal | level | expired, color? }`：`days < 0` → expired；命中「满足 `days ≤ 阈值` 的级别中阈值最小（最紧急）的一级」→ level（返回该级颜色）；否则 normal。分级结构 `UrgencyLevel { threshold_days, color }`，后端归一化（去重/降序/上限 6/非法颜色回落）。
  - `formatDays(days) -> string`：`已过期 N 天` / `今天` / `明天` / `N 天`。
- **资源占用口径**：精度到天，天与天的分界在午夜——常规 setInterval 每分钟重算一次已远超需要且开销可忽略（纯字符串日期差，无 DOM 重排）；窗口隐藏（`visibilitychange`）时暂停 tick，恢复可见时立即重算一次。
- 变红/着色：整行统一颜色——`level` 行内联样式采用该级自定义颜色，`expired` 行固定红色弱化，`normal` 行默认色；备注列与其它列同色同字号，不做弱化。
- 排序：非过期行按剩余天数升序，过期行统一放最后。
- 紧急度分级是全局设置（默认 7/3/0 三级），在设置页配置；如需按行覆盖再加 `item.urgency_levels?: UrgencyLevel[]`。

## 7. 目录结构

```dir
cdown/
  package.json  pnpm-workspace.yaml  Cargo.toml        # workspace 根 + release profile
  index.html  vite.config.ts  vitest.config.ts
  tailwind.config.js  postcss.config.js  .oxfmtrc.json
  scripts/gen-bindings.mjs
  src/
    bindings.ts                  # 生成物，入库
    app/App.vue
    features/countdown/
      CountdownTable.vue  EditableRow.vue  SettingsRow.vue  SettingsPage.vue
      useCountdown.ts  logic.ts  logic.test.ts
  src-tauri/
    tauri.conf.json  capabilities/default.json  icons/
    src/                     # 同名 .rs + 同名目录组织子模块，不用 mod.rs
      lib.rs  main.rs
      commands.rs  commands/{items.rs, items.test.rs, settings.rs}
      domain.rs    domain/{model.rs, model.test.rs, error.rs}
      infra.rs     infra/{store.rs, store.test.rs}
```

## 8. 关键配置

- `tauri.conf.json`：`productName: "cdown"`、`identifier: "com.cdown.perphyyoung"`、`version: "../package.json"`、`removeUnusedCommands: true`、bundle targets `["nsis"]`；CSP 同 paim 模式但去掉 `asset:` 相关指令，`devCsp` 放开 ws。
- `capabilities/default.json`（按实际调用逐条开）：
  `core:window:allow-start-dragging`、`core:window:allow-hide`（右上角隐藏按钮；托盘显示/退出由 Rust 侧操作窗口，无需前端权限）。
- 环境变量前缀 `CDOWN_`：`CDOWN_EXPORT_BINDINGS`（导出即退）。数据目录用 `app_config_dir`，一期不需要重定向变量。
- vite：端口 1420、`strictPort`、`@` alias、watch 白名单（index.html + src/ + public/）。

## 9. 实施步骤

1. 脚手架：package.json（pnpm pin + scripts 照指南改）、pnpm-workspace.yaml、根 Cargo.toml、vite/tailwind/oxfmt/vitest 配置。
2. Rust 骨架：lib.rs（specta_builder + 两条导出路径）、domain/infra/commands 分层、JSON store + cargo 单测。
3. tauri.conf.json + capabilities + 图标（`pnpm tauri icon` 生成）。
4. 前端：表格组件、tick、变红逻辑（先写 logic.ts 与单测）、增删改表单、阈值设置行。
5. 集成：single-instance 最先注册、托盘、无边框拖动、隐藏到托盘。
6. `pnpm check` 跑通一次 → `pnpm dev` 手工验收。
7. 二期：Playwright e2e（抄 paim 骨架）、~~全局热键~~（已做，默认 `Ctrl+Alt+C`）、开机自启、透明背景、行内编辑优化。

## 10. 待确认

1. `identifier`（反向域名，如 `com.cdown.perphyyoung` 或个人域名风格）。
2. 是否需要开机自启（影响是否一期就引 `tauri-plugin-autostart`）。
3. 变红阈值做成全局设置即可，还是每行可单独覆盖（一期默认全局）。
