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
  colorInput,
  dataDirFor,
  disposeApp,
  launchApp,
  mainPage,
  openSettingsWindow,
  readPersistedColumnWidths,
  readPersistedSettings,
  restartApp,
  type AppHandle,
} from "./e2e-helpers";

const DEFAULT_RGB = [15, 23, 42];
const ITEM_TITLES = ["还原测试一", "还原测试二"] as const;
/// 预置的非默认列宽，grid 列顺序为 countdown/target/name/note
const PRESET_WIDTHS = { name: 200, target: 120, countdown: 100, note: 110 };
const PRESET_GRID = "100px 120px 200px 110px";

/// 主窗口 html 上的 CSS 变量值（字号/字体家族 watch 写入）
function cssVar(page: Page, name: string): Promise<string> {
  return page.evaluate(
    (n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
    name,
  );
}

/// 表头行的 grid-template-columns（列宽渲染的直接证据）
async function headerGridTemplate(main: Page): Promise<string> {
  return main
    .getByRole("table", { name: "倒计时表格" })
    .getByRole("row")
    .first()
    .evaluate((el) => getComputedStyle(el).gridTemplateColumns);
}

/// 两条倒计时的名称单元格都可见（数据未被还原动作删除）
async function expectItemsKept(main: Page): Promise<void> {
  for (const title of ITEM_TITLES) {
    await expect(main.getByRole("cell", { name: title })).toBeVisible();
  }
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
          title: ITEM_TITLES[0],
          target_date: "2026-12-01",
          note: null,
          created_at: "2026-09-01T00:00:00Z",
        },
        {
          id: "rst-02",
          title: ITEM_TITLES[1],
          target_date: "2026-12-31",
          note: "备注",
          created_at: "2026-09-01T00:00:00Z",
        },
      ],
      settings: {
        levels: [{ threshold_days: 30, color: "#123456" }],
        always_on_top: false,
        hotkey: null,
        background_opacity: 55,
        background_color: "#1a2b3c",
        font_family: "SimSun",
        font_size: 18,
      },
      // 列宽已移出 settings：与 items/settings 平级的顶层独立数据
      column_widths: PRESET_WIDTHS,
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
    await expectItemsKept(main);
    expect(await cssVar(main, "--table-font-size")).toBe("18px");
    expect(await cssVar(main, "--font-family")).toContain("SimSun");
    await expect.poll(() => backgroundRgba(main)).toEqual([26, 43, 60, 0.55]);
    // 预置列宽（顶层字段）已加载到表格
    expect(await headerGridTemplate(main)).toBe(PRESET_GRID);

    settings = await openSettingsWindow(app, main);
    await expect(settings.getByRole("slider", { name: "背景透明度" })).toHaveValue("55");
    await expect(settings.getByRole("slider", { name: "表格字体大小" })).toHaveValue("18");
    await expect(colorInput(settings)).toHaveValue("#1a2b3c");

    // —— 点还原 → 二次确认 ——
    await settings.getByRole("button", { name: "还原默认" }).click();
    const dialog = settings.getByRole("dialog", { name: "还原所有配置" });
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "还原默认" }).click();
    await expect(dialog).toBeHidden();

    // —— 设置页控件全部回默认 ——
    await expect(settings.getByRole("slider", { name: "背景透明度" })).toHaveValue("100");
    await expect(settings.getByRole("slider", { name: "表格字体大小" })).toHaveValue("14");
    await expect(colorInput(settings)).toHaveValue("#0f172a");
    await expect(settings.getByTitle("跟随系统")).toBeVisible();

    // —— 主窗口实时回退 ——
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);
    expect(await cssVar(main, "--table-font-size")).toBe("14px");
    // 空家族 = 移除内联覆盖，计算值回落 :root 默认栈（不再含 SimSun）
    expect(await cssVar(main, "--font-family")).not.toContain("SimSun");
    await expectItemsKept(main);

    // —— 落盘即默认 ——
    const saved = readPersistedSettings(app.dataDir);
    expect(saved.background_opacity).toBe(100);
    expect(saved.background_color).toBe("#0f172a");
    expect(saved.font_size).toBe(14);
    expect(saved.font_family).toBe("");
    expect(saved.always_on_top).toBe(true);

    // —— 列宽不属于设置：还原不动，仍在顶层且表格渲染不变 ——
    expect(saved.column_widths).toBeUndefined();
    expect(readPersistedColumnWidths(app.dataDir)).toEqual(PRESET_WIDTHS);
    expect(await headerGridTemplate(main)).toBe(PRESET_GRID);

    // —— 重启后默认值、列宽与倒计时数据都仍在（reload 异步，可见断言自带等待）——
    await restartApp(app);
    main = await mainPage(app);
    await expectItemsKept(main);
    expect(await cssVar(main, "--table-font-size")).toBe("14px");
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);
    await expect.poll(() => headerGridTemplate(main)).toBe(PRESET_GRID);

    settings = await openSettingsWindow(app, main);
    await expect(settings.getByRole("slider", { name: "表格字体大小" })).toHaveValue("14");
    await expect(settings.getByRole("slider", { name: "背景透明度" })).toHaveValue("100");
  });
});
