<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { CountdownItem } from "@/bindings";
import type { FormValue } from "./useCountdown";

const props = defineProps<{ editing: CountdownItem | null }>();
const emit = defineEmits<{ submit: [value: FormValue]; cancel: [] }>();

const title = ref("");
const targetDate = ref("");
const note = ref("");

watch(
  () => props.editing,
  (it) => {
    title.value = it?.title ?? "";
    targetDate.value = it?.target_date ?? "";
    note.value = it?.note ?? "";
  },
  { immediate: true },
);

const valid = computed(
  () => title.value.trim() !== "" && /^\d{4}-\d{2}-\d{2}$/.test(targetDate.value),
);

function submit() {
  if (!valid.value) return;
  emit("submit", {
    title: title.value,
    targetDate: targetDate.value,
    note: note.value.trim() || null,
  });
}
</script>

<template>
  <form class="flex flex-col gap-1.5" @submit.prevent="submit">
    <div class="flex gap-1.5">
      <input
        v-model="title"
        type="text"
        placeholder="名称"
        class="min-w-0 flex-1 rounded bg-slate-800 px-2 py-1 text-sm text-slate-100 outline-none placeholder:text-slate-500 focus:ring-1 focus:ring-slate-500"
      />
      <input
        v-model="targetDate"
        type="date"
        class="w-32 rounded bg-slate-800 px-2 py-1 text-sm text-slate-100 outline-none focus:ring-1 focus:ring-slate-500"
      />
      <button
        type="submit"
        :disabled="!valid"
        class="rounded bg-slate-700 px-2.5 py-1 text-sm text-slate-100 hover:bg-slate-600 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {{ editing ? "保存" : "添加" }}
      </button>
      <button
        v-if="editing"
        type="button"
        class="rounded px-2 py-1 text-sm text-slate-400 hover:text-slate-100"
        @click="emit('cancel')"
      >
        取消
      </button>
    </div>
    <input
      v-model="note"
      type="text"
      placeholder="备注（可选）"
      class="rounded bg-slate-800 px-2 py-1 text-xs text-slate-100 outline-none placeholder:text-slate-500 focus:ring-1 focus:ring-slate-500"
    />
  </form>
</template>
