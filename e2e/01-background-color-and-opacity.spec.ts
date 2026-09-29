/// 主界面背景色与透明度：取色器的实时预览 / 关闭落盘 / 撤销 / 重置 / 透明度 / 重启保持。
///
/// 取色器是页面内 popover，无法用 Playwright 操作原生控件，故按 DOM 事件序列驱动
/// （拖动期间多次 input、关闭时一次 change，且关闭会把取色器里的颜色推回输入框），
/// 这是浏览器对 <input type="color"> 的标准分工，也是被测逻辑唯一关心的输入。
import { expect, test, type Page } from "@playwright/test";
import {
  backgroundRgba,
  closeColorPicker,
  colorInput,
  DEFAULT_BACKGROUND_COLOR,
  disposeApp,
  expectPersistedColor,
  expectPersistedOpacity,
  launchApp,
  mainPage,
  openSettingsWindow,
  pickColor,
  readPersistedSettings,
  resetButton,
  restartApp,
  setOpacity,
  undoButton,
  type AppHandle,
} from "./e2e-helpers";

/// 被测色与它对应的 rgb（#1a2b3c → 26, 43, 60）；用非默认色才能区分「已落盘/未落盘」
const PICKED = "#1a2b3c";
const PICKED_RGB = [26, 43, 60];
const DEFAULT = DEFAULT_BACKGROUND_COLOR;
const DEFAULT_RGB = [15, 23, 42];

test.describe("主界面背景色与透明度", () => {
  let app!: AppHandle;
  let main!: Page;
  let settings!: Page;

  test.beforeAll(async () => {
    app = await launchApp();
    main = await mainPage(app);
    settings = await openSettingsWindow(app, main);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  /// 走一次完整的取色器选色：拖动（input）+ 关闭（change）
  async function setBackgroundColor(color: string): Promise<void> {
    await pickColor(settings, color);
    await closeColorPicker(settings);
    await expectPersistedColor(app.dataDir, color);
  }

  test("拖动取色器时主窗口实时预览且不落盘", async () => {
    await expect(colorInput(settings)).toHaveValue(DEFAULT);

    await pickColor(settings, PICKED);

    // 色块跟着走：:value 若绑已保存值，Vue 会在这里把色块打回旧色
    await expect(colorInput(settings)).toHaveValue(PICKED);
    await expect.poll(() => backgroundRgba(main)).toEqual([...PICKED_RGB, 1]);
    expect(readPersistedSettings(app.dataDir).background_color, "预览阶段不应落盘").not.toBe(
      PICKED,
    );
    // 改色瞬间就能撤销，不必等关闭取色器
    await expect(undoButton(settings)).toBeVisible();
  });

  test("关闭取色器后落盘", async () => {
    await closeColorPicker(settings);

    await expectPersistedColor(app.dataDir, PICKED);
    await expect(colorInput(settings)).toHaveValue(PICKED);
    await expect.poll(() => backgroundRgba(main)).toEqual([...PICKED_RGB, 1]);
    await expect(undoButton(settings)).toBeVisible();
  });

  test("撤销回到上一次的颜色", async () => {
    await undoButton(settings).click();

    await expect(colorInput(settings)).toHaveValue(DEFAULT);
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);
    await expectPersistedColor(app.dataDir, DEFAULT);
    // 单步撤销：用完即失效
    await expect(undoButton(settings)).toBeHidden();
  });

  test("取色器打开期间撤销＝放弃本次取色", async () => {
    await expect(colorInput(settings)).toHaveValue(DEFAULT);

    await pickColor(settings, PICKED);
    await expect(undoButton(settings)).toBeVisible();
    await undoButton(settings).click();
    await expect(colorInput(settings)).toHaveValue(DEFAULT);

    // 取色器关闭会把里面那个已被放弃的颜色推回输入框，此时不能再提交它
    await pickColor(settings, PICKED);
    await closeColorPicker(settings, PICKED);

    await expect(colorInput(settings)).toHaveValue(DEFAULT);
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);
    await expectPersistedColor(app.dataDir, DEFAULT);
    await expect(undoButton(settings)).toBeHidden();
  });

  test("重置恢复默认背景色", async () => {
    await setBackgroundColor(PICKED);
    await expect(resetButton(settings)).toBeVisible();

    await resetButton(settings).click();

    await expect(colorInput(settings)).toHaveValue(DEFAULT);
    await expect.poll(() => backgroundRgba(main)).toEqual([...DEFAULT_RGB, 1]);
    await expectPersistedColor(app.dataDir, DEFAULT);
    await expect(resetButton(settings)).toBeHidden();
  });

  test("拖动透明度滑杆实时生效并落盘", async () => {
    await setBackgroundColor(PICKED);

    await setOpacity(settings, 40);

    await expect.poll(() => backgroundRgba(main)).toEqual([...PICKED_RGB, 0.4]);
    await expectPersistedOpacity(app.dataDir, 40);
  });

  test("重启后背景色与透明度保持", async () => {
    await setBackgroundColor(PICKED);
    await setOpacity(settings, 55);
    await expectPersistedOpacity(app.dataDir, 55);

    await restartApp(app);

    main = await mainPage(app);
    await expect.poll(() => backgroundRgba(main)).toEqual([...PICKED_RGB, 0.55]);
    settings = await openSettingsWindow(app, main);
    await expect(colorInput(settings)).toHaveValue(PICKED);
    await expect(settings.getByRole("slider")).toHaveValue("55");
  });
});
