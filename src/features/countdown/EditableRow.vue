<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { daysUntil, formatDays } from "./logic";
import { countdownState } from "./useCountdown";

// 行内编辑行：新增草稿与修改既有行共用；倒计时列只读，随目标日期实时重算
const { draft, today, commitEdit, cancelEdit } = countdownState();

const daysLabel = computed(() => {
  const d = daysUntil(draft.value.targetDate, today.value);
  return Number.isNaN(d) ? "—" : formatDays(d);
});

const rowEl = ref<HTMLElement | null>(null);

// 失焦自动保存：焦点移到行内其它输入框不触发
function onBlur(e: FocusEvent) {
  const next = e.relatedTarget as Node | null;
  if (next && rowEl.value?.contains(next)) return;
  void commitEdit();
}

// 点击非可聚焦区域（行外空白等）不会触发 blur，用 document 级 pointerdown 兜底
function onDocPointerdown(e: PointerEvent) {
  if (rowEl.value && !rowEl.value.contains(e.target as Node)) void commitEdit();
}

onMounted(() => document.addEventListener("pointerdown", onDocPointerdown, true));
onUnmounted(() => document.removeEventListener("pointerdown", onDocPointerdown, true));
</script>

<template>
  <div
    ref="rowEl"
    class="grid items-center gap-x-2 rounded bg-slate-800/40 px-1 py-1 text-center"
    title="回车保存，Esc 取消"
    @keydown.enter.prevent="commitEdit()"
    @keydown.esc="cancelEdit()"
    @focusout="onBlur"
  >
    <span class="text-xs font-medium text-slate-400">{{ daysLabel }}</span>
    <input
      v-model="draft.targetDate"
      type="date"
      class="min-w-0 rounded bg-slate-900/70 px-1.5 py-0.5 text-center text-xs text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500"
    />
    <input
      v-model="draft.title"
      type="text"
      placeholder="名称"
      data-focus-first
      class="min-w-0 rounded bg-slate-900/70 px-1.5 py-0.5 text-center text-xs text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500 placeholder:text-slate-500"
    />
    <input
      v-model="draft.note"
      type="text"
      placeholder="备注"
      class="min-w-0 rounded bg-slate-900/70 px-1.5 py-0.5 text-center text-xs text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500 placeholder:text-slate-500"
    />
  </div>
</template>
