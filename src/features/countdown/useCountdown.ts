import { computed, onMounted, onUnmounted, ref } from "vue";
import { commands, type CountdownItem, type Settings } from "@/bindings";
import { daysUntil, rowState, todayStr, type RowState, type UrgencyLevel } from "./logic";

export type { UrgencyLevel };

export interface FormValue {
  title: string;
  targetDate: string;
  note: string | null;
}

/** 表格四列宽度（px），可拖拽调整并持久化 */
export interface ColumnWidths {
  name: number;
  target: number;
  countdown: number;
  note: number;
}

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
}

// bindings 里字段因 Rust 侧 serde(default) 导出为可选，读取前先归一化
const DEFAULT_WIDTHS: ColumnWidths = { name: 92, target: 70, countdown: 52, note: 50 };
const FALLBACK_COLOR = "#fb923c";

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
  };
}

// 模块级单例状态：App 挂生命周期，子组件经 countdownState() 共享同一份
const items = ref<CountdownItem[]>([]);
const settings = ref<SettingsView>({
  levels: [
    { thresholdDays: 7, color: "#a78bfa" },
    { thresholdDays: 3, color: "#fb923c" },
  ],
  column_widths: { ...DEFAULT_WIDTHS },
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

async function saveSettings(patch: { levels?: UrgencyLevel[]; columnWidths?: ColumnWidths }) {
  const next: Settings = {
    levels: (patch.levels ?? settings.value.levels).map((l) => ({
      threshold_days: l.thresholdDays,
      color: l.color,
    })),
    column_widths: patch.columnWidths ?? settings.value.column_widths,
  };
  settings.value = normalizeSettings(await commands.setSettings(next));
}

function startAdd() {
  draft.value = { title: "", targetDate: "", note: null };
  editing.value = { mode: "add" };
}

function startEdit(item: CountdownItem) {
  draft.value = { title: item.title, targetDate: item.target_date, note: item.note };
  editing.value = { mode: "edit", id: item.id };
}

function cancelEdit() {
  editing.value = null;
}

async function commitEdit() {
  const target = editing.value;
  if (!target) return;
  const value = draft.value;
  if (value.title.trim() === "" || !/^\d{4}-\d{2}-\d{2}$/.test(value.targetDate)) {
    error.value = "名称不能为空，目标日期需为 YYYY-MM-DD";
    return;
  }
  try {
    if (target.mode === "add") await addItem(value);
    else await updateItem(target.id, value);
    // 仅当仍是本次提交的编辑目标时才退出编辑态（失焦保存与「＋」新建可能竞态）
    if (editing.value === target) editing.value = null;
  } catch (e) {
    error.value = String(e);
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
