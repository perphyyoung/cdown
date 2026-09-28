<script setup lang="ts">
// 设置页通用开关：轨道+滑块；不做内部状态反转，点击直接上报，
// 父级失败时保持原 modelValue 即完成视觉回滚
const props = defineProps<{ modelValue: boolean; disabled?: boolean }>();
const emit = defineEmits<{ change: [value: boolean] }>();

function toggle() {
  if (!props.disabled) emit("change", !props.modelValue);
}
</script>

<template>
  <button
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    class="relative h-5 w-9 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed disabled:opacity-40"
    :class="modelValue ? 'bg-emerald-600' : 'bg-slate-700'"
    @click="toggle"
  >
    <span
      class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-slate-100 transition-transform"
      :class="modelValue ? 'translate-x-4' : 'translate-x-0'"
    />
  </button>
</template>
