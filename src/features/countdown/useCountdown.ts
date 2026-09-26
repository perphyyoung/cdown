import { computed, onMounted, onUnmounted, ref } from "vue";
import { commands, type CountdownItem, type Settings } from "@/bindings";
import { daysUntil, rowState, todayStr, type RowState } from "./logic";

export interface FormValue {
  title: string;
  targetDate: string;
  note: string | null;
}

/** 表格四列宽度（px），可拖拽调整并持久化 */
export interface ColumnWidths {
  name: number;
  target: number;
  remaining: number;
  note: number;
}

export interface CountdownRow {
  item: CountdownItem;
  days: number;
  state: RowState;
}

interface SettingsView {
  red_threshold_days: number;
  column_widths: ColumnWidths;
}

// bindings 里字段因 Rust 侧 serde(default) 导出为可选，读取前先归一化
const DEFAULT_WIDTHS: ColumnWidths = { name: 92, target: 70, remaining: 52, note: 50 };

function normalizeSettings(cfg: Settings): SettingsView {
  const w = cfg.column_widths ?? {};
  return {
    red_threshold_days: cfg.red_threshold_days ?? 3,
    column_widths: {
      name: w.name ?? DEFAULT_WIDTHS.name,
      target: w.target ?? DEFAULT_WIDTHS.target,
      remaining: w.remaining ?? DEFAULT_WIDTHS.remaining,
      note: w.note ?? DEFAULT_WIDTHS.note,
    },
  };
}

// 模块级单例状态（App 只有一处调用）
const items = ref<CountdownItem[]>([]);
const settings = ref<SettingsView>({
  red_threshold_days: 3,
  column_widths: { ...DEFAULT_WIDTHS },
});
const today = ref(todayStr());
const ready = ref(false);
const error = ref("");

let timer: number | null = null;

function tick() {
  today.value = todayStr();
}

function onVisibility() {
  // 恢复可见立即重算（跨午夜 / 长眠唤醒）
  if (!document.hidden) tick();
}

export function useCountdown() {
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

  onMounted(() => {
    void reload();
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

  const rows = computed<CountdownRow[]>(() =>
    items.value
      .map((item) => {
        const days = daysUntil(item.target_date, today.value);
        return { item, days, state: rowState(days, settings.value.red_threshold_days) };
      })
      .sort((a, b) => a.days - b.days),
  );

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

  async function saveSettings(patch: { redThresholdDays?: number; columnWidths?: ColumnWidths }) {
    const next: Settings = {
      red_threshold_days: patch.redThresholdDays ?? settings.value.red_threshold_days,
      column_widths: patch.columnWidths ?? settings.value.column_widths,
    };
    settings.value = normalizeSettings(await commands.setSettings(next));
  }

  return {
    items,
    settings,
    today,
    rows,
    ready,
    error,
    reload,
    addItem,
    updateItem,
    deleteItem,
    saveSettings,
  };
}
