# cdown

Tauri 2 倒计时桌面小组件：表格样式列出倒计时项，剩余天数按可自定义的紧急度分级着色（默认 7 天紫 / 3 天黄 / 1 天橙），过期标红。
时间精度到天（不做秒级计时），资源占用优先。方案与取舍见 `cdown起步方案.md`，
工程约定来自 `D:\py-code\paim\tauri2项目起步指南.md`（建议先读速查表）。

## 项目规则

- 修改代码后，**先**执行 `pnpm check` 验证（format → build:rs → gen:bindings → typecheck → build），通过后再按需跑 `pnpm test`（全部单元测试，含前后端）/ `sentrux check .` / `pnpm e2e`；验证通过才输出**单独一行**的简要的一句话 git commit 信息，方便复制。不要跳过 `pnpm check` 直接跑其它命令

## 命令约定

- 命令一律 **PowerShell 7** 写，串联用 `&&` / `||`。
- 命令输出需要截断时**一律 `tail -100`**，不得用其它行数。
- 质量门唯一入口：`pnpm check` = format → build:rs → gen:bindings → typecheck → build。
- 测试：`pnpm test:ui`（vitest，纯逻辑）+ `pnpm test:rs`（cargo test）。
- `CARGO_TARGET_DIR` 指向共享目录 `D:\cargo-shared-target`（机器级环境变量），
  所有指向构建产物的脚本必须读该环境变量、不得硬编码；共享 target 是全局一把锁，
  与其它 tauri 项目不能并行 build。

## 生成物

- `src/bindings.ts`：tauri-specta 运行期导出（rc.25），不手改、不入格式化。
  Rust 命令签名变更后：`pnpm build:rs && pnpm gen:bindings`，或直接 `pnpm check` / `pnpm dev`。
- `src-tauri/gen/schemas`：生成物，已在 `src-tauri/.gitignore`。

## 环境变量

| 变量 | 作用 | 生效条件 |
| --- | --- | --- |
| `CDOWN_EXPORT_BINDINGS` | 导出即退，供 `pnpm check` 复写 `src/bindings.ts` | debug |

## 结构速记

- Rust 分层即目录名：`commands.rs`+`commands/`（命令）→ `domain.rs`+`domain/`（模型/错误）→
  `infra.rs`+`infra/`（JSON 存储），同名文件放子模块声明，**不用 mod.rs**；
  测试平铺为 `<源文件>.test.rs`（源文件末尾 `#[cfg(test)] #[path]` 声明）；
  入口 `lib.rs` 的 `run()`。
- 前端：`src/features/countdown/`（业务切片）+ `src/app/App.vue`；`logic.ts` 纯函数 + `logic.test.ts` 单测。
- 窗口：无边框、置顶、不进任务栏；关闭/隐藏到托盘，退出走托盘菜单。
- 存储：`<app_config_dir>/cdown.json`（临时文件 + rename 原子写）。
