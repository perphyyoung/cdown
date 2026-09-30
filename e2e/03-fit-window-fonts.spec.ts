/**
 * 复现：切换字体家族后点标题栏「自适应宽高」，窗口应刚好包住表格且不出任何滚动条。
 *
 * 怀疑点：fitWindow 的宽高在 Windows 无边框窗口 setSize 后有 1px 欠账，
 * 宽度余量为零，欠 1px 先挤出横向滚动条（占 8px 高），再连锁挤出纵向滚动条。
 * 单实例内顺序测三种字体：直接写 --font-family 并强制加载字体后点自适应，
 * 量 DOM 与窗口尺寸对账（不刷新、不重启——reload 会断 CDP 导致 worker 重启）。
 */
import fs from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";
import { e2eLog } from "./e2e-logger";
import { dataDirFor, disposeApp, launchApp, mainPage, type AppHandle } from "./e2e-helpers";

const FONTS = ["Microsoft YaHei", "LXGW WenKai Screen R", "Noto Sans SC"] as const;
const DEFAULT_STACK =
  'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Microsoft YaHei", Roboto, Arial, sans-serif';

interface Measure {
  winW: number;
  winH: number;
  mainSW: number;
  mainCW: number;
  mainSH: number;
  mainCH: number;
  innerW: number;
  innerH: number;
  dpr: number;
}

async function measure(page: Page): Promise<Measure> {
  return page.evaluate(() => {
    const main = document.querySelector<HTMLElement>("main");
    const inner = main?.querySelector<HTMLElement>(".w-max");
    return {
      winW: window.innerWidth,
      winH: window.innerHeight,
      mainSW: main?.scrollWidth ?? 0,
      mainCW: main?.clientWidth ?? 0,
      mainSH: main?.scrollHeight ?? 0,
      mainCH: main?.clientHeight ?? 0,
      innerW: inner?.offsetWidth ?? 0,
      innerH: inner?.offsetHeight ?? 0,
      dpr: window.devicePixelRatio,
    };
  });
}

/// 同一实例内切字体：写 CSS 变量（等价 App.vue watch 的效果）+ 强制加载，不刷新页面
async function applyFont(page: Page, family: string): Promise<void> {
  const loaded = await page.evaluate(
    async ({ fam, stack }) => {
      document.documentElement.style.setProperty("--font-family", `"${fam}", ${stack}`);
      try {
        await Promise.all([
          document.fonts.load(`12px "${fam}"`),
          document.fonts.load(`14px "${fam}"`),
        ]);
        await document.fonts.ready;
        return document.fonts.check(`14px "${fam}"`);
      } catch {
        return false;
      }
    },
    { fam: family, stack: DEFAULT_STACK },
  );
  expect(loaded, `字体 ${family} 未在本机安装/不可用`).toBeTruthy();
  // 等字体真正作用到布局
  await page.waitForTimeout(100);
}

test.describe("字体家族 × 自适应宽高", () => {
  let app: AppHandle;
  let page: Page;

  test.beforeAll(async () => {
    // 启动前预置 cdown.json：应用首启即带数据，前端 rows 直接有内容，
    // 不必启动后 invoke 添加（那会绕过前端 reload，页面拿到的仍是空列表）
    const dataDir = dataDirFor(3);
    fs.mkdirSync(dataDir, { recursive: true });
    const items = [
      {
        id: "e2e-01",
        title: "kl6",
        target_date: "2026-10-03",
        note: null,
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-02",
        title: "workbuddy",
        target_date: "2026-10-04",
        note: "bash6",
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-03",
        title: "566",
        target_date: "2026-10-05",
        note: "9",
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-04",
        title: "895",
        target_date: "2026-10-06",
        note: null,
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-05",
        title: "trae-code",
        target_date: "2026-10-07",
        note: "Sign in to GitHub",
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-06",
        title: "586",
        target_date: "2026-10-08",
        note: null,
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-07",
        title: "696",
        target_date: "2026-10-09",
        note: null,
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-08",
        title: "587",
        target_date: "2026-09-27",
        note: null,
        created_at: "2026-09-01T00:00:00Z",
      },
      {
        id: "e2e-09",
        title: "trae-work",
        target_date: "2026-09-29",
        note: "测试测试测试测试",
        created_at: "2026-09-01T00:00:00Z",
      },
    ];
    fs.writeFileSync(path.join(dataDir, "cdown.json"), JSON.stringify({ items }, null, 2));

    app = await launchApp(3);
    page = await mainPage(app);
  });

  test.afterAll(async () => {
    await disposeApp(app);
  });

  for (const family of FONTS) {
    test(`自适应宽高刚好包住表格且无滚动条：${family}`, async () => {
      await applyFont(page, family);

      await page.getByTitle("自适应宽高").click();
      // fitWindow：重排列宽 → setSize（含回读补差），轮询到窗口尺寸稳定
      let m = await measure(page);
      for (let i = 0; i < 30; i++) {
        await page.waitForTimeout(100);
        const next = await measure(page);
        if (next.winW === m.winW && next.winH === m.winH) {
          m = next;
          break;
        }
        m = next;
      }

      e2eLog.info(
        `[fit-font] ${family} dpr=${m.dpr} win=${m.winW}x${m.winH} inner=${m.innerW}x${m.innerH} ` +
          `main scroll=${m.mainSW}x${m.mainSH} client=${m.mainCW}x${m.mainCH}`,
      );

      // 无横向/纵向溢出（出滚动条的直接判据）
      expect(m.mainSW, `${family} 横向溢出`).toBeLessThanOrEqual(m.mainCW);
      expect(m.mainSH, `${family} 纵向溢出`).toBeLessThanOrEqual(m.mainCH);
      // 窗口尺寸与「标题栏 32 + 表格 + 容器留白」对账，容差 1px（取整）
      expect(Math.abs(m.winW - (m.innerW + 16)), `${family} 窗口宽度不贴合`).toBeLessThanOrEqual(1);
      expect(
        Math.abs(m.winH - (32 + m.innerH + 4)),
        `${family} 窗口高度不贴合`,
      ).toBeLessThanOrEqual(1);
    });
  }
});
