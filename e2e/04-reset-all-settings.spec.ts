/**
 * 设置页「还原所有配置」：预置一份全非默认的设置 + 两条倒计时，
 * 点还原并二次确认后——设置页控件、主窗口实时表现、落盘文件全部回默认，
 * 倒计时数据不动；重启后默认值与数据都仍在。
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";
import {
  backgroundRgba,
  dataDirFor,
  disposeApp,
  launchApp,
  mainPage,
  openSettingsWindow,
  readPersistedSettings,
  restartApp,
  type AppHandle,
} from "./e2e-helpers";

const DEFAULT_RGB = [15, 23, 42];

/// 主窗口 html 上的 CSS 变量值（字号/字体家族 watch 写入）
function cssVar(page: Page, name: string): Promise<string> {
  return page.evaluate(
    (n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
    name,
  );
}

/// 表格数据行数（数据行容器是 .grid.h-9；表头无 h-9）
function rowCount(main: Page) {
  return main.locator(".grid.h-9").count();
}

test.describe("还原所有配置", () => {
  let app!: AppHandle;
  let main!: Page;
  let settings!: Page;

  test.beforeAll(async () => {
    // 全非默认设置：透明度/背景色/字号/字体家族/置顶/热键/分级/列宽
    const dataDir = dataDirFor(4);
    fs.mkdirSync(dataDir, { recursive: true });
    const data = {
      items: [
        {
          id: "rst-01",
          title: "还原测试一",
          target_date: "2026-12-01",
          note: null,
          created_at: "2026-09-01T00:00:00Z",
        },
        {
          id: "rst-02",
          title: "还原测试二",
          target_date: "2026-12-31",
          note: "备注",
          created_at: "2026-09-01T00:00:00Z",
        },
      ],
      settings: {
        levels: [{ threshold_days: 30, color: "#123456" }],
        column_widths: { name: 200, target: 120, countdown: 100, note: 110 },
        always_on_top: false,
        hotkey: null,
        background_opacity: 55,
        background_color: "#1a2b3c",
        font_family: "SimSun",
        font_size: 18,
      },
    };
    fs.writeFileSync(path.join(dataDir, "cdown.json"), JSON.stringify(data, null, 2));

    app = await launchApp(4);
    main = await mainPage(app);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  test("二次确认后所有配置回默认，倒计时不动，重启后仍保持", async () => {
    // —— 还原前：预置的非默认值确已生效 ——
    expect(await rowCount(main)).toBe(2);
    expect(await cssVar(main, "--table-font-size")).toBe("18px");
    expect(await cssVar(main, "--font-family")).toContain("SimSun");
    await expect.poll(() => backgroundRgba(main)).toEqual([26, 43, 60, 0.55]);

    settings = await openSettingsWindow(app, main);
    await expect(settings.getByRole("slider", { name: "背景透明度" })).toHaveValue("55");
    await expect(settings.getByRole("slider", { name: "表格字体大小" })).toHaveValue("18");
    await expect(settings.locator("input[type='color']").first()).toHaveValue("#1a2b3c");

    // —— 点还原 → 二次确认 ——
    await settings.getByRole("button", { name: "还原默认" }).click();
    // ConfirmDialog 未挂 ARIA 角色，按遮罩层 class 与标题文本定位
    const box = settings.locator(".fixed.inset-0.z-\\[110\\]");
    await expect(box).toBeVisible();
    await expect(settings.getByRole("heading", { name: "还原所有配置" })).toBeVisible();
    // 弹窗确认按钮与底部触发按钮同名，取弹层内的一个
    await box.getByRole("button", { name: "还原默认" }).click();
    await expect(box).toBeHidden();

    // —— 设置页控件全部回默认 ——
    await expect(settings.getByRole("slider", { name: "背景透明度" })).toHaveValue("100");
    await expect(settings.getByRole("slider", { name: "表格字体大小" })).toHaveValue("14");
    await expect(settings.locator("input[type='color']").first()).toHaveValue("#0f172a");
    await expect(settings.locator('button[title="跟随系统"]')).toBeVisible();

    // —— 主窗口实时回退 ——
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);
    expect(await cssVar(main, "--table-font-size")).toBe("14px");
    // 空家族 = 移除内联覆盖，计算值回落 :root 默认栈（不再含 SimSun）
    expect(await cssVar(main, "--font-family")).not.toContain("SimSun");
    expect(await rowCount(main)).toBe(2);

    // —— 落盘即默认 ——
    const saved = readPersistedSettings(app.dataDir);
    expect(saved.background_opacity).toBe(100);
    expect(saved.background_color).toBe("#0f172a");
    expect(saved.font_size).toBe(14);
    expect(saved.font_family).toBe("");
    expect(saved.always_on_top).toBe(true);

    // —— 重启后默认值与倒计时数据都仍在（reload 异步，poll 等渲染）——
    await restartApp(app);
    main = await mainPage(app);
    await expect.poll(() => rowCount(main)).toBe(2);
    await expect.poll(() => cssVar(main, "--table-font-size")).toBe("14px");
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);

    settings = await openSettingsWindow(app, main);
    await expect(settings.getByRole("slider", { name: "表格字体大小" })).toHaveValue("14");
    await expect(settings.getByRole("slider", { name: "背景透明度" })).toHaveValue("100");
  });
});
