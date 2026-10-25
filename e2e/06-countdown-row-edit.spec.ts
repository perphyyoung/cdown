/**
 * 行内编辑入口：右键「备注」列 —— **备注为空时也要能**弹出菜单并进入编辑。
 *
 * 备注列是行网格的子项，行容器是 `grid items-center`：备注为空 ⇒ 这一格的高度是 0
 * ⇒ 挂在它身上的 `@contextmenu` 收不到事件（右键落到没有处理器的行容器上）⇒ 菜单不弹、
 * 也就没有「编辑」入口。本用例的判据就是这一下的**可达性**：先量格子的命中高度，再真右键一次。
 *
 * 数据用界面造：只填名称、备注留空 —— 不写 `cdown.json`，免得跟存储结构耦合。
 */
import { expect } from "@playwright/test";
import type { Page } from "@playwright/test";
import { disposeApp, launchApp, mainPage, test, type AppHandle } from "./e2e-helpers";
import { e2eLog } from "./e2e-logger";

test.describe("行内编辑入口", () => {
  let app!: AppHandle;
  let main!: Page;

  test.beforeAll(async () => {
    app = await launchApp(6);
    main = await mainPage(app);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  test("备注为空时，右键备注列仍能弹出菜单并进入编辑（备注框获得焦点）", async () => {
    // 新增一条：只填名称，备注留空（回车保存）
    await main.getByTitle("添加倒计时").click();
    const title = main.getByPlaceholder("名称");
    await expect(title).toBeFocused();
    await title.fill("空备注行");
    await main.keyboard.press("Enter");

    const row = main.getByRole("row").filter({ hasText: "空备注行" });
    await expect(row).toBeVisible();
    const noteCell = row.locator('[data-col="note"]');
    const box = (await noteCell.boundingBox())!;
    e2eLog.info(`[row-edit] 空备注格 ${JSON.stringify(box)}`);

    // 空格子也必须占满整行高，否则右键落不到它身上（修前这一格高度为 0）
    expect(box.height, "空备注格应有整行高的命中区").toBeGreaterThan(10);

    // 右键这一格中心 → 菜单 →「编辑」
    await main.mouse.click(box.x + box.width / 2, box.y + box.height / 2, { button: "right" });
    const edit = main.getByRole("button", { name: "编辑" });
    await expect(edit, "空备注格右键也要弹出菜单").toBeVisible();
    await edit.click();

    // 编辑行按"右键点中的列"分流初始焦点：点的是备注列，备注框应直接获得焦点
    const noteInput = main.getByPlaceholder("备注");
    await expect(noteInput).toBeFocused();
    await noteInput.fill("补上的备注");
    await main.keyboard.press("Enter");

    await expect(main.getByRole("row").filter({ hasText: "补上的备注" })).toBeVisible();
  });

  test("双击三个可编辑列分别进入编辑并聚焦对应输入框，只读列无反应", async () => {
    await main.getByTitle("添加倒计时").click();
    await main.getByPlaceholder("名称").fill("双击行");
    await main.keyboard.press("Enter");
    const row = main.getByRole("row").filter({ hasText: "双击行" });
    await expect(row).toBeVisible();

    // ① 双击空备注格：进入编辑、备注框获得焦点
    await row.locator('[data-col="note"]').dblclick();
    await expect(main.getByPlaceholder("备注"), "双击备注列应进入编辑").toBeFocused();
    await main.keyboard.press("Escape");
    await expect(main.getByPlaceholder("备注")).toHaveCount(0);

    // ② 双击名称格：聚焦名称框
    await row.locator('[data-col="name"]').dblclick();
    await expect(main.getByPlaceholder("名称"), "双击名称列应聚焦名称框").toBeFocused();
    await main.keyboard.press("Escape");

    // ③ 双击只读的倒计时列：不进入编辑
    await row.locator('[data-col="countdown"]').dblclick();
    await expect(main.getByPlaceholder("备注"), "只读列双击不该进入编辑").toHaveCount(0);
  });
});
