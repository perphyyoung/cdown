// 倒计时纯逻辑：以「本地日期字符串 YYYY-MM-DD」为入参，无时钟依赖，可单测。
// 精度到天：不做秒级计时，天与天的分界在午夜。

/** 紧急度分级：剩余天数 ≤ thresholdDays 时采用 color（levels 按阈值降序） */
export interface UrgencyLevel {
  thresholdDays: number;
  color: string;
}

export type RowState = "normal" | "level" | "expired";

/** 表格四列宽度（px）。跨 IPC 传给后端（u32）前必须取整，见 roundColumnWidths。 */
export interface ColumnWidths {
  name: number;
  target: number;
  countdown: number;
  note: number;
}

/**
 * 列宽取整：拖拽的 clientX 在缩放屏下带小数，直接传给后端 u32 会被拒
 * （invalid type: floating point, expected u32）。NaN/空值回落 0（后端再 clamp）。
 */
export function roundColumnWidths(w: ColumnWidths): ColumnWidths {
  const r = (v: number) => Math.round(Number(v) || 0);
  return {
    name: r(w.name),
    target: r(w.target),
    countdown: r(w.countdown),
    note: r(w.note),
  };
}

export interface RowStateResult {
  state: RowState;
  /** 仅 level 态有值 */
  color?: string;
}

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

/**
 * days < 0 → expired（固定红色样式）；
 * days ≤ 某级阈值 → level（返回该级颜色）；
 * 命中规则：在所有满足 days ≤ 阈值的级别里取阈值最小（最紧急）的一级；
 * 否则 normal。
 */
export function rowState(days: number, levels: UrgencyLevel[]): RowStateResult {
  if (days < 0) return { state: "expired" };
  const hit = levels
    .filter((l) => days <= l.thresholdDays)
    .sort((a, b) => a.thresholdDays - b.thresholdDays)[0];
  return hit ? { state: "level", color: hit.color } : { state: "normal" };
}

export function formatDays(days: number): string {
  if (days < 0) return `已过期 ${-days} 天`;
  if (days === 0) return "今天";
  if (days === 1) return "明天";
  return `${days} 天`;
}
