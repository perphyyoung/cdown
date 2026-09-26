<script setup lang="ts">
import { onMounted, ref } from "vue";
import { emit, listen } from "@tauri-apps/api/event";
import { VueDatePicker } from "@vuepic/vue-datepicker";
import { commands } from "@/bindings";

// 独立日期选择弹窗：inline 日历 + auto-apply，选中即广播并关窗；无时间选择
const value = ref<Date | null>(null);

function applyIso(iso: string | null) {
  if (!iso) return;
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (m) value.value = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

onMounted(async () => {
  applyIso(await commands.getDatePickerPayload());
  // 窗口常驻复用：再次打开时主窗口经此事件推送新的初始值
  await listen<string | null>("date-payload", (e) => applyIso(e.payload));
});

function toIso(v: Date): string {
  return `${v.getFullYear()}-${String(v.getMonth() + 1).padStart(2, "0")}-${String(
    v.getDate(),
  ).padStart(2, "0")}`;
}

async function onPick(v: Date | null) {
  if (!v) return;
  await emit("date-picked", toIso(v));
  await commands.closeDatePicker();
}

// 「今天」= 定位视图并写入草稿，但保持弹窗打开，便于继续调整其它日期
async function pickToday() {
  value.value = new Date();
  await emit("date-picked", toIso(value.value));
}
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden bg-slate-800 p-1">
    <VueDatePicker
      v-model="value"
      inline
      auto-apply
      dark
      :enable-time-picker="false"
      :clearable="false"
      :formats="{ month: 'MM' }"
      @update:model-value="onPick"
    />
    <button
      class="mt-1 w-full rounded bg-slate-700 py-1 text-xs text-slate-100 hover:bg-slate-600"
      @click="pickToday"
    >
      今天
    </button>
  </div>
</template>
