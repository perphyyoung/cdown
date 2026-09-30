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

/**
 * 背景色（`#RRGGBB`）+ 百分比透明度合成 CSS 颜色。
 * 只用 rgba 而非元素 opacity：后者会把子元素一起变透明。
 * 非法色值回落 slate-900（与后端 DEFAULT_BACKGROUND_COLOR 一致）。
 */
export function hexToRgba(hex: string, percent: number): string {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  const v = m ? parseInt(m[1], 16) : 0x0f172a;
  const [r, g, b] = [(v >> 16) & 255, (v >> 8) & 255, v & 255];
  const a = Math.min(1, Math.max(0, (Number(percent) || 0) / 100));
  return `rgba(${r}, ${g}, ${b}, ${a})`;
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
