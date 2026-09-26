<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import { formatDays } from "./logic";
import { countdownState, type ColumnWidths, type CountdownRow } from "./useCountdown";
import type { CountdownItem } from "@/bindings";
import EditableRow from "./EditableRow.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const props = defineProps<{ rows: CountdownRow[]; widths: ColumnWidths }>();
const emit = defineEmits<{
  remove: [id: string];
  resize: [widths: ColumnWidths];
}>();

const MIN = 24;
const MAX = 400;

// 拖拽过程中的即时宽度走本地副本，pointerup 才上报持久化
const local = reactive<ColumnWidths>({ ...props.widths });
watch(
  () => props.widths,
  (w) => Object.assign(local, w),
);

let drag: { col: keyof ColumnWidths; startX: number; startW: number } | null = null;

function onDown(col: keyof ColumnWidths, e: PointerEvent) {
  drag = { col, startX: e.clientX, startW: local[col] };
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}
function onMove(e: PointerEvent) {
  if (!drag) return;
  local[drag.col] = Math.min(MAX, Math.max(MIN, drag.startW + e.clientX - drag.startX));
}
function onUp() {
  if (!drag) return;
  drag = null;
  emit("resize", { ...local });
}

// 列顺序：倒计时、目标日期、名称、备注
const gridStyle = computed(() => ({
  gridTemplateColumns: `${local.countdown}px ${local.target}px ${local.name}px ${local.note}px`,
}));

// 行着色：level 态使用用户自定义颜色（内联样式覆盖默认字色）
function rowStyle(row: CountdownRow) {
  return row.state === "level" && row.color
    ? { ...gridStyle.value, color: row.color }
    : gridStyle.value;
}

const { editing, startEdit } = countdownState();

const mainEl = ref<HTMLElement | null>(null);
// 进入编辑时聚焦名称输入框
watch(editing, async (v) => {
  if (!v) return;
  await nextTick();
  mainEl.value?.querySelector<HTMLInputElement>("input[data-focus-first]")?.focus();
});

function isEditing(id: string) {
  return editing.value?.mode === "edit" && editing.value.id === id;
}

// 行右键菜单：全应用唯一的右键入口（默认菜单已在 main.ts 全局禁用）。
// 删除走自定义确认对话框（双击误确认，菜单内两步确认已废弃）。
const menu = ref<{ x: number; y: number; item: CountdownItem } | null>(null);
const pendingDelete = ref<CountdownItem | null>(null);

function openMenu(e: MouseEvent, item: CountdownItem) {
  const mw = 96;
  const mh = 76;
  menu.value = {
    x: Math.min(e.clientX, window.innerWidth - mw - 4),
    y: Math.min(e.clientY, window.innerHeight - mh - 4),
    item,
  };
}
function closeMenu() {
  menu.value = null;
}
function menuEdit() {
  if (menu.value) startEdit(menu.value.item);
  closeMenu();
}
function menuRemove() {
  if (!menu.value) return;
  pendingDelete.value = menu.value.item;
  closeMenu();
}

function confirmRemove() {
  if (pendingDelete.value) emit("remove", pendingDelete.value.id);
  pendingDelete.value = null;
}
</script>

<template>
  <main ref="mainEl" class="min-h-0 min-w-0 flex-1 overflow-x-auto overflow-y-auto px-2 pb-1">
    <div class="min-w-max">
      <div
        class="grid items-center gap-x-2 border-b border-slate-800 px-1 py-1 text-center text-xs text-slate-500"
        :style="gridStyle"
      >
        <span class="relative">
          倒计时
          <span
            class="absolute -right-1 top-0 h-full w-2 cursor-col-resize hover:bg-slate-600/60"
            title="拖拽调整列宽"
            @pointerdown="onDown('countdown', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
          ></span>
        </span>
        <span class="relative">
          目标日期
          <span
            class="absolute -right-1 top-0 h-full w-2 cursor-col-resize hover:bg-slate-600/60"
            title="拖拽调整列宽"
            @pointerdown="onDown('target', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
          ></span>
        </span>
        <span class="relative">
          名称
          <span
            class="absolute -right-1 top-0 h-full w-2 cursor-col-resize hover:bg-slate-600/60"
            title="拖拽调整列宽"
            @pointerdown="onDown('name', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
          ></span>
        </span>
        <span class="relative">
          备注
          <span
            class="absolute -right-1 top-0 h-full w-2 cursor-col-resize hover:bg-slate-600/60"
            title="拖拽调整列宽"
            @pointerdown="onDown('note', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
          ></span>
        </span>
      </div>
      <div
        v-if="rows.length === 0 && editing?.mode !== 'add'"
        class="py-8 text-center text-xs text-slate-500"
      >
        暂无倒计时，点右上角 ＋ 添加
      </div>
      <template v-for="row in rows" :key="row.item.id">
        <EditableRow v-if="isEditing(row.item.id)" :style="gridStyle" />
        <div
          v-else
          class="grid items-center gap-x-2 rounded px-1 py-1.5 text-center hover:bg-slate-800/60"
          :style="rowStyle(row)"
          :class="
            row.state === 'expired'
              ? 'text-red-400/70'
              : row.state === 'level'
                ? ''
                : 'text-slate-200'
          "
          @contextmenu.prevent="openMenu($event, row.item)"
        >
          <span class="text-xs font-medium">{{ formatDays(row.days) }}</span>
          <span class="text-xs text-slate-400">{{ row.item.target_date }}</span>
          <span class="truncate text-sm" :title="row.item.title">{{ row.item.title }}</span>
          <span class="truncate text-xs text-slate-500" :title="row.item.note ?? ''">
            {{ row.item.note }}
          </span>
        </div>
      </template>
      <!-- 新增草稿行：固定显示在最后，保存后随列表按倒计时重排 -->
      <EditableRow v-if="editing?.mode === 'add'" :style="gridStyle" />
    </div>

    <template v-if="menu">
      <!-- 透明遮罩：点击/右键任意处关闭菜单 -->
      <div class="fixed inset-0 z-40" @pointerdown="closeMenu" @contextmenu.prevent="closeMenu" />
      <div
        class="fixed z-50 min-w-[96px] rounded bg-slate-800 py-1 text-xs shadow-xl shadow-black/40 ring-1 ring-slate-700"
        :style="{ left: `${menu.x}px`, top: `${menu.y}px` }"
      >
        <button
          class="block w-full px-3 py-1.5 text-left text-slate-200 hover:bg-slate-700"
          @click="menuEdit()"
        >
          编辑
        </button>
        <button
          class="block w-full px-3 py-1.5 text-left text-red-300 hover:bg-slate-700"
          @click="menuRemove()"
        >
          删除
        </button>
      </div>
    </template>

    <ConfirmDialog
      :open="pendingDelete !== null"
      title="删除倒计时"
      :message="`删除「${pendingDelete?.title ?? ''}」？删除后不可恢复。`"
      confirm-text="删除"
      danger
      @confirm="confirmRemove"
      @cancel="pendingDelete = null"
    />
  </main>
</template>
