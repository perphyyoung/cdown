/**
 * 复现「重启后主界面高度变高」：每轮重启都记录视口高度（用户所见）与插件落盘的几何。
 * 注意：窗口几何由插件写在应用配置目录，与 CDOWN_DATA_DIR 无关，本 spec 会改写本机
 * `window-state.dev.json`（即 pnpm dev 用的那份几何）。
 */
import { expect, type Page } from "@playwright/test";
import {
  disposeApp,
  launchApp,
  mainInnerSize,
  mainPage,
  readWindowState,
  restartApp,
  test,
  type AppHandle,
} from "./e2e-helpers";
import { e2eLog } from "./e2e-logger";

test.describe("主窗口几何", () => {
  let app: AppHandle;
  let main: Page;

  test.beforeAll(async () => {
    app = await launchApp(0);
    main = await mainPage(app);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  test("连续重启后主窗口宽高不变", async () => {
    const heights: number[] = [];
    const widths: number[] = [];
    const trace: string[] = [];

    for (let round = 0; round <= 3; round++) {
      if (round > 0) {
        await restartApp(app);
        main = await mainPage(app);
      }
      const size = await mainInnerSize(main);
      const state = readWindowState("main");
      heights.push(size.height);
      widths.push(size.width);
      trace.push(
        `第${round}次启动 viewport=${size.width}x${size.height} dpr=${size.dpr} ` +
          `persisted=${state.width}x${state.height}`,
      );
    }

    e2eLog.info(`[win-size] ${trace.join(" | ")}`);
    expect(heights.at(-1)).toBe(heights[0]);
    expect(widths.at(-1)).toBe(widths[0]);
  });

  test("双击标题栏不会最大化（主窗口禁止最大化）", async () => {
    const before = await mainInnerSize(main);

    // 标题栏空白处双击：Tauri 注入脚本在 detail === 2 时 invoke internal_toggle_maximize，
    // 而 tauri 侧会判 is_maximizable()——主窗口 conf 里 maximizable:false，这里必须什么都不发生
    const header = main.locator("header");
    const box = (await header.boundingBox())!;
    await main.mouse.dblclick(box.x + 40, box.y + box.height / 2);
    await main.waitForTimeout(300);

    const after = await mainInnerSize(main);
    e2eLog.info(`[maximize-guard] ${JSON.stringify(before)} → ${JSON.stringify(after)}`);
    // 只看尺寸：最大化会把视口撑到屏幕大小；位置不断言——标题栏按下还会走一次原生拖动循环，
    // 期间真实鼠标移动会带动窗口
    expect(after.width).toBe(before.width);
    expect(after.height).toBe(before.height);
  });
});
