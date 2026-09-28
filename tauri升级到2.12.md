# Tauri 升级到 2.12（cdown 已完成 / paim 执行清单）

## 背景

- cdown 因 `tauri-plugin-autostart` 的 npm 包（2.6.0）与 Rust crate（2.5.1）版本不匹配警告触发升级，
  crate 升到 2.6.0 连带把 tauri 从 2.11.6 带到 **2.12.0**（wry 0.57、windows 0.62、tao 0.37 同批前移）。
- 升级后 tauri-build 2.7 废弃了 `STATIC_VCRUNTIME` 环境变量通道，已按官方迁移到
  `tauri.conf.json → build.windows.staticVCRuntime: true`（默认值即 true，行为不变）。
- npm 侧对齐：`@tauri-apps/api` 2.12.0、`@tauri-apps/cli` 2.12.0（CLI 必须与 tauri 主版本一致）。
- 共享 target（`D:\cargo-shared-target`）现在缓存两套 tauri 栈（cdown 的 2.12 系 + paim 的旧版系），
  哈希按版本共存、互不覆盖，paim 不升级也能正常增量构建，代价只是磁盘。

## paim 升级步骤（建议顺序）

1. **升级 Rust 侧**（paim 根目录执行）：

   ```bash
   cargo update -p tauri --precise 2.12.0
   ```

   tauri 前移会连带解析 wry/tao/windows 等一串依赖，属正常现象；若 paim 也用
   `tauri-plugin-*`，同样方式升到与 npm 包匹配的 minor 版本。也可直接 `cargo update` 全量浮到最新兼容版。

2. **升级 npm 侧并对齐**：

   ```bash
   pnpm add "@tauri-apps/api@~2.12.0"
   pnpm add -D "@tauri-apps/cli@~2.12.0"
   ```

   然后 `pnpm dev` 跑一次：CLI 会在启动时列出所有 npm 包 ↔ Rust crate 的版本不匹配，
   按报错逐个 `pnpm add @tauri-apps/plugin-xxx@~<crate版本>` 对齐即可（同一插件的两侧版本保持同 minor）。

3. **STATIC_VCRUNTIME 迁移**（若 paim 之前依赖该环境变量）：
   tauri-build ≥2.7 会打废弃警告。改在 `tauri.conf.json` 里显式声明：

   ```json
   "build": { "windows": { "staticVCRuntime": true } }
   ```

   该字段默认就是 `true`（静态链接 VC 运行时），显式写出只为固定意图。

4. **验证**（与 cdown 相同的三层）：

   - `pnpm check` 全绿（Rust 新栈编译 + 前端构建）；
   - `pnpm test` 全部单测；
   - `pnpm dev` 冒烟：启动输出无任何版本警告 / 废弃警告，应用能起；
   - 人工点一遍窗口栈相关路径：拖拽、置顶、多窗口、托盘、全局热键——tao/wry 在这次升级里有实质变化。

5. **删除共享 target 并重建**：确认 paim 升级完成后（顺序很重要：先升级再删，
   否则重建会先生成一套旧版缓存、升级后又全量重编一遍）：

   - 关闭所有正在运行/构建的 tauri 应用与终端；
   - 删除 `D:\cargo-shared-target` 整个目录；
   - 两个项目各跑一次 `pnpm check`：首次全量编译约 5–15 分钟/项目，之后两项目共享同一套 2.12 缓存。

## 注意事项

- **pnpm 不要动**：两项目均 pin 12.4.2；`pnpm add` 时提示的 12.6.0 更新忽略。
- **specta 三件套不动**：tauri-specta / specta =2.0.0-rc.25、specta-typescript =0.0.12 为精确锁版，
  与 tauri 2.12 兼容，无需调整。
- 2.12 的 CLI 在构建时会多打印一串 `Removed unused commands from xxx`——这是
  `build.removeUnusedCommands: true` 的正常代码生成输出，不是警告。
- 共享 target 是全局一把锁：任意时刻只能有一个项目在构建（既有约定，与本次升级无关）。
- cdown 的升级明细见其 `Cargo.lock` / `package.json` 当前状态：
  tauri 2.12.0、wry 0.57.0、windows 0.62.2、tauri-plugin-autostart 2.6.0（双侧）、
  @tauri-apps/api 2.12.0、@tauri-apps/cli 2.12.0。
