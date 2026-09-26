<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{ threshold: number }>();
const emit = defineEmits<{ change: [days: number] }>();

const value = ref(props.threshold);
watch(
  () => props.threshold,
  (v) => (value.value = v),
);

function commit() {
  const n = Math.max(0, Math.min(365, Math.floor(Number(value.value) || 0)));
  value.value = n;
  if (n !== props.threshold) emit("change", n);
}
</script>

<template>
  <label class="flex items-center gap-1.5 text-xs text-slate-500">
    临近
    <input
      v-model.number="value"
      type="number"
      min="0"
      max="365"
      class="w-12 rounded bg-slate-800 px-1.5 py-0.5 text-center text-xs text-slate-100 outline-none focus:ring-1 focus:ring-slate-500"
      @change="commit"
    />
    天内变红
  </label>
</template>
