<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { formatDays } from "./logic";
import {
  countdownState,
  type ColumnWidths,
  type CountdownRow,
  type EditField,
} from "./useCountdown";
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

let drag: {
  col: keyof ColumnWidths;
  startX: number;
  startW: number;
} | null = null;

function onDown(col: keyof ColumnWidths, e: PointerEvent) {
  drag = { col, startX: e.clientX, startW: local[col] };
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}
function onMove(e: PointerEvent) {
  if (!drag) return;
  local[drag.col] = Math.round(Math.min(MAX, Math.max(MIN, drag.startW + e.clientX - drag.startX)));
}
function onUp() {
  if (!drag) return;
  drag = null;
  emit("resize", { ...local });
}

// —— 表头列定义与窗口自适应（标题栏右键触发）——
// 表头列定义：顺序即列顺序，每列右缘各挂一根拖拽手柄
const HEADER_COLS = [
  { key: "countdown", label: "倒计时" },
  { key: "target", label: "目标日期" },
  { key: "name", label: "名称" },
  { key: "note", label: "备注" },
] as const satisfies readonly { key: keyof ColumnWidths; label: string }[];
const COLUMN_KEYS = HEADER_COLS.map((c) => c.key);
const GAP = 8; // grid gap-x-2
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

// 宽度留白：main 的 px-2 共 16，再加 1px 安全余量。
// 实测 Windows 无边框透明窗口缩小时 setSize 会欠 1px（请求 427 落地 426，放大时不欠，
// 与 DWM 异步调整有关，回读补差读到的是动画中间态、不可靠）。宽度方向零余量，
// 欠 1px 就先挤出横向滚动条（占 8px 高）再连锁挤出纵向滚动条；多请 1px，欠账后正好
// 包住，不欠时也只多 1px，无滚动条且肉眼无感。
const WIN_PAD_X = 16 + 1;
// 高度留白：main 的 pb-1(4)（高度方向实测不欠账，4px 足够，不另加余量）
const WIN_PAD_Y = 4;
const TITLE_BAR_H = 32; // 标题栏 h-8

// 标题栏右键「自适应宽高」：四列收放到内容宽度并持久化，然后把窗口调到刚好容纳整张表。
// 宽高一律量真实 DOM（内层 w-max 容器 + 标题栏/容器的固定留白），不手算行高常量，
// 空态、草稿行、字体变化都自动涵盖。
async function fitWindow() {
  COLUMN_KEYS.forEach((c) => (local[c] = fittedWidth(c)));
  emitResize();
  await nextTick();
  const tableW = innerEl.value?.offsetWidth ?? 0;
  const tableH = innerEl.value?.offsetHeight ?? 0;
  await getCurrentWindow().setSize(
    new LogicalSize(Math.ceil(tableW + WIN_PAD_X), TITLE_BAR_H + tableH + WIN_PAD_Y),
  );
}

defineExpose({ fitWindow });

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
// 内层 w-max 容器：窗口宽高按它的真实渲染尺寸收放
const innerEl = ref<HTMLElement | null>(null);
// 进入新增时记录进入前窗高：退出新增（保存/取消）后恢复到 max(内容高, 进入前高)，
// 不偷走用户手动拉大的高度；取消时草稿行消失，窗口随之收回这一行
let heightBeforeAdd: number | null = null;

// 新增草稿行会多出一行高：窗口不够高时纵向滚动条出现，经典滚动条占布局宽度又连锁
// 挤出横向滚动条。进入新增只长不短，退出时按「进入前高度与当前内容取大」恢复。
async function syncHeightForAdd(active: boolean) {
  await nextTick();
  const tableH = innerEl.value?.offsetHeight ?? 0;
  const win = getCurrentWindow();
  const inner = await win.innerSize();
  const logical = inner.toLogical(await win.scaleFactor());
  const fitH = TITLE_BAR_H + tableH + WIN_PAD_Y;
  if (active) {
    heightBeforeAdd = Math.round(logical.height);
    if (fitH > logical.height) {
      await win.setSize(new LogicalSize(Math.round(logical.width), fitH));
    }
    // 极端情况下（窗口被拖到很矮）仍可能需要滚动，保证草稿行可见
    mainEl.value?.querySelector<HTMLElement>("[data-draft]")?.scrollIntoView({ block: "nearest" });
  } else if (heightBeforeAdd !== null) {
    const target = Math.max(fitH, heightBeforeAdd);
    heightBeforeAdd = null;
    if (Math.abs(target - logical.height) >= 1) {
      await win.setSize(new LogicalSize(Math.round(logical.width), target));
    }
  }
}

// 进入新增时窗口自动增高；聚焦由 EditableRow 按编辑字段自理
watch(editing, async (v) => {
  if (v?.mode === "add") {
    await syncHeightForAdd(true);
    return;
  }
  if (heightBeforeAdd !== null) await syncHeightForAdd(false);
});

function isEditing(id: string) {
  return editing.value?.mode === "edit" && editing.value.id === id;
}

// 行右键菜单：只挂在可编辑的三列上（第一列倒计时只读，不响应右键）。
// 删除走自定义确认对话框（双击误确认，菜单内两步确认已废弃）。
const menu = ref<{ x: number; y: number; item: CountdownItem; field: EditField } | null>(null);
const pendingDelete = ref<CountdownItem | null>(null);

function openMenu(e: MouseEvent, item: CountdownItem, field: EditField) {
  const mw = 96;
  const mh = 76;
  menu.value = {
    x: Math.min(e.clientX, window.innerWidth - mw - 4),
    y: Math.min(e.clientY, window.innerHeight - mh - 4),
    item,
    field,
  };
}
function closeMenu() {
  menu.value = null;
}
function menuEdit() {
  if (menu.value) startEdit(menu.value.item, menu.value.field);
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
    <div ref="innerEl" class="w-max mx-auto">
      <!-- 手柄 -right-2.5 = -(手柄宽 w-3 的一半 6px + gap-x-2 的一半 4px)，
           中心落在它与右邻列的间隙中点（末列右缘落在表格外缘），
           于是每列文字到左右竖线的距离恒等；两侧留白量见 COL_PAD -->
      <div
        class="group/head grid items-center gap-x-2 border-b border-slate-800 px-1 py-1 text-center text-xs text-slate-500"
        :style="gridStyle"
      >
        <span v-for="col in HEADER_COLS" :key="col.key" class="relative" :data-col="col.key">
          <span
            class="group/col absolute top-0 -right-2.5 z-10 flex h-full w-3 cursor-col-resize items-center justify-center"
            title="拖拽调整列宽"
            @pointerdown="onDown(col.key, $event)"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
            ><span
              class="h-4 w-0.5 rounded-full bg-slate-600 opacity-0 transition group-hover/head:opacity-100 group-active/col:opacity-100 group-hover/col:bg-slate-300"
            ></span
          ></span>
          {{ col.label }}
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
        >
          <span class="text-xs font-medium" data-col="countdown">{{ formatDays(row.days) }}</span>
          <span
            class="text-xs"
            data-col="target"
            @contextmenu.prevent="openMenu($event, row.item, 'target')"
            >{{ row.item.target_date }}</span
          >
          <span
            class="truncate text-sm"
            data-col="name"
            :title="row.item.title"
            @contextmenu.prevent="openMenu($event, row.item, 'name')"
            >{{ row.item.title }}</span
          >
          <span
            class="truncate text-sm"
            data-col="note"
            :title="row.item.note ?? ''"
            @contextmenu.prevent="openMenu($event, row.item, 'note')"
          >
            {{ row.item.note }}
          </span>
        </div>
      </template>
      <!-- 新增草稿行：固定显示在最后，保存后随列表按倒计时重排 -->
      <EditableRow v-if="editing?.mode === 'add'" :style="gridStyle" data-draft />
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
