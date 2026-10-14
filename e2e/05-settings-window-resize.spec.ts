/**
 * 设置窗的几何行为（只看用户能看到的结果，不查实现细节）：
 *   - 高度契约：要么刚好装下内容（底部不留空白条），要么被屏幕可用高度夹住——此时内容区
 *     必须能纵向滚动，底部控件滚得出来（窄窗口下内容会高过一屏）；
 *   - 宽度只能在右边缘拖，左边缘拖不动；
 *   - 宽度有下限，拖过头会停住。
 *
 * 注意：窗口几何由插件写在应用配置目录，与 CDOWN_DATA_DIR 无关，本 spec 会改写本机
 * `window-state.dev.json`（即 pnpm dev 用的那份几何，末尾会拖回常规宽度）。
 * 断言按方向/是否变化/是否可滚来写：合成指针的移动会被 Chromium 量化合并，精确像素值属于实现细节。
 *
 * 跑本文件时**不要动鼠标**：拖拽走的是真实指针事件通道，人手移动会和合成拖拽混在一起
 * （曾出现窗口反向变窄：真指针把起点参照拖到了另一侧），这不是产品问题。
 */
import { expect, type Page } from "@playwright/test";
import {
  disposeApp,
  launchApp,
  mainPage,
  openSettingsWindow,
  test,
  type AppHandle,
} from "./e2e-helpers";
import { e2eLog } from "./e2e-logger";

/// 与 SettingsPage.vue 的 MIN_WIDTH 一致
const MIN_WIDTH = 360;

interface Geometry {
  width: number;
  height: number;
  screenX: number;
  availHeight: number;
}

/// 内容区（设置项那一块，唯一会滚的地方）的滚动指标：
/// scrollHeight = 内容自然高，clientHeight = 容器当前可视高
interface ScrollMetrics {
  scrollHeight: number;
  clientHeight: number;
}

/// 窗口视口尺寸 + 屏幕工作区高（窗口不该长过它）
function geometry(page: Page): Promise<Geometry> {
  return page.evaluate(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
    screenX: window.screenX,
    availHeight: window.screen.availHeight,
  }));
}

test.describe("设置窗缩放", () => {
  let app!: AppHandle;
  let main!: Page;
  let settings!: Page;

  test.beforeAll(async () => {
    app = await launchApp(5);
    main = await mainPage(app);
    settings = await openSettingsWindow(app, main);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  /// 从某个视口坐标按下并水平拖动 deltaX。
  /// 分几步、每步隔一帧：Chromium 会把同一帧内的多次 move 合并量化，一次性大位移可能只投递一部分
  async function dragFrom(x: number, y: number, deltaX: number): Promise<void> {
    await settings.mouse.move(x, y);
    await settings.mouse.down();
    const steps = 5;
    for (let i = 1; i <= steps; i++) {
      await settings.mouse.move(x + (deltaX * i) / steps, y);
      await settings.waitForTimeout(16);
    }
    await settings.mouse.up();
  }

  /// 拖右边缘：把手是贴右边缘的一条窄热区，取其中点即可
  async function dragRightEdge(deltaX: number): Promise<void> {
    const box = (await settings.locator('[data-resize-handle="right"]').boundingBox())!;
    await dragFrom(box.x + 3, box.y + box.height / 2, deltaX);
  }

  /// 松手后等尺寸落定再断言（真实用户也是拖完才看结果，不会在毫秒内读一次）
  async function settledGeometry(): Promise<Geometry> {
    let prev: Geometry | null = null;
    for (let i = 0; i < 40; i++) {
      const now = await geometry(settings);
      if (prev && prev.width === now.width && prev.height === now.height) return now;
      prev = now;
      await settings.waitForTimeout(25);
    }
    throw new Error("窗口尺寸未在 1 秒内稳定");
  }

  /// 内容区的滚动指标（用语义定位，不认类名）
  function scrollMetrics(): Promise<ScrollMetrics> {
    return settings.evaluate(() => {
      const el = document.querySelector<HTMLElement>('[aria-label="设置项"]');
      return { scrollHeight: el?.scrollHeight ?? -1, clientHeight: el?.clientHeight ?? -1 };
    });
  }

  /// 底部空白 = 窗口下边缘到「贴底的最后一个内容元素」的距离（正常只剩内容区下内边距 ~16px）。
  /// 为负 = 内容被夹住、最后一个元素在可视区下方（此时不可能有空白）。
  /// 不能拿内容容器的 clientHeight/scrollHeight 比较：容器本身会被 flex 拉伸，两边永远相等
  function bottomGap(page: Page): Promise<number> {
    return page.evaluate(() => {
      const root = document.querySelector<HTMLElement>('[role="region"][aria-label="设置窗口"]');
      const items = root?.querySelector<HTMLElement>('[aria-label="设置项"]');
      if (!root || !items) return -1;
      const boxes = [
        ...Array.from(items.children),
        // 滚动区之外还可能有状态条（msg/error）；它的高度是内容，不是空白
        ...Array.from(root.children).filter(
          (el) => el !== items && el.tagName !== "HEADER" && !el.hasAttribute("aria-hidden"),
        ),
      ];
      const bottom = Math.max(...boxes.map((el) => el.getBoundingClientRect().bottom));
      return Math.round(window.innerHeight - bottom);
    });
  }

  /// 高度契约（用户可见含义）：高度 = min(内容高, 可用高)，且底边不留空白区域
  async function expectHeightContract(geo: Geometry): Promise<void> {
    expect(geo.height, "窗口不该长过屏幕工作区").toBeLessThanOrEqual(geo.availHeight);
    const gap = await bottomGap(settings);
    const box = await scrollMetrics();
    e2eLog.info(
      `[settings-resize] 契约 ${JSON.stringify(geo)} 内容区 ${JSON.stringify(box)} 底部留白 ${gap}`,
    );
    expect(gap, "底边不该出现空白区域").toBeLessThanOrEqual(24);
  }

  /// 一路往左拖到宽度触底（每步都落在视口内），返回触底后的几何
  async function dragToMinWidth(): Promise<Geometry> {
    let last = await settledGeometry();
    for (let round = 0; round < 6; round++) {
      await dragRightEdge(-Math.max(80, last.width - MIN_WIDTH - 20));
      const now = await settledGeometry();
      if (now.width === last.width) break;
      last = now;
    }
    return settledGeometry();
  }

  test("打开时高度即等于内容高度", async () => {
    const open = await settledGeometry();
    e2eLog.info(`[settings-resize] 打开 ${JSON.stringify(open)}`);
    await expectHeightContract(open);
  });

  test("拖右边缘可以调宽，高度跟着内容走", async () => {
    const before = await settledGeometry();

    await dragRightEdge(90);
    const wider = await settledGeometry();
    e2eLog.info(`[settings-resize] 拖宽 ${before.width} → ${wider.width}`);
    expect(wider.width).toBeGreaterThan(before.width + 40);
    expect(wider.screenX, "只拖右边缘，窗口位置不动").toBe(before.screenX);
    await expectHeightContract(wider);

    await dragRightEdge(-90);
    const narrower = await settledGeometry();
    e2eLog.info(`[settings-resize] 拖窄 ${wider.width} → ${narrower.width}`);
    expect(narrower.width).toBeLessThan(wider.width - 40);
    expect(narrower.screenX, "只拖右边缘，窗口位置不动").toBe(before.screenX);
    await expectHeightContract(narrower);
  });

  test("左边缘拖不动", async () => {
    const before = await settledGeometry();

    // 贴左边缘按下并往右拖：既不该改宽，也不该带动窗口位移
    await dragFrom(2, before.height / 2, 80);

    const after = await settledGeometry();
    e2eLog.info(`[settings-resize] 左边缘拖 ${before.width} → ${after.width}`);
    expect(after.width).toBe(before.width);
    expect(after.screenX).toBe(before.screenX);
    await expectHeightContract(after);
  });

  test("宽度有下限：一直往左拖会停住", async () => {
    const after = await dragToMinWidth();
    e2eLog.info(`[settings-resize] 拖到下限 ${JSON.stringify(after)}`);
    expect(after.width).toBe(MIN_WIDTH);
    await expectHeightContract(after);

    // 再拖一次也不该更窄
    await dragRightEdge(-200);
    const again = await settledGeometry();
    expect(again.width).toBe(MIN_WIDTH);
    await expectHeightContract(again);
  });

  test("内容高过一屏时：高度被夹住，底部控件滚得出来，拖宽后恢复", async () => {
    const narrow = await dragToMinWidth();

    // 用界面把内容撑高（分级加满 6 级）：窄窗口 + 满分级 = 内容必超一屏
    const add = settings.getByRole("button", { name: "＋ 添加分级" });
    for (let i = 0; i < 8 && (await add.isEnabled()); i++) {
      await add.click();
      await settings.waitForTimeout(60);
    }

    const tall = await settledGeometry();
    const box = await scrollMetrics();
    e2eLog.info(
      `[settings-resize] 窄+满分級 ${JSON.stringify(tall)} 内容区 ${JSON.stringify(box)}`,
    );
    await expectHeightContract(tall);

    if (box.scrollHeight > box.clientHeight + 1) {
      // 真被夹住了：最底部的控件要能滚进可视区（用户滚轮能做到的事）
      const last = settings.getByRole("button", { name: "还原默认" });
      await last.scrollIntoViewIfNeeded();
      const rect = (await last.boundingBox())!;
      expect(rect.y + rect.height, "底部控件滚进可视区后应完全可见").toBeLessThanOrEqual(
        tall.height + 1,
      );
    }

    // 拖宽 → 内容变矮 → 不再需要滚动，也不留空白
    await dragRightEdge(400);
    const wide = await settledGeometry();
    const wideBox = await scrollMetrics();
    e2eLog.info(
      `[settings-resize] 拖宽后 ${JSON.stringify(wide)} 内容区 ${JSON.stringify(wideBox)}`,
    );
    expect(wide.width).toBeGreaterThan(narrow.width + 100);
    expect(wideBox.scrollHeight, "内容装得下时不该溢出").toBeLessThanOrEqual(
      wideBox.clientHeight + 1,
    );
    await expectHeightContract(wide);
  });

  test("拉宽后底边不留空白：内容变矮，窗口必须跟着变矮", async () => {
    // 先到最窄（内容变高、窗口被可用高夹住），再大幅拉宽——内容重排会明显变矮
    await dragToMinWidth();
    const narrow = await settledGeometry();
    e2eLog.info(`[settings-resize] 拉宽前 ${JSON.stringify(narrow)}`);

    await dragRightEdge(600);
    const wide = await settledGeometry();
    e2eLog.info(`[settings-resize] 拉宽后 ${JSON.stringify(wide)}`);

    expect(wide.width).toBeGreaterThan(narrow.width + 200);
    expect(
      await bottomGap(settings),
      "拉宽后内容变矮了，窗口底边不该留出空白区域",
    ).toBeLessThanOrEqual(24);
    await expectHeightContract(wide);
  });
});
