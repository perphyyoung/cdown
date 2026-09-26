<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands } from "@/bindings";
import { daysUntil, formatDays, rowState } from "./logic";
import { countdownState } from "./useCountdown";

// 行内编辑行：新增草稿与修改既有行共用；倒计时列只读，随目标日期实时重算并按分级着色
const { draft, today, settings, error, commitEdit, cancelEdit } = countdownState();

const days = computed(() => daysUntil(draft.value.targetDate, today.value));
const preview = computed(() => rowState(days.value, settings.value.levels));

const rowEl = ref<HTMLElement | null>(null);
// 日期弹窗打开期间它持有焦点，会触发本行的 focusout/pointerdown —— 期间抑制自动提交，
// 否则弹窗一开编辑行就退出，整个界面跳动
const pickerOpen = ref(false);

// 失焦自动保存：焦点移到行内其它输入框不触发；弹窗打开期间不触发
function onBlur(e: FocusEvent) {
  if (pickerOpen.value) return;
  const next = e.relatedTarget as Node | null;
  if (next && rowEl.value?.contains(next)) return;
  void commitEdit();
}

// 点击非可聚焦区域（行外空白等）不会触发 blur，用 document 级 pointerdown 兜底
function onDocPointerdown(e: PointerEvent) {
  if (pickerOpen.value) {
    pickerOpen.value = false; // 弹窗已随之关闭，本次点击不提交，再点一次才提交
    return;
  }
  if (rowEl.value && !rowEl.value.contains(e.target as Node)) void commitEdit();
}

onMounted(() => document.addEventListener("pointerdown", onDocPointerdown, true));
onUnmounted(() => document.removeEventListener("pointerdown", onDocPointerdown, true));

// 日期弹窗：点击日期框在下方开独立日历窗口；选中经事件回写草稿
let unlistenPicked: UnlistenFn | null = null;
onMounted(async () => {
  unlistenPicked = await listen<string>("date-picked", (e) => {
    draft.value.targetDate = e.payload;
    pickerOpen.value = false;
  });
});
onUnmounted(() => unlistenPicked?.());

async function openDatePicker(e: MouseEvent) {
  pickerOpen.value = true;
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const win = getCurrentWindow();
  const pos = (await win.outerPosition()).toLogical(await win.scaleFactor());
  try {
    await commands.openDatePicker(
      pos.x + rect.left,
      pos.y + rect.bottom + 4,
      draft.value.targetDate || null,
    );
  } catch (err) {
    error.value = String(err);
  }
}
</script>

<template>
  <div
    ref="rowEl"
    class="grid h-9 items-center gap-x-2 rounded bg-slate-800/40 px-1 text-center"
    title="回车保存，Esc 取消"
    @keydown.enter.prevent="commitEdit()"
    @keydown.esc="cancelEdit()"
    @focusout="onBlur"
  >
    <span
      class="text-xs font-medium"
      :class="preview.state === 'expired' ? 'text-red-400 line-through' : 'text-slate-400'"
      :style="preview.state === 'level' && preview.color ? { color: preview.color } : {}"
    >
      {{ Number.isNaN(days) ? "—" : formatDays(days) }}
    </span>
    <input
      v-model="draft.targetDate"
      type="text"
      inputmode="numeric"
      maxlength="10"
      placeholder="YYYY-MM-DD"
      class="min-w-0 rounded bg-slate-900/70 px-1.5 h-7 text-center text-xs text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500 placeholder:text-slate-500"
      @click="openDatePicker"
    />
    <input
      v-model="draft.title"
      type="text"
      placeholder="名称"
      data-focus-first
      class="min-w-0 rounded bg-slate-900/70 px-1.5 h-7 text-center text-xs text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500 placeholder:text-slate-500"
    />
    <input
      v-model="draft.note"
      type="text"
      placeholder="备注"
      class="min-w-0 rounded bg-slate-900/70 px-1.5 h-7 text-center text-xs text-slate-100 outline-none ring-1 ring-slate-700 focus:ring-slate-500 placeholder:text-slate-500"
    />
  </div>
</template>
