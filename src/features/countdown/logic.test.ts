import { describe, expect, it } from "vitest";
import { daysUntil, formatDays, rowState, todayStr, type UrgencyLevel } from "./logic";

describe("daysUntil", () => {
  it("同一天为 0", () => {
    expect(daysUntil("2026-09-26", "2026-09-26")).toBe(0);
  });

  it("跨月/跨年按日历天计算", () => {
    expect(daysUntil("2026-09-30", "2026-09-26")).toBe(4);
    expect(daysUntil("2026-10-01", "2026-09-26")).toBe(5);
    expect(daysUntil("2027-01-01", "2026-12-31")).toBe(1);
    // 闰年 2 月
    expect(daysUntil("2028-03-01", "2028-02-28")).toBe(2);
  });

  it("过去日期为负数", () => {
    expect(daysUntil("2026-09-25", "2026-09-26")).toBe(-1);
    expect(daysUntil("2020-01-01", "2026-09-26")).toBe(
      -Math.round(
        (Date.parse("2026-09-26T00:00:00Z") - Date.parse("2020-01-01T00:00:00Z")) / 86_400_000,
      ),
    );
  });

  it("非法日期返回 NaN", () => {
    expect(Number.isNaN(daysUntil("2030-1-1", "2026-09-26"))).toBe(true);
  });
});

describe("rowState", () => {
  const levels: UrgencyLevel[] = [
    { thresholdDays: 7, color: "#a78bfa" },
    { thresholdDays: 3, color: "#fb923c" },
  ];

  it("过去为 expired", () => {
    expect(rowState(-1, levels)).toEqual({ state: "expired" });
    expect(rowState(-100, levels)).toEqual({ state: "expired" });
  });

  it("命中最高级（含边界 days == threshold）", () => {
    expect(rowState(7, levels)).toEqual({ state: "level", color: "#a78bfa" });
    expect(rowState(6, levels)).toEqual({ state: "level", color: "#a78bfa" });
  });

  it("命中次级（含当天）", () => {
    expect(rowState(3, levels)).toEqual({ state: "level", color: "#fb923c" });
    expect(rowState(0, levels)).toEqual({ state: "level", color: "#fb923c" });
  });

  it("超过所有阈值为 normal", () => {
    expect(rowState(8, levels)).toEqual({ state: "normal" });
  });

  it("乱序输入也按降序命中最高可用级", () => {
    expect(rowState(5, [levels[1], levels[0]])).toEqual({ state: "level", color: "#a78bfa" });
  });

  it("空分级时全部 normal（过期除外）", () => {
    expect(rowState(0, [])).toEqual({ state: "normal" });
    expect(rowState(-1, [])).toEqual({ state: "expired" });
  });
});

describe("formatDays", () => {
  it("过期显示已过天数", () => {
    expect(formatDays(-1)).toBe("已过期 1 天");
    expect(formatDays(-30)).toBe("已过期 30 天");
  });

  it("今天/明天特称，其余显示天数", () => {
    expect(formatDays(0)).toBe("今天");
    expect(formatDays(1)).toBe("明天");
    expect(formatDays(2)).toBe("2 天");
    expect(formatDays(365)).toBe("365 天");
  });
});

describe("todayStr", () => {
  it("按本地日期补零输出 YYYY-MM-DD", () => {
    expect(todayStr(new Date(2026, 8, 26))).toBe("2026-09-26");
    expect(todayStr(new Date(2026, 0, 5))).toBe("2026-01-05");
  });
});
