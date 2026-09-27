<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { emit } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { commands } from "@/bindings";
import { log } from "@/utils/logger";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import SettingsRow from "./SettingsRow.vue";
import { countdownState } from "./useCountdown";
import type { UrgencyLevel } from "./logic";

// 独立设置窗口的根视图：与主窗口各自持有状态副本，保存后广播刷新
const { settings, ready, error, reload, saveSettings } = countdownState();

// 版本单一事实源 package.json（vite define 注入）
const appVersion = __APP_VERSION__;

const msg = ref("");
// 自定义确认对话框（WebView2 下 window.confirm/alert 不可用，见 design.md）
const confirmBox = ref<{
  title: string;
  message: string;
  confirmText: string;
  danger: boolean;
  action: () => void;
} | null>(null);

function askConfirm(
  message: string,
  action: () => void,
  opts: { title?: string; confirmText?: string; danger?: boolean } = {},
) {
  confirmBox.value = {
    title: opts.title ?? "确认",
    message,
    confirmText: opts.confirmText ?? "确定",
    danger: opts.danger ?? true,
    action,
  };
}

function onConfirm() {
  const action = confirmBox.value?.action;
  confirmBox.value = null;
  action?.();
}
const rootEl = ref<HTMLElement | null>(null);
const win = getCurrentWindow();

// 窗口高度自适应内容：量内容实际高度后 setSize（宽度保持不变）
async function fitHeight() {
  await nextTick();
  if (!rootEl.value) return;
  const content = rootEl.value.getBoundingClientRect().height;
  const inner = await win.innerSize();
  const width = inner.toLogical(await win.scaleFactor()).width;
  await win.setSize(new LogicalSize(Math.round(width), Math.ceil(content)));
}

watch([ready, msg, error, settings], () => void fitHeight());

onMounted(() => {
  void reload();
  void fitHeight();
});

function fileStamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(
    d.getMinutes(),
  )}${p(d.getSeconds())}`;
}

async function onThresholdChange(days: number) {
  try {
    await saveSettings({ levels: settings.value.levels });
    await emit("settings-changed", null);
  } catch (e) {
    error.value = String(e);
  }
}

type ExportKind = "items" | "settings";

async function onExport(kind: ExportKind) {
  log.info("[export] 开始导出", kind);
  try {
    const path = await save({
      defaultPath: `cdown-${kind}-${fileStamp()}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    if (kind === "items") await commands.exportItems(path);
    else await commands.exportSettings(path);
    msg.value = "已导出";
    error.value = "";
    log.info("[export] 导出完成", kind);
  } catch (e) {
    error.value = String(e);
  }
}

async function doImport(kind: ExportKind, path: string) {
  if (kind === "items") {
    const result = await commands.importItems(path);
    msg.value = `已导入 ${result.items} 条倒计时`;
  } else {
    await commands.importSettings(path);
    msg.value = "设置已导入";
  }
  log.info("[import] 导入完成", kind);
  await reload();
  await emit("settings-changed", null);
  error.value = "";
}

async function onImport(kind: ExportKind) {
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    const what = kind === "items" ? "全部倒计时数据" : "全部设置";
    askConfirm(`导入将覆盖现有${what}。`, () => void doImport(kind, path), {
      confirmText: "导入",
    });
  } catch (e) {
    log.error("[import] 导入失败", String(e));
    error.value = String(e);
  }
}

const MAX_LEVELS = 6;
// 与后端 default_levels 保持一致
const DEFAULT_LEVELS: UrgencyLevel[] = [
  { thresholdDays: 7, color: "#a78bfa" },
  { thresholdDays: 3, color: "#facc15" },
  { thresholdDays: 0, color: "#fb923c" },
];
// 展示升序（天数小的最紧急，放最上面）；settings.levels 为降序
const levelsAsc = computed(() => [...settings.value.levels].reverse());

// 分级编辑：每次修改整表提交，后端负责归一化（去重/排序/颜色回落）
async function setLevels(next: UrgencyLevel[]) {
  try {
    await saveSettings({ levels: next });
    await emit("settings-changed", null);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

// 升序展示索引 → 降序存储索引
function toStoreIndex(displayIndex: number): number {
  return settings.value.levels.length - 1 - displayIndex;
}

function updateLevel(displayIndex: number, patch: Partial<UrgencyLevel>) {
  const index = toStoreIndex(displayIndex);
  void setLevels(settings.value.levels.map((l, i) => (i === index ? { ...l, ...patch } : l)));
}

// 破坏性操作（删除/重置/导入覆盖）必须二次确认（见根目录 design.md）
function removeLevel(displayIndex: number) {
  const index = toStoreIndex(displayIndex);
  const lvl = settings.value.levels[index];
  askConfirm(
    `删除该级（${lvl?.thresholdDays} 天内）？删除后不可恢复。`,
    () => void setLevels(settings.value.levels.filter((_, i) => i !== index)),
    { confirmText: "删除" },
  );
}

function addLevel() {
  const cur = settings.value.levels;
  if (cur.length >= MAX_LEVELS) return;
  const nextThreshold = cur.length ? Math.max(...cur.map((l) => l.thresholdDays)) + 4 : 7;
  void setLevels([...cur, { thresholdDays: nextThreshold, color: "#a78bfa" }]);
}

function resetLevels() {
  askConfirm(
    "重置将恢复为默认的三级（7 天紫 / 3 天黄 / 0 天橙），丢弃现有分级。",
    () => void setLevels(DEFAULT_LEVELS.map((l) => ({ ...l }))),
    { confirmText: "重置" },
  );
}

function onLevelThreshold(index: number, value: number) {
  updateLevel(index, { thresholdDays: value });
}

function onLevelColor(index: number, e: Event) {
  updateLevel(index, { color: (e.target as HTMLInputElement).value });
}

const btnCls =
  "shrink-0 rounded border border-slate-600 px-3 py-1 text-sm text-slate-200 hover:bg-slate-700";
const btnSmCls =
  "rounded border border-slate-600 px-2 py-0.5 text-xs font-normal text-slate-200 transition-colors hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-40";
</script>

<template>
  <!-- 布局参考 paim SettingsView：每个设置项一行，左栏标题+副标题，右栏控件/按钮，行间分隔线 -->
  <div v-if="ready" ref="rootEl" class="bg-slate-900 text-slate-100">
    <!-- 自绘标题栏（无边框窗口）：设置 | 版本居中 | 关闭 -->
    <header class="relative flex h-8 shrink-0 items-center px-3" data-tauri-drag-region>
      <span class="text-sm text-slate-300">设置</span>
      <span
        class="absolute left-1/2 -translate-x-1/2 text-xs text-slate-500"
        data-tauri-drag-region
      >
        cdown v{{ appVersion }}
      </span>
      <button
        class="ml-auto h-8 w-10 text-slate-500 hover:bg-slate-800 hover:text-slate-100"
        title="关闭"
        @click="getCurrentWindow().close()"
      >
        ✕
      </button>
    </header>
    <div class="p-4">
      <dl class="divide-y divide-slate-700">
        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="flex items-center gap-2 text-slate-300">
              紧急度分级
              <button
                :class="btnSmCls"
                :disabled="settings.levels.length >= MAX_LEVELS"
                @click="addLevel"
              >
                ＋ 添加分级
              </button>
              <button :class="btnSmCls" @click="resetLevels">重置</button>
            </dt>
            <dd class="text-sm text-slate-500">
              剩余天数 ≤ 级别天数时按该级颜色显示（天数小的优先，最紧急在最上面），最多
              {{ MAX_LEVELS }} 级；过期固定红色，清空分级则全部正常色
            </dd>
          </div>
          <div class="flex shrink-0 flex-col items-end gap-1.5">
            <div
              v-for="(lvl, i) in levelsAsc"
              :key="lvl.thresholdDays"
              class="flex items-center gap-1.5"
            >
              <SettingsRow
                :model-value="lvl.thresholdDays"
                :max="365"
                @change="onLevelThreshold(i, $event)"
              />
              <span class="text-xs text-slate-500">天内</span>
              <input
                type="color"
                :value="lvl.color"
                class="h-7 w-10 cursor-pointer rounded bg-slate-800"
                @change="onLevelColor(i, $event)"
              />
              <button
                class="px-1 text-slate-500 hover:text-red-300"
                title="删除该级"
                @click="removeLevel(i)"
              >
                ✕
              </button>
            </div>
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="text-slate-300">倒计时数据</dt>
            <dd class="text-sm text-slate-500">
              导出/导入全部倒计时项；导入为替换语义，只覆盖倒计时，设置不动
            </dd>
          </div>
          <div class="flex shrink-0 gap-2">
            <button :class="btnCls" @click="onExport('items')">导出</button>
            <button :class="btnCls" @click="onImport('items')">导入</button>
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="text-slate-300">设置备份</dt>
            <dd class="text-sm text-slate-500">
              导出/导入紧急度分级与列宽；导入为替换语义，只覆盖设置，倒计时不动
            </dd>
          </div>
          <div class="flex shrink-0 gap-2">
            <button :class="btnCls" @click="onExport('settings')">导出</button>
            <button :class="btnCls" @click="onImport('settings')">导入</button>
          </div>
        </div>
      </dl>

      <p v-if="msg" class="mt-2 rounded bg-emerald-900/50 px-2 py-1 text-xs text-emerald-200">
        {{ msg }}
      </p>
      <p
        v-if="error"
        class="mt-2 flex items-center justify-between gap-2 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200"
      >
        <span class="min-w-0 break-all">{{ error }}</span>
        <button
          class="shrink-0 px-1 text-red-200/70 hover:text-red-100"
          title="关闭"
          @click="error = ''"
        >
          ✕
        </button>
      </p>
    </div>

    <ConfirmDialog
      :open="confirmBox !== null"
      :title="confirmBox?.title"
      :message="confirmBox?.message"
      :confirm-text="confirmBox?.confirmText"
      :danger="confirmBox?.danger"
      @confirm="onConfirm"
      @cancel="confirmBox = null"
    />
  </div>
</template>
