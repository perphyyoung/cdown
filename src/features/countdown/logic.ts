// 倒计时纯逻辑：以「本地日期字符串 YYYY-MM-DD」为入参，无时钟依赖，可单测。
// 精度到天：不做秒级计时，天与天的分界在午夜。

export type RowState = "normal" | "soon" | "expired";

export function todayStr(date: Date = new Date()): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** 目标日期 − 今天的天数差（按日历天，忽略时区/时分秒）。 */
export function daysUntil(targetDate: string, today: string): number {
  const t = Date.parse(`${targetDate}T00:00:00Z`);
  const n = Date.parse(`${today}T00:00:00Z`);
  if (Number.isNaN(t) || Number.isNaN(n)) return Number.NaN;
  return Math.round((t - n) / 86_400_000);
}

/** days < 0 → expired；days ≤ thresholdDays（含当天 days = 0）→ soon；否则 normal。 */
export function rowState(days: number, thresholdDays: number): RowState {
  if (days < 0) return "expired";
  if (days <= thresholdDays) return "soon";
  return "normal";
}

export function formatDays(days: number): string {
  if (days < 0) return `已过期 ${-days} 天`;
  if (days === 0) return "今天";
  if (days === 1) return "明天";
  return `${days} 天`;
}
