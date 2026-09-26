<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import CountdownTable from "@/features/countdown/CountdownTable.vue";
import SettingsRow from "@/features/countdown/SettingsRow.vue";
import { useCountdown, type ColumnWidths } from "@/features/countdown/useCountdown";

const {
  rows,
  settings,
  ready,
  error,
  editing,
  deleteItem,
  saveSettings: persistSettings,
  startAdd,
  cancelEdit,
} = useCountdown();

async function onRemove(id: string) {
  try {
    await deleteItem(id);
    const cur = editing.value;
    if (cur?.mode === "edit" && cur.id === id) cancelEdit();
  } catch (e) {
    error.value = String(e);
  }
}

async function saveSettings(patch: { redThresholdDays?: number; columnWidths?: ColumnWidths }) {
  try {
    await persistSettings(patch);
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <div class="flex h-full select-none flex-col bg-slate-900 text-slate-100">
    <!-- 标题栏：无边框窗口拖动区 + 添加/隐藏按钮 -->
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
        title="隐藏到托盘"
        @click="getCurrentWindow().hide()"
      >
        —
      </button>
    </header>

    <p v-if="error" class="mx-2 mb-1 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200">
      {{ error }}
    </p>

    <template v-if="ready">
      <CountdownTable
        :rows="rows"
        :widths="settings.column_widths"
        @remove="onRemove"
        @resize="saveSettings({ columnWidths: $event })"
      />

      <SettingsRow
        :threshold="settings.red_threshold_days"
        class="shrink-0 border-t border-slate-800 px-2 py-1.5"
        @change="saveSettings({ redThresholdDays: $event })"
      />
    </template>
  </div>
</template>
