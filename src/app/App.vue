<script setup lang="ts">
import { onUnmounted } from "vue";
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

async function saveSettings(patch: { levels?: UrgencyLevel[]; columnWidths?: ColumnWidths }) {
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
  <div v-else class="flex h-full select-none flex-col bg-slate-900 text-slate-100">
    <!-- 标题栏：无边框窗口拖动区 + 添加/设置/隐藏按钮 -->
    <header class="flex h-8 shrink-0 items-center pl-3" data-tauri-drag-region>
      <span class="text-xs font-semibold tracking-wide text-slate-400" data-tauri-drag-region>
        cdown 倒计时
      </span>
      <span class="flex-1" data-tauri-drag-region></span>
      <button
        class="h-8 rounded px-2.5 text-slate-500 hover:bg-slate-800 hover:text-slate-100"
        title="添加倒计时"
        @click="startAdd()"
      >
        ＋
      </button>
      <button
        class="h-8 rounded px-2.5 text-slate-500 hover:bg-slate-800 hover:text-slate-100"
        title="设置"
        @click="openSettings()"
      >
        ⚙
      </button>
      <button
        class="h-8 rounded px-2.5 text-slate-500 hover:bg-slate-800 hover:text-slate-100"
        title="隐藏到托盘"
        @click="getCurrentWindow().hide()"
      >
        —
      </button>
    </header>

    <p v-if="error" class="mx-2 mb-1 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200">
      {{ error }}
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
