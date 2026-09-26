<script setup lang="ts">
import { onMounted, ref } from "vue";
import { emit } from "@tauri-apps/api/event";
import { VueDatePicker } from "@vuepic/vue-datepicker";
import { commands } from "@/bindings";

// 独立日期选择弹窗：inline 日历 + auto-apply，选中即广播并关窗；无时间选择
const value = ref<Date | null>(null);

onMounted(async () => {
  const iso = await commands.getDatePickerPayload();
  if (!iso) return;
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (m) value.value = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
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
</script>

<template>
  <div class="h-screen overflow-hidden bg-slate-800 p-1">
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
  </div>
</template>
