<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { emit } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { commands } from "@/bindings";
import SettingsRow from "./SettingsRow.vue";
import { countdownState } from "./useCountdown";
import type { UrgencyLevel } from "./logic";

// 独立设置窗口的根视图：与主窗口各自持有状态副本，保存后广播刷新
const { settings, ready, error, reload, saveSettings } = countdownState();

const msg = ref("");
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
  } catch (e) {
    error.value = String(e);
  }
}

async function onImport(kind: ExportKind) {
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    const what = kind === "items" ? "全部倒计时数据" : "全部设置";
    if (!window.confirm(`导入将覆盖现有${what}，继续？`)) return;
    if (kind === "items") {
      const result = await commands.importItems(path);
      msg.value = `已导入 ${result.items} 条倒计时`;
    } else {
      await commands.importSettings(path);
      msg.value = "设置已导入";
    }
    await reload();
    await emit("settings-changed", null);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

const MAX_LEVELS = 6;

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

function updateLevel(index: number, patch: Partial<UrgencyLevel>) {
  void setLevels(settings.value.levels.map((l, i) => (i === index ? { ...l, ...patch } : l)));
}

function removeLevel(index: number) {
  void setLevels(settings.value.levels.filter((_, i) => i !== index));
}

function addLevel() {
  const cur = settings.value.levels;
  if (cur.length >= MAX_LEVELS) return;
  const nextThreshold = cur.length ? Math.max(...cur.map((l) => l.thresholdDays)) + 4 : 7;
  void setLevels([...cur, { thresholdDays: nextThreshold, color: "#a78bfa" }]);
}

function onLevelThreshold(index: number, value: number) {
  updateLevel(index, { thresholdDays: value });
}

function onLevelColor(index: number, e: Event) {
  updateLevel(index, { color: (e.target as HTMLInputElement).value });
}

const btnCls =
  "shrink-0 rounded border border-slate-600 px-3 py-1 text-sm text-slate-200 hover:bg-slate-700";
</script>

<template>
  <!-- 布局参考 paim SettingsView：每个设置项一行，左栏标题+副标题，右栏控件/按钮，行间分隔线 -->
  <div v-if="ready" ref="rootEl" class="bg-slate-900 p-4 text-slate-100">
    <dl class="divide-y divide-slate-700">
      <div class="flex items-center justify-between gap-3 py-3">
        <div class="min-w-0">
          <dt class="text-slate-300">紧急度分级</dt>
          <dd class="text-sm text-slate-500">
            剩余天数 ≤ 级别天数时按该级颜色显示（降序生效），最多
            {{ MAX_LEVELS }} 级；过期固定红色，清空分级则全部正常色
          </dd>
        </div>
        <div class="flex shrink-0 flex-col items-end gap-1.5">
          <div v-for="(lvl, i) in settings.levels" :key="i" class="flex items-center gap-1.5">
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
          <button
            :class="btnCls"
            :disabled="settings.levels.length >= MAX_LEVELS"
            @click="addLevel"
          >
            ＋ 添加分级
          </button>
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
    <p v-if="error" class="mt-2 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200">
      {{ error }}
    </p>
  </div>
</template>
