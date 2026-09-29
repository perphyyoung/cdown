/**
 * e2e 共用工具：启动调试二进制 → CDP 连接 → 按窗口 label 取页面 → 背景色/透明度的操作与查询。
 *
 * 约定：spec 里只写场景步骤与断言；「怎么起应用、怎么找窗口、怎么读落盘值」这类样板都下沉到这里。
 * 实例生命周期由 spec 的 beforeAll/afterAll 直接管理（暂不引入 fixture）。
 */
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { execSync, spawn, type ChildProcess } from "node:child_process";
import { chromium, expect, type Browser, type Page } from "@playwright/test";

/// 项目根（e2e/ 的上一级）
const ROOT = path.join(import.meta.dirname, "..");
/// 内嵌前端的页面地址（非 dev 模式的 localhost:1420）；主窗口与设置窗口共用它，只能靠 label 区分
const APP_URL = "http://tauri.localhost";
/// 背景色默认值（与 src-tauri 的 DEFAULT_BACKGROUND_COLOR 一致）
export const DEFAULT_BACKGROUND_COLOR = "#0f172a";

export interface AppHandle {
  child: ChildProcess;
  browser: Browser;
  dataDir: string;
  cdpPort: number;
  env: NodeJS.ProcessEnv;
}

/// 调试二进制路径（`tauri build --debug --no-bundle` 产物，globalSetup 已构建）。
/// 共享 target 目录由 CARGO_TARGET_DIR 指定，不得硬编码。
function exePath(): string {
  const targetDir = process.env.CARGO_TARGET_DIR ?? path.join(ROOT, "target");
  return path.join(targetDir, "debug", "cdown.exe");
}

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const port = (server.address() as net.AddressInfo).port;
      server.close(() => resolve(port));
    });
    server.on("error", reject);
  });
}

/// 轮询连接 CDP，直到应用页面出现；child 用于提前发现进程已崩溃
async function connectAppCdp(
  cdpPort: number,
  child: ChildProcess,
  timeoutMs: number,
): Promise<Browser> {
  const deadline = Date.now() + timeoutMs;
  let lastErr: unknown = new Error("CDP 连接超时");
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`应用进程提前退出 code=${child.exitCode}`);
    try {
      const browser = await chromium.connectOverCDP(`http://127.0.0.1:${cdpPort}`);
      if (
        browser
          .contexts()
          .flatMap((c) => c.pages())
          .some((p) => p.url().startsWith(APP_URL))
      ) {
        return browser;
      }
      // 连上了但窗口还没加载出来：放掉这个连接，下一轮重连
      lastErr = new Error("已连接 CDP 但还没有应用页面");
      await browser.close().catch(() => {});
    } catch (e) {
      lastErr = e;
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw lastErr;
}

/// spawn 一个应用实例并连上 CDP。数据目录隔离到 temp/e2e-<序号>（已 gitignore）
export async function launchApp(seq = 0, logLevel = "error"): Promise<AppHandle> {
  const dataDir = path.join(ROOT, "temp", `e2e-${seq}`);
  const cdpPort = await freePort();
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    CDOWN_DATA_DIR: dataDir,
    // 断言只看 UI 与落盘文件，默认压掉应用日志噪声，避免污染共享的 cdown.log；
    // 排查窗口几何等启动期行为时可显式传 info
    CDOWN_LOG: logLevel,
    WEBVIEW2_USER_DATA_FOLDER: path.join(ROOT, "temp", "wv2-e2e"),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
  };
  const child = spawn(exePath(), [], { cwd: ROOT, env, stdio: "ignore" });
  child.on("error", (e) => console.error(`[app] spawn 失败: ${e.message}`));
  child.on("exit", (code) => console.log(`[app] 进程退出: ${code}`));
  const browser = await connectAppCdp(cdpPort, child, 10_000);
  return { child, browser, dataDir, cdpPort, env };
}

/// 等子进程真正退出：Windows 上 taskkill 返回 ≠ 句柄已释放，紧接着删目录会撞 EBUSY
function waitForExit(child: ChildProcess, timeoutMs: number): Promise<boolean> {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve(true);
  return new Promise<boolean>((resolve) => {
    const timer = setTimeout(() => resolve(false), timeoutMs);
    timer.unref();
    child.once("exit", () => {
      clearTimeout(timer);
      resolve(true);
    });
  });
}

async function closeApp(app: AppHandle): Promise<void> {
  const pid = app.child.pid;
  await app.browser.close().catch(() => {});
  if (pid === undefined) return;
  try {
    execSync(`taskkill /PID ${pid}`, { stdio: "ignore" });
  } catch {
    // 已退出
  }
  if (await waitForExit(app.child, 3_000)) return;
  try {
    execSync(`taskkill /F /T /PID ${pid}`, { stdio: "ignore" });
  } catch {
    // 已优雅退出
  }
  await waitForExit(app.child, 10_000);
}

/// 删目录（best-effort，绝不抛）：失败则改名让位，残留由下一轮 globalSetup 清掉。
/// 清理失败是环境噪声，不该把全绿的用例判失败。
function removeDirBestEffort(dir: string): void {
  try {
    fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
  } catch {
    try {
      fs.renameSync(dir, `${dir}-stale-${Date.now()}`);
    } catch (e) {
      console.warn(`[cleanup] 目录删除与改名均失败：${dir} — ${e}`);
    }
  }
}

export async function disposeApp(app: AppHandle): Promise<void> {
  await closeApp(app);
  await waitForExit(app.child, 15_000);
  removeDirBestEffort(app.dataDir);
}

/// 关掉实例后用同一数据目录重开（验证「重启后设置仍在」）
export async function restartApp(app: AppHandle): Promise<void> {
  await closeApp(app);
  // 端口 TIME_WAIT 与句柄释放留一点时间，Windows 上 taskkill 后句柄释放不是即时的
  await new Promise((r) => setTimeout(r, 1_500));
  const child = spawn(exePath(), [], { cwd: ROOT, env: app.env, stdio: "ignore" });
  child.on("exit", (code) => console.log(`[restart] 进程退出: ${code}`));
  app.child = child;
  app.browser = await connectAppCdp(app.cdpPort, child, 12_000);
}

/// 读页面的窗口 label（TAURI 注入的元数据，不发 IPC）
function pageWindowLabel(page: Page): Promise<string> {
  return page
    .evaluate(() => {
      const meta = (
        window as unknown as {
          __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } };
        }
      ).__TAURI_INTERNALS__?.metadata?.currentWindow?.label;
      return meta ?? "";
    })
    .catch(() => "");
}

/// 按窗口 label 轮询查找页面（窗口创建到页面就绪是异步的）
export async function findPageByWindowLabel(
  browser: Browser,
  label: string,
  timeoutMs: number,
): Promise<Page> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    for (const candidate of browser.contexts().flatMap((c) => c.pages())) {
      if ((await pageWindowLabel(candidate)) === label) return candidate;
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`未找到窗口 label=${label} 的页面`);
}

/// 主窗口页面（等根节点挂上，说明前端已渲染）
export async function mainPage(app: AppHandle): Promise<Page> {
  const page = await findPageByWindowLabel(app.browser, "main", 15_000);
  await expect(page.locator("#app > div")).toBeAttached();
  return page;
}

/// 从主窗口点齿轮打开设置窗口，返回设置窗口页面
export async function openSettingsWindow(app: AppHandle, main: Page): Promise<Page> {
  await main.getByTitle("设置").click();
  const settings = await findPageByWindowLabel(app.browser, "settings", 10_000);
  await expect(settings.locator("dl").first()).toBeVisible();
  return settings;
}

/// 调用 tauri 命令（统一 __TAURI_INTERNALS__ 访问样板）
export function invokeCommand<T>(
  page: Page,
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return page.evaluate(
    ({ cmd, args }) =>
      (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (c: string, a?: unknown) => Promise<T> };
        }
      ).__TAURI_INTERNALS__.invoke(cmd, args),
    { cmd, args },
  );
}

/// ---- 背景色与透明度 ----

/// 设置页最上方的取色器：背景色行固定排在最前，故取第一个 color 输入
/// （紧急度分级行里也有 color 输入，位置在后）
export function colorInput(settings: Page) {
  return settings.locator('input[type="color"]').first();
}

export function undoButton(settings: Page) {
  return settings.getByTitle("撤销本次修改，恢复上一次的颜色");
}

export function resetButton(settings: Page) {
  return settings.getByTitle("恢复默认背景色");
}

/// 模拟取色器里改色：`input` 在拖动过程中持续触发（只预览不落盘）
export async function pickColor(settings: Page, color: string): Promise<void> {
  await colorInput(settings).evaluate((el, c) => {
    const input = el as HTMLInputElement;
    input.value = c;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, color);
}

/// 模拟取色器关闭：`change` 触发，并把取色器里的颜色推回输入框（浏览器行为）
export async function closeColorPicker(settings: Page, color?: string): Promise<void> {
  await colorInput(settings).evaluate((el, c) => {
    const input = el as HTMLInputElement;
    if (c !== null) input.value = c;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, color ?? null);
}

/// 拖一次透明度滑杆（input 即保存）
export async function setOpacity(settings: Page, percent: number): Promise<void> {
  await settings.getByRole("slider").evaluate((el, v) => {
    const input = el as HTMLInputElement;
    input.value = String(v);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, percent);
}

/// 主窗口根节点的背景色，解析成 [r, g, b, a]（读计算样式，避开 rgb/rgba 序列化差异）
export function backgroundRgba(main: Page): Promise<[number, number, number, number]> {
  return main.evaluate(() => {
    const el = document.querySelector<HTMLElement>("#app > div[style]");
    const nums = (getComputedStyle(el!).backgroundColor.match(/[\d.]+/g) ?? []).map(Number);
    return [nums[0], nums[1], nums[2], nums[3] ?? 1] as [number, number, number, number];
  });
}

/// 主窗口视口尺寸（CSS 像素＝逻辑像素）与缩放比：用户肉眼看到的窗口大小
export function mainInnerSize(page: Page): Promise<{ width: number; height: number; dpr: number }> {
  return page.evaluate(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
    dpr: window.devicePixelRatio,
  }));
}

/// 窗口状态文件：插件写在应用配置目录，与 CDOWN_DATA_DIR 重定向无关
export function devWindowStatePath(): string {
  return path.join(process.env.APPDATA ?? "", "com.cdown.perphyyoung", "window-state.dev.json");
}

/// 读某个窗口的持久化几何（插件在退出时写入）
export function readWindowState(label: string): Record<string, unknown> {
  const file = devWindowStatePath();
  if (!fs.existsSync(file)) return {};
  const raw = JSON.parse(fs.readFileSync(file, "utf8")) as Record<string, Record<string, unknown>>;
  return raw[label] ?? {};
}

/// 落盘的设置（cdown.json；文件不存在视为空，便于断言「还没写入」）
export function readPersistedSettings(dataDir: string): Record<string, unknown> {
  const file = path.join(dataDir, "cdown.json");
  if (!fs.existsSync(file)) return {};
  const raw = JSON.parse(fs.readFileSync(file, "utf8")) as Record<string, unknown>;
  const settings = raw.settings;
  return (settings && typeof settings === "object" ? settings : raw) as Record<string, unknown>;
}

/// 等落盘值与期望一致（写文件是异步的）
export function expectPersistedColor(dataDir: string, color: string): Promise<void> {
  return expect
    .poll(() => readPersistedSettings(dataDir).background_color, { timeout: 2_000 })
    .toBe(color);
}

export function expectPersistedOpacity(dataDir: string, percent: number): Promise<void> {
  return expect
    .poll(() => readPersistedSettings(dataDir).background_opacity, { timeout: 2_000 })
    .toBe(percent);
}
