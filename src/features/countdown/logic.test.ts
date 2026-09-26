import { describe, expect, it } from "vitest";
import { daysUntil, formatDays, rowState, todayStr } from "./logic";

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
  it("过去为 expired", () => {
    expect(rowState(-1, 3)).toBe("expired");
    expect(rowState(-100, 3)).toBe("expired");
  });

  it("当天与阈值内（含边界）为 soon", () => {
    expect(rowState(0, 3)).toBe("soon");
    expect(rowState(3, 3)).toBe("soon");
  });

  it("超过阈值为 normal", () => {
    expect(rowState(4, 3)).toBe("normal");
  });

  it("阈值为 0 时仅当天变红", () => {
    expect(rowState(0, 0)).toBe("soon");
    expect(rowState(1, 0)).toBe("normal");
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
