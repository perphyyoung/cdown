<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { formatDays } from "./logic";
import type { ColumnWidths, CountdownRow } from "./useCountdown";
import type { CountdownItem } from "@/bindings";

const props = defineProps<{ rows: CountdownRow[]; widths: ColumnWidths }>();
const emit = defineEmits<{
  edit: [item: CountdownItem];
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

const gridStyle = computed(() => ({
  gridTemplateColumns: `${local.name}px ${local.target}px ${local.remaining}px ${local.note}px`,
}));

// 行右键菜单：全应用唯一的右键入口（默认菜单已在 main.ts 全局禁用）
const menu = ref<{ x: number; y: number; item: CountdownItem } | null>(null);

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
</script>

<template>
  <main class="min-h-0 min-w-0 flex-1 overflow-x-auto overflow-y-auto px-2 pb-1">
    <div class="min-w-max">
      <div
        class="grid items-center gap-x-2 border-b border-slate-800 px-1 py-1 text-xs text-slate-500"
        :style="gridStyle"
      >
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
        <span class="relative text-right">
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
        <span class="relative text-right">
          剩余
          <span
            class="absolute -right-1 top-0 h-full w-2 cursor-col-resize hover:bg-slate-600/60"
            title="拖拽调整列宽"
            @pointerdown="onDown('remaining', $event)"
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
      <div v-if="rows.length === 0" class="py-8 text-center text-xs text-slate-500">
        暂无倒计时，在下方添加
      </div>
      <div
        v-for="row in rows"
        :key="row.item.id"
        class="group grid items-center gap-x-2 rounded px-1 py-1.5 hover:bg-slate-800/60"
        :style="gridStyle"
        :class="
          row.state === 'expired'
            ? 'text-red-400/70'
            : row.state === 'soon'
              ? 'text-red-400'
              : 'text-slate-200'
        "
        @contextmenu.prevent="openMenu($event, row.item)"
      >
        <span class="truncate text-sm" :title="row.item.title">{{ row.item.title }}</span>
        <span class="text-right text-xs text-slate-400">{{ row.item.target_date }}</span>
        <span class="text-right text-xs font-medium">{{ formatDays(row.days) }}</span>
        <span class="truncate text-xs text-slate-500" :title="row.item.note ?? ''">
          {{ row.item.note }}
        </span>
      </div>
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
          @click="
            emit('edit', menu.item);
            closeMenu();
          "
        >
          编辑
        </button>
        <button
          class="block w-full px-3 py-1.5 text-left text-red-300 hover:bg-slate-700"
          @click="
            emit('remove', menu.item.id);
            closeMenu();
          "
        >
          删除
        </button>
      </div>
    </template>
  </main>
</template>
