import { computed, onMounted, onUnmounted, ref } from "vue";
import { commands, type CountdownItem } from "@/bindings";
import { daysUntil, rowState, todayStr, type RowState } from "./logic";

export interface FormValue {
  title: string;
  targetDate: string;
  note: string | null;
}

export interface CountdownRow {
  item: CountdownItem;
  days: number;
  state: RowState;
}

// 模块级单例状态（App 只有一处调用）
const items = ref<CountdownItem[]>([]);
// bindings 里 red_threshold_days 因 Rust 侧 serde(default) 导出为可选，读取前先归一化
const settings = ref<{ red_threshold_days: number }>({ red_threshold_days: 3 });
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
      settings.value = { red_threshold_days: cfg.red_threshold_days ?? 3 };
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

  async function setThreshold(days: number) {
    const cfg = await commands.setSettings(days);
    settings.value = { red_threshold_days: cfg.red_threshold_days ?? days };
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
    setThreshold,
  };
}
