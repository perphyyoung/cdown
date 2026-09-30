// `countdown/font.ts` 的字体家族单测（`pnpm test:ui`，node 环境，无真实 DOM）。
//
// 测什么：家族名清洗（一个 `;` 就能把整条 font-family 打崩）、font-family 值的拼接与回退、
// 中英文搜索文本、长列表渲染窗口，以及本机字体枚举的三条回退路径（无 API / 拒授权 / 空列表）与去重。
// 不测什么：真实授权弹窗与字体渲染效果（依赖 WebView）。
//
// loadSystemFonts 只 stub window/navigator 并在自己的 afterEach 撤掉。
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_FONT_STACK,
  FALLBACK_FONT_FAMILIES,
  buildFontFamilyValue,
  displayFontFamily,
  fontFamilySearchText,
  fontListWindow,
  loadSystemFonts,
  sanitizeFontFamily,
} from "./font";

describe("sanitizeFontFamily", () => {
  it("剥掉引号，避免二次加引号破坏 CSS", () => {
    expect(sanitizeFontFamily('"Microsoft YaHei"')).toBe("Microsoft YaHei");
  });

  it("去掉会破坏 CSS 的字符（; : { }）：截断在第一个非法字符处", () => {
    const cleaned = sanitizeFontFamily("Arial;}html{color:red");
    expect(cleaned).toBe("Arial");
    expect(cleaned).not.toMatch(/["';:{}]/);
  });

  it("保留中文族名与常用符号（. _ -）", () => {
    expect(sanitizeFontFamily("思源黑体 CN")).toBe("思源黑体 CN");
    expect(sanitizeFontFamily("LXGW_WenKai-Regular.otf")).toBe("LXGW_WenKai-Regular.otf");
  });

  it("纯符号/空白 → 空串（走默认字体栈）", () => {
    expect(sanitizeFontFamily(";;;")).toBe("");
    expect(sanitizeFontFamily("   ")).toBe("");
  });

  it("超长值截断（异常值兜底）", () => {
    expect(sanitizeFontFamily("A".repeat(500)).length).toBe(64);
  });
});

describe("buildFontFamilyValue", () => {
  it("空值 → 默认字体栈", () => {
    expect(buildFontFamilyValue("")).toBe(DEFAULT_FONT_STACK);
  });

  it("非空 → 用户字体在前，默认栈兜底", () => {
    expect(buildFontFamilyValue("Microsoft YaHei")).toBe(
      `"Microsoft YaHei", ${DEFAULT_FONT_STACK}`,
    );
  });

  it("先清洗再拼接：非法字符不会进入 CSS", () => {
    const value = buildFontFamilyValue('"Arial";color:red');
    expect(value.startsWith('"Arial"')).toBe(true);
    expect(value).toContain(DEFAULT_FONT_STACK);
    expect(value).not.toContain("color:red");
  });
});

describe("displayFontFamily", () => {
  const map = { "Microsoft YaHei": "微软雅黑" };

  it("有中文映射 → 中文名 (English)", () => {
    expect(displayFontFamily("Microsoft YaHei", map)).toBe("微软雅黑 (Microsoft YaHei)");
  });

  it("无映射 → 原样返回英文族名", () => {
    expect(displayFontFamily("Arial", map)).toBe("Arial");
  });

  it("空族名（跟随系统）→ 空串，不显示括号", () => {
    expect(displayFontFamily("", map)).toBe("");
  });
});

describe("fontFamilySearchText", () => {
  const map = { "Microsoft YaHei": "微软雅黑" };

  it("中英文都能被搜到（搜「雅黑」也要命中 Microsoft YaHei）", () => {
    const text = fontFamilySearchText("Microsoft YaHei", map);
    expect(text.toLowerCase()).toContain("microsoft");
    expect(text).toContain("微软雅黑");
  });

  it("无映射时只含英文族名", () => {
    expect(fontFamilySearchText("Arial", map)).toBe("Arial");
  });
});

describe("fontListWindow", () => {
  // 用 f0..f9 造一个有序列表，便于用下标断言窗口位置
  const all = Array.from({ length: 10 }, (_, i) => `f${i}`);

  it("无选中（跟随系统）→ 从头开始", () => {
    expect(fontListWindow(all, "", 5, 2)).toEqual(["f0", "f1", "f2", "f3", "f4"]);
  });

  it("选中项在窗口外 → 窗口前移，且保留字母序", () => {
    expect(fontListWindow(all, "f7", 5, 2)).toEqual(["f5", "f6", "f7", "f8", "f9"]);
  });

  it("选中项靠近开头 → 不移动窗口（避免上溢）", () => {
    expect(fontListWindow(all, "f1", 5, 2)).toEqual(["f0", "f1", "f2", "f3", "f4"]);
  });

  it("选中项不在列表中（字体已卸载）→ 从头开始", () => {
    expect(fontListWindow(all, "不存在的字体", 5, 2)).toEqual(["f0", "f1", "f2", "f3", "f4"]);
  });

  it("列表短于窗口大小 → 原样返回", () => {
    expect(fontListWindow(all, "f9", 50, 2)).toEqual(all);
  });

  it("窗口尽量填满：末尾对齐，不为留白裁掉前面", () => {
    // 期望起点 7 会让窗口只剩 3 项，应回退到 2 使窗口填满 8 项
    expect(fontListWindow(all, "f9", 8, 2)).toEqual(all.slice(2));
  });
});

describe("loadSystemFonts", () => {
  afterEach(() => {
    vi.stubGlobal("window", undefined);
    vi.stubGlobal("navigator", undefined);
  });

  it("无 queryLocalFonts（内核过旧）→ 回退候选表 + unsupported", async () => {
    vi.stubGlobal("window", {});
    const result = await loadSystemFonts();
    expect(result).toMatchObject({ families: FALLBACK_FONT_FAMILIES, status: "unsupported" });
    expect(result.detail).toContain("queryLocalFonts");
  });

  it("用户在授权框点了拒绝（NotAllowedError）→ unreadable，原因里带异常名", async () => {
    vi.stubGlobal("window", {
      queryLocalFonts: () =>
        Promise.reject(Object.assign(new Error("denied"), { name: "NotAllowedError" })),
    });
    const result = await loadSystemFonts();
    expect(result).toMatchObject({ families: FALLBACK_FONT_FAMILIES, status: "unreadable" });
    expect(result.detail).toContain("NotAllowedError");
  });

  it("枚举到空列表 → 也按读不到处理（WebView2 拒绝授权时可能返回空数组）", async () => {
    vi.stubGlobal("window", { queryLocalFonts: () => Promise.resolve([]) });
    const result = await loadSystemFonts();
    expect(result).toMatchObject({ families: FALLBACK_FONT_FAMILIES, status: "unreadable" });
    expect(result.detail).toContain("0 个字体家族");
  });

  it("正常枚举 → ok + 去重后的家族名（逐 style 返回，同一 family 多次出现）", async () => {
    vi.stubGlobal("window", {
      queryLocalFonts: () =>
        Promise.resolve([
          { family: "Microsoft YaHei" },
          { family: "Microsoft YaHei" },
          { family: "Arial" },
        ]),
    });
    await expect(loadSystemFonts()).resolves.toEqual({
      families: ["Arial", "Microsoft YaHei"],
      status: "ok",
      detail: "",
    });
  });

  it("丢弃空家族名；非法字符按截断处理，不影响其它项", async () => {
    vi.stubGlobal("window", {
      queryLocalFonts: () =>
        Promise.resolve([{ family: "  " }, { family: "Bad;Name" }, { family: "SimHei" }]),
    });
    await expect(loadSystemFonts()).resolves.toEqual({
      families: ["Bad", "SimHei"],
      status: "ok",
      detail: "",
    });
  });
});
