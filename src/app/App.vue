<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CountdownTable from "@/features/countdown/CountdownTable.vue";
import ItemForm from "@/features/countdown/ItemForm.vue";
import SettingsRow from "@/features/countdown/SettingsRow.vue";
import { useCountdown, type ColumnWidths, type FormValue } from "@/features/countdown/useCountdown";
import type { CountdownItem } from "@/bindings";

const {
  rows,
  settings,
  ready,
  error,
  addItem,
  updateItem,
  deleteItem,
  saveSettings: persistSettings,
} = useCountdown();

const editing = ref<CountdownItem | null>(null);

async function onSubmit(value: FormValue) {
  try {
    if (editing.value) await updateItem(editing.value.id, value);
    else await addItem(value);
    editing.value = null;
  } catch (e) {
    error.value = String(e);
  }
}

async function onRemove(id: string) {
  try {
    await deleteItem(id);
    if (editing.value?.id === id) editing.value = null;
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
    <!-- 标题栏：无边框窗口拖动区 + 隐藏按钮 -->
    <header class="flex h-8 shrink-0 items-center pl-3" data-tauri-drag-region>
      <span class="text-xs font-semibold tracking-wide text-slate-400" data-tauri-drag-region>
        cdown 倒计时
      </span>
      <span class="flex-1" data-tauri-drag-region></span>
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
        @edit="editing = $event"
        @remove="onRemove"
        @resize="saveSettings({ columnWidths: $event })"
      />

      <ItemForm
        :editing="editing"
        class="shrink-0 border-t border-slate-800 p-2"
        @submit="onSubmit"
        @cancel="editing = null"
      />

      <SettingsRow
        :threshold="settings.red_threshold_days"
        class="shrink-0 border-t border-slate-800 px-2 py-1.5"
        @change="saveSettings({ redThresholdDays: $event })"
      />
    </template>
  </div>
</template>
