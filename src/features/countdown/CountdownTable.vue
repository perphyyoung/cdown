<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { log } from "@/utils/logger";
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

// inverted：左缘手柄，拖拽方向与宽度变化相反（向左拖 = 加宽）
let drag: {
  col: keyof ColumnWidths;
  startX: number;
  startW: number;
  inverted?: boolean;
} | null = null;

function onDown(col: keyof ColumnWidths, e: PointerEvent, inverted = false) {
  drag = { col, startX: e.clientX, startW: local[col], inverted };
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}
function onMove(e: PointerEvent) {
  if (!drag) return;
  const delta = drag.inverted ? drag.startX - e.clientX : e.clientX - drag.startX;
  local[drag.col] = Math.round(Math.min(MAX, Math.max(MIN, drag.startW + delta)));
}
function onUp() {
  if (!drag) return;
  drag = null;
  emit("resize", { ...local });
}

// —— 列宽自适应（表头右键）——
const COLUMN_KEYS = ["countdown", "target", "name", "note"] as const;
const GAP = 8; // grid gap-x-2
const ROW_PAD = 8; // 行 px-1
const MAIN_PAD = 16; // 表格容器 px-2
// 最长文字与左右竖线的间隙：所有列共用同一个值，改这里即可整体调整留白。
// 竖线画在列间隙中点，故可见列宽 = 列宽 + GAP，列宽 = 最长文字 + 2 * COL_PAD - GAP。
const COL_PAD = 12;

function emitResize() {
  emit("resize", { ...local });
}

function measureColumn(col: keyof ColumnWidths): number {
  let max = 0;
  // scrollWidth 在盒子比内容宽时返回盒子自身宽度（导致只能加宽不能收窄），
  // 故临时置为 max-content 量取文本自然宽度，量完立刻还原
  mainEl.value?.querySelectorAll<HTMLElement>(`[data-col="${col}"]`).forEach((el) => {
    const prev = el.style.width;
    el.style.width = "max-content";
    max = Math.max(max, el.getBoundingClientRect().width);
    el.style.width = prev;
  });
  return max;
}

// 最长内容 + 两侧留白（各 COL_PAD），再夹到拖拽同款上下限
function fittedWidth(col: keyof ColumnWidths): number {
  return Math.min(MAX, Math.max(MIN, measureColumn(col) + 2 * COL_PAD - GAP));
}

function fitColumn(col: keyof ColumnWidths) {
  local[col] = fittedWidth(col);
  emitResize();
}

function fitAllColumns() {
  COLUMN_KEYS.forEach((c) => (local[c] = fittedWidth(c)));
  emitResize();
  fitWindowWidthSafe();
}

// 全部列自适应时，把主窗口宽度也调到刚好容纳整张表
async function fitWindowWidth() {
  const total = COLUMN_KEYS.reduce((sum, c) => sum + local[c], 0) + GAP * 3 + ROW_PAD + MAIN_PAD;
  const win = getCurrentWindow();
  // setSize 设置的是 inner 尺寸，读取也必须用 innerSize：outer 含隐藏边框差值
  // （无边框带阴影窗口），混用会让高度每次点击棘轮式增长
  const inner = await win.innerSize();
  const logical = inner.toLogical(await win.scaleFactor());
  if (Math.abs(logical.width - total) < 1) return;
  await win.setSize(new LogicalSize(Math.ceil(total), Math.round(logical.height)));
}

function fitWindowWidthSafe() {
  fitWindowWidth().catch((e) => log.error("[fit-all] 调整窗口宽度失败", String(e)));
}

// 表头右键菜单
const headerMenu = ref<{ x: number; y: number; col: keyof ColumnWidths } | null>(null);

function openHeaderMenu(e: MouseEvent, col: keyof ColumnWidths) {
  const mw = 168;
  const mh = 76;
  headerMenu.value = {
    x: Math.min(e.clientX, window.innerWidth - mw - 4),
    y: Math.min(e.clientY, window.innerHeight - mh - 4),
    col,
  };
}
function closeHeaderMenu() {
  headerMenu.value = null;
}
function menuFitColumn() {
  const col = headerMenu.value?.col;
  closeHeaderMenu();
  if (col) fitColumn(col);
}
function menuFitAll() {
  closeHeaderMenu();
  fitAllColumns();
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
    <!-- w-max 收缩为表格自然宽度 + mx-auto 左右居中：拖拽列宽时两侧间距始终相等；
         总宽超出窗口时 margin auto 归零、从左溢出滚动，行为与占满时一致 -->
    <div class="w-max mx-auto">
      <!-- 手柄 -right-2.5 / -left-2.5 = -(手柄宽 w-3 的一半 6px + gap-x-2 的一半 4px)，
           中心落在它分隔的列间隙中点（首尾两根落在表格外缘），
           于是每列文字到左右竖线的距离恒等；两侧留白量见 COL_PAD -->
      <div
        class="grid items-center gap-x-2 border-b border-slate-800 px-1 py-1 text-center text-xs text-slate-500"
        :style="gridStyle"
      >
        <span
          class="relative"
          data-col="countdown"
          @contextmenu.prevent="openHeaderMenu($event, 'countdown')"
        >
          <span
            class="group/col absolute -left-2.5 top-0 z-10 flex h-full w-3 cursor-col-resize items-center justify-center"
            title="拖拽调整列宽"
            @pointerdown="onDown('countdown', $event, true)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
            ><span
              class="h-4 w-0.5 rounded-full bg-slate-600 transition-colors group-hover/col:bg-slate-300"
            ></span
          ></span>
          倒计时
          <span
            class="group/col absolute -right-2.5 top-0 z-10 flex h-full w-3 cursor-col-resize items-center justify-center"
            title="拖拽调整列宽"
            @pointerdown="onDown('countdown', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
            ><span
              class="h-4 w-0.5 rounded-full bg-slate-600 transition-colors group-hover/col:bg-slate-300"
            ></span
          ></span>
        </span>
        <span
          class="relative"
          data-col="target"
          @contextmenu.prevent="openHeaderMenu($event, 'target')"
        >
          目标日期
          <span
            class="group/col absolute -right-2.5 top-0 z-10 flex h-full w-3 cursor-col-resize items-center justify-center"
            title="拖拽调整列宽"
            @pointerdown="onDown('target', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
            ><span
              class="h-4 w-0.5 rounded-full bg-slate-600 transition-colors group-hover/col:bg-slate-300"
            ></span
          ></span>
        </span>
        <span
          class="relative"
          data-col="name"
          @contextmenu.prevent="openHeaderMenu($event, 'name')"
        >
          名称
          <span
            class="group/col absolute -right-2.5 top-0 z-10 flex h-full w-3 cursor-col-resize items-center justify-center"
            title="拖拽调整列宽"
            @pointerdown="onDown('name', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
            ><span
              class="h-4 w-0.5 rounded-full bg-slate-600 transition-colors group-hover/col:bg-slate-300"
            ></span
          ></span>
        </span>
        <span
          class="relative"
          data-col="note"
          @contextmenu.prevent="openHeaderMenu($event, 'note')"
        >
          备注
          <span
            class="group/col absolute -right-2.5 top-0 z-10 flex h-full w-3 cursor-col-resize items-center justify-center"
            title="拖拽调整列宽"
            @pointerdown="onDown('note', $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
            ><span
              class="h-4 w-0.5 rounded-full bg-slate-600 transition-colors group-hover/col:bg-slate-300"
            ></span
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
          class="grid h-9 items-center gap-x-2 rounded px-1 text-center hover:bg-slate-800/60"
          :style="rowStyle(row)"
          :class="
            row.state === 'expired'
              ? 'text-red-400 line-through'
              : row.state === 'level'
                ? ''
                : 'text-slate-200'
          "
          @contextmenu.prevent="openMenu($event, row.item)"
        >
          <span class="text-xs font-medium" data-col="countdown">{{ formatDays(row.days) }}</span>
          <span class="text-xs" data-col="target">{{ row.item.target_date }}</span>
          <span class="truncate text-sm" data-col="name" :title="row.item.title">{{
            row.item.title
          }}</span>
          <span class="truncate text-sm" data-col="note" :title="row.item.note ?? ''">
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

    <template v-if="headerMenu">
      <!-- 透明遮罩：点击/右键任意处关闭菜单 -->
      <div
        class="fixed inset-0 z-40"
        @pointerdown="closeHeaderMenu"
        @contextmenu.prevent="closeHeaderMenu"
      />
      <div
        class="fixed z-50 min-w-[168px] rounded bg-slate-800 py-1 text-xs shadow-xl shadow-black/40 ring-1 ring-slate-700"
        :style="{ left: `${headerMenu.x}px`, top: `${headerMenu.y}px` }"
      >
        <button
          class="block w-full px-3 py-1.5 text-left text-slate-200 hover:bg-slate-700"
          @click="menuFitColumn()"
        >
          单列自适应
        </button>
        <button
          class="block w-full px-3 py-1.5 text-left text-slate-200 hover:bg-slate-700"
          @click="menuFitAll()"
        >
          全部列自适应
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
