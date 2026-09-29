/**
 * 复现「重启后主界面高度变高」：每轮重启都记录视口高度（用户所见）与插件落盘的几何。
 * 注意：窗口几何由插件写在应用配置目录，与 CDOWN_DATA_DIR 无关，本 spec 会改写本机
 * `window-state.dev.json`（即 pnpm dev 用的那份几何）。
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";
import {
  disposeApp,
  launchApp,
  mainInnerSize,
  mainPage,
  readWindowState,
  restartApp,
  type AppHandle,
} from "./e2e-helpers";

test.describe("主窗口几何", () => {
  let app: AppHandle;
  let main: Page;

  test.beforeAll(async () => {
    // 清空（不删）上一轮留下的日志，本次日志只含这一串重启
    fs.writeFileSync(path.join(import.meta.dirname, "..", "cdown.log"), "");
    app = await launchApp(0, "info");
    main = await mainPage(app);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  test("连续重启后主窗口高度不变", async () => {
    const heights: number[] = [];
    const trace: string[] = [];

    for (let round = 0; round <= 3; round++) {
      if (round > 0) {
        await restartApp(app);
        main = await mainPage(app);
      }
      const size = await mainInnerSize(main);
      const state = readWindowState("main");
      heights.push(size.height);
      trace.push(
        `第${round}次启动 viewport=${size.width}x${size.height} dpr=${size.dpr} ` +
          `persisted=${state.width}x${state.height}`,
      );
    }

    console.log(`[win-size] ${trace.join(" | ")}`);
    expect(heights.at(-1)).toBe(heights[0]);
  });
});
