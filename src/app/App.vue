<script setup lang="ts">
import { onUnmounted, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import CountdownTable from "@/features/countdown/CountdownTable.vue";
import SettingsPage from "@/features/countdown/SettingsPage.vue";
import DatePickerWindow from "@/features/countdown/DatePickerWindow.vue";
import { commands } from "@/bindings";
import { log } from "@/utils/logger";
import {
  useCountdown,
  type ColumnWidths,
  type UrgencyLevel,
} from "@/features/countdown/useCountdown";

const {
  rows,
  settings,
  ready,
  error,
  editing,
  deleteItem,
  saveSettings: persistSettings,
  reload,
  startAdd,
  cancelEdit,
} = useCountdown();

// 多窗口共用同一份前端：按窗口 label 区分渲染内容
const windowLabel = getCurrentWindow().label;
log.info("[App] mounted, window =", windowLabel);
const isSettingsWindow = windowLabel === "settings";
const isDatePickerWindow = windowLabel === "date-picker";

let unlisteners: UnlistenFn[] = [];
// 设置窗口保存后广播，主窗口重拉设置
void listen("settings-changed", () => void reload()).then((u) => (unlisteners = [u]));
onUnmounted(() => unlisteners.forEach((u) => u()));

// 置顶：单一数据源是 Settings——图钉按钮只改设置，此处 watch 把它同步到窗口
// 实际状态（设置导入后经 settings-changed → reload 也会走到这里）。仅主窗口执行：
// 设置/日历窗口共用本组件，否则会把它们自身的置顶状态改掉。
if (windowLabel === "main") {
  watch(
    () => settings.value.alwaysOnTop,
    (v) => void getCurrentWindow().setAlwaysOnTop(v),
    { immediate: true },
  );
}

async function togglePin() {
  await saveSettings({ alwaysOnTop: !settings.value.alwaysOnTop });
}

async function openSettings() {
  try {
    await commands.openSettings();
  } catch (e) {
    error.value = String(e);
  }
}

async function onRemove(id: string) {
  try {
    await deleteItem(id);
    const cur = editing.value;
    if (cur?.mode === "edit" && cur.id === id) cancelEdit();
  } catch (e) {
    log.error("[remove] 删除失败", String(e));
    error.value = String(e);
  }
}

async function saveSettings(patch: {
  levels?: UrgencyLevel[];
  columnWidths?: ColumnWidths;
  alwaysOnTop?: boolean;
}) {
  try {
    await persistSettings(patch);
  } catch (e) {
    log.error("[settings] 保存失败", String(e));
    error.value = String(e);
  }
}
</script>

<template>
  <DatePickerWindow v-if="isDatePickerWindow" />
  <SettingsPage v-else-if="isSettingsWindow" />
  <!-- 主窗口背景透明度可调（slate-900 的 RGB），设置页保存后经 settings-changed 即时生效 -->
  <div
    v-else
    class="flex h-full select-none flex-col text-slate-100"
    :style="{ backgroundColor: `rgba(15, 23, 42, ${settings.backgroundOpacity / 100})` }"
  >
    <!-- 标题栏：无边框窗口拖动区 + 添加/设置/隐藏按钮 -->
    <header class="relative flex h-8 shrink-0 items-center" data-tauri-drag-region>
      <img src="/icon.png" alt="cdown" class="ml-2 h-4 w-4 select-none" draggable="false" />
      <!-- 标题居左：窄窗口（无倒计时行）下绝对居中会与右侧功能键重合 -->
      <span
        class="ml-1.5 text-xs font-semibold tracking-wide text-slate-400"
        data-tauri-drag-region
      >
        cdown
      </span>
      <span class="ml-auto flex items-center" data-tauri-drag-region>
        <button
          class="flex h-8 w-8 items-center justify-center rounded text-slate-500 hover:bg-slate-800 hover:text-slate-100"
          title="添加倒计时"
          @click="startAdd()"
        >
          +
        </button>
        <button
          class="flex h-8 w-8 items-center justify-center rounded hover:bg-slate-800"
          :class="settings.alwaysOnTop ? 'text-slate-100' : 'text-slate-500 hover:text-slate-100'"
          :title="settings.alwaysOnTop ? '取消置顶' : '置顶'"
          @click="togglePin()"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            :fill="settings.alwaysOnTop ? 'currentColor' : 'none'"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="transition-transform"
            :class="settings.alwaysOnTop ? 'rotate-45' : ''"
          >
            <line x1="12" x2="12" y1="17" y2="22" />
            <path
              d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"
            />
          </svg>
        </button>
        <button
          class="flex h-8 w-8 items-center justify-center rounded text-slate-500 hover:bg-slate-800 hover:text-slate-100"
          title="设置"
          @click="openSettings()"
        >
          ⚙
        </button>
        <button
          class="flex h-8 w-8 items-center justify-center rounded text-slate-500 hover:bg-slate-800 hover:text-slate-100"
          title="隐藏到托盘"
          @click="getCurrentWindow().hide()"
        >
          −
        </button>
      </span>
    </header>

    <p
      v-if="error"
      class="mx-2 mb-1 flex items-center justify-between gap-2 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200"
    >
      <span class="min-w-0 break-all">{{ error }}</span>
      <button
        class="shrink-0 px-1 text-red-200/70 hover:text-red-100"
        title="关闭"
        @click="error = ''"
      >
        ✕
      </button>
    </p>

    <CountdownTable
      v-if="ready"
      :rows="rows"
      :widths="settings.column_widths"
      @remove="onRemove"
      @resize="saveSettings({ columnWidths: $event })"
    />
  </div>
</template>
