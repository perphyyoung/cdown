import { computed, onMounted, onUnmounted, ref } from "vue";
import { commands, type CountdownItem, type Settings } from "@/bindings";
import {
  daysUntil,
  rowState,
  roundColumnWidths,
  todayStr,
  type RowState,
  type UrgencyLevel,
} from "./logic";

export type { UrgencyLevel };

export interface FormValue {
  title: string;
  targetDate: string;
  note: string | null;
}

/** 表格四列宽度（px），可拖拽调整并持久化（类型与取整逻辑在 logic.ts） */
import type { ColumnWidths } from "./logic";
export type { ColumnWidths };

export interface CountdownRow {
  item: CountdownItem;
  days: number;
  state: RowState;
  /** 仅 level 态有值 */
  color?: string;
}

/** 行内编辑目标：add = 新增草稿行（显示在最后）；edit = 修改既有行 */
export type EditTarget = { mode: "add" } | { mode: "edit"; id: string };

interface SettingsView {
  levels: UrgencyLevel[];
  column_widths: ColumnWidths;
  alwaysOnTop: boolean;
  /** 全局热键 accelerator（如 Ctrl+Alt+C）；null = 关闭热键 */
  hotkey: string | null;
  /** 主窗口背景透明度（%），10–100 */
  backgroundOpacity: number;
  /** 主窗口背景色 `#RRGGBB` */
  backgroundColor: string;
  /** 全局字体家族（完整 CSS font-family 值）；空串 = 跟随系统默认栈 */
  fontFamily: string;
}

// bindings 里字段因 Rust 侧 serde(default) 导出为可选，读取前先归一化
const DEFAULT_WIDTHS: ColumnWidths = { name: 92, target: 70, countdown: 52, note: 50 };
const FALLBACK_COLOR = "#fb923c";
/** 与后端 DEFAULT_BACKGROUND_COLOR 保持一致 */
export const DEFAULT_BACKGROUND_COLOR = "#0f172a";

function normalizeSettings(cfg: Settings): SettingsView {
  const w = cfg.column_widths ?? {};
  const levels = (cfg.levels ?? []).map((l) => ({
    thresholdDays: l.threshold_days ?? 0,
    color: l.color ?? FALLBACK_COLOR,
  }));
  return {
    levels,
    column_widths: {
      name: w.name ?? DEFAULT_WIDTHS.name,
      target: w.target ?? DEFAULT_WIDTHS.target,
      countdown: w.countdown ?? DEFAULT_WIDTHS.countdown,
      note: w.note ?? DEFAULT_WIDTHS.note,
    },
    alwaysOnTop: cfg.always_on_top ?? true,
    hotkey: cfg.hotkey ?? null,
    backgroundOpacity: cfg.background_opacity ?? 100,
    backgroundColor: cfg.background_color ?? DEFAULT_BACKGROUND_COLOR,
    fontFamily: cfg.font_family ?? "",
  };
}

// 模块级单例状态：App 挂生命周期，子组件经 countdownState() 共享同一份
const items = ref<CountdownItem[]>([]);
const settings = ref<SettingsView>({
  levels: [
    { thresholdDays: 7, color: "#a78bfa" },
    { thresholdDays: 3, color: "#facc15" },
    { thresholdDays: 0, color: "#fb923c" },
  ],
  column_widths: { ...DEFAULT_WIDTHS },
  alwaysOnTop: true,
  hotkey: null,
  backgroundOpacity: 100,
  backgroundColor: DEFAULT_BACKGROUND_COLOR,
  fontFamily: "",
});
const today = ref(todayStr());
const ready = ref(false);
const error = ref("");

const editing = ref<EditTarget | null>(null);
const draft = ref<FormValue>({ title: "", targetDate: "", note: null });

let timer: number | null = null;

function tick() {
  today.value = todayStr();
}

function onVisibility() {
  // 恢复可见立即重算（跨午夜 / 长眠唤醒）
  if (!document.hidden) tick();
}

async function reload() {
  try {
    const [list, cfg] = await Promise.all([commands.listItems(), commands.getSettings()]);
    items.value = list;
    settings.value = normalizeSettings(cfg);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  } finally {
    ready.value = true;
  }
}

async function addItem(value: FormValue) {
  await commands.addItem(value.title, value.targetDate, value.note);
  await reload();
}

async function updateItem(id: string, value: FormValue) {
  await commands.updateItem(id, value.title, value.targetDate, value.note);
  await reload();
}

async function deleteItem(id: string) {
  await commands.deleteItem(id);
  await reload();
}

async function saveSettings(patch: {
  levels?: UrgencyLevel[];
  columnWidths?: ColumnWidths;
  alwaysOnTop?: boolean;
  hotkey?: string | null;
  backgroundOpacity?: number;
  backgroundColor?: string;
  fontFamily?: string;
}) {
  const next: Settings = {
    levels: (patch.levels ?? settings.value.levels).map((l) => ({
      threshold_days: l.thresholdDays,
      color: l.color,
    })),
    // 列宽必须取整：后端 u32 不接受浮点（缩放屏拖拽会产生小数）
    column_widths: roundColumnWidths(patch.columnWidths ?? settings.value.column_widths),
    always_on_top: patch.alwaysOnTop ?? settings.value.alwaysOnTop,
    // 热键必须显式带上：字段缺失会被后端 serde default 填回默认键，
    // 故用 in 判别「未修改」与「关闭热键（null）」
    hotkey: "hotkey" in patch ? (patch.hotkey ?? null) : settings.value.hotkey,
    background_opacity: Math.round(patch.backgroundOpacity ?? settings.value.backgroundOpacity),
    background_color: patch.backgroundColor ?? settings.value.backgroundColor,
    // 字体家族：空串是合法取值（跟随系统），`??` 只在 undefined 时兜底，不会吞掉它
    font_family: patch.fontFamily ?? settings.value.fontFamily,
  };
  settings.value = normalizeSettings(await commands.setSettings(next));
}

function startAdd() {
  // 默认日期为当天：日历直接落在当前月，多数场景只需改名称
  draft.value = { title: "", targetDate: today.value, note: null };
  editing.value = { mode: "add" };
}

function startEdit(item: CountdownItem) {
  draft.value = { title: item.title, targetDate: item.target_date, note: item.note };
  editing.value = { mode: "edit", id: item.id };
}

function cancelEdit() {
  editing.value = null;
}

// 提交进行中标志：同一次点击会先后触发 pointerdown 兜底保存与 focusout 失焦保存，
// addItem 又是异步 IPC——不加防重入会同一草稿创建两条相同记录
let committing = false;

async function commitEdit() {
  const target = editing.value;
  if (!target || committing) return;
  const value = draft.value;
  if (value.title.trim() === "" || !/^\d{4}-\d{2}-\d{2}$/.test(value.targetDate)) {
    error.value = "名称不能为空，目标日期需为 YYYY-MM-DD";
    return;
  }
  committing = true;
  try {
    if (target.mode === "add") await addItem(value);
    else await updateItem(target.id, value);
    // 仅当仍是本次提交的编辑目标时才退出编辑态（失焦保存与「＋」新建可能竞态）
    if (editing.value === target) editing.value = null;
  } catch (e) {
    error.value = String(e);
  } finally {
    committing = false;
  }
}

export function countdownState() {
  return {
    items,
    settings,
    today,
    ready,
    error,
    editing,
    draft,
    rows: computed<CountdownRow[]>(() =>
      items.value
        .map((item) => {
          const days = daysUntil(item.target_date, today.value);
          const { state, color } = rowState(days, settings.value.levels);
          return { item, days, state, color };
        })
        // 过期行放最下面，其余按剩余天数升序
        .sort((a, b) => {
          const aExp = a.days < 0;
          const bExp = b.days < 0;
          if (aExp !== bExp) return aExp ? 1 : -1;
          return a.days - b.days;
        }),
    ),
    reload,
    addItem,
    updateItem,
    deleteItem,
    saveSettings,
    startAdd,
    startEdit,
    cancelEdit,
    commitEdit,
  };
}

/** 仅 App 调用：注册定时 tick 与可见性监听的生命周期 */
export function useCountdown() {
  const state = countdownState();
  onMounted(() => {
    void state.reload();
    tick();
    // 精确到天：天界在午夜，每分钟重算一次已远超需要（纯字符串日期差，开销可忽略）；
    // 页面隐藏时暂停，恢复可见时由 visibilitychange 立即重算。
    timer = window.setInterval(() => {
      if (!document.hidden) tick();
    }, 60_000);
    document.addEventListener("visibilitychange", onVisibility);
  });
  onUnmounted(() => {
    if (timer !== null) window.clearInterval(timer);
    timer = null;
    document.removeEventListener("visibilitychange", onVisibility);
  });
  return state;
}
