<script setup lang="ts">
import { ref, watch } from "vue";

// 设置页通用数字输入：失焦/回车时 clamp 到 [min, max] 并上报
const props = defineProps<{ modelValue: number; min?: number; max?: number }>();
const emit = defineEmits<{ change: [value: number] }>();

const value = ref(props.modelValue);
watch(
  () => props.modelValue,
  (v) => (value.value = v),
);

function commit() {
  const min = props.min ?? 0;
  const max = props.max ?? 365;
  const n = Math.max(min, Math.min(max, Math.floor(Number(value.value) || 0)));
  value.value = n;
  if (n !== props.modelValue) emit("change", n);
}
</script>

<template>
  <input
    v-model.number="value"
    type="number"
    :min="min ?? 0"
    :max="max ?? 365"
    class="w-14 shrink-0 rounded bg-slate-800 px-1.5 py-1 text-center text-sm text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500"
    @change="commit"
  />
</template>
