<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { emit } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { commands } from "@/bindings";
import { log } from "@/utils/logger";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import SettingsRow from "./SettingsRow.vue";
import SettingsToggle from "./SettingsToggle.vue";
import FontSelect from "./FontSelect.vue";
import { countdownState, DEFAULT_BACKGROUND_COLOR } from "./useCountdown";
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
  void isEnabled()
    .then((on) => (autostart.value = on))
    .catch((e) => (error.value = String(e)));
  void fitHeight();
});

async function onFontFamilyChange(value: string) {
  try {
    await saveSettings({ fontFamily: value });
    await emit("settings-changed", null);
    msg.value = "字体已更新";
  } catch (e) {
    error.value = String(e);
  }
}

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

// —— 全局热键：点按钮进入录制态，捕获组合键后交后端校验并注册 ——
const recording = ref(false);
// 与后端 domain::model::DEFAULT_HOTKEY 保持一致（同 DEFAULT_LEVELS 的先例）
const DEFAULT_HOTKEY = "Ctrl+Alt+C";
const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

// 拼 accelerator：修饰键顺序 Ctrl/Alt/Shift/Super，主键用 e.code（KeyC/Digit1/F8…，与后端 Code 名一致）
function acceleratorOf(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null; // 只按下修饰键，继续等主键
  const mods = [
    e.ctrlKey && "Ctrl",
    e.altKey && "Alt",
    e.shiftKey && "Shift",
    e.metaKey && "Super",
  ].filter(Boolean) as string[];
  if (!mods.length) return null; // 无修饰键不接受（与后端 rule 一致）
  return [...mods, e.code].join("+");
}

function onRecordKeydown(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    recording.value = false;
    return;
  }
  if (e.key === "Backspace" || e.key === "Delete") {
    closeHotkey(); // 清空 = 关闭热键，走同一确认
    return;
  }
  const acc = acceleratorOf(e);
  if (acc) void setHotkey(acc);
}

// 仅在录制期间挂 document 级监听，并成对移除（离开设置页不留残留监听）
watch(recording, (on) => {
  if (on) document.addEventListener("keydown", onRecordKeydown, true);
  else document.removeEventListener("keydown", onRecordKeydown, true);
});
onUnmounted(() => document.removeEventListener("keydown", onRecordKeydown, true));

async function setHotkey(hotkey: string | null) {
  recording.value = false;
  const wasEnabled = settings.value.hotkey !== null;
  try {
    await saveSettings({ hotkey });
    await emit("settings-changed", null);
    msg.value = hotkey
      ? hotkey === DEFAULT_HOTKEY
        ? wasEnabled
          ? `全局热键已重置为 ${DEFAULT_HOTKEY}`
          : `全局热键已启用（${DEFAULT_HOTKEY}）`
        : `全局热键已设为 ${settings.value.hotkey}`
      : "全局热键已关闭";
    error.value = "";
  } catch (e) {
    // 被其它程序占用 / 无法识别都走这里，保留原键
    error.value = String(e);
  }
}

// 关闭与重置都是破坏性操作，二次确认（与分级删除/重置一致）
function closeHotkey() {
  // 从录制态进入确认前先退出，避免录制监听器吞掉确认框的 Esc/Enter
  recording.value = false;
  askConfirm("关闭后全局热键立即失效，需重新录制才能再次启用。", () => void setHotkey(null), {
    confirmText: "关闭",
  });
}

// 未启用时点「重置」= 用默认键启用；自定义键时 = 恢复默认键
function resetHotkey() {
  const enabled = settings.value.hotkey !== null;
  askConfirm(
    enabled ? `重置为默认热键 ${DEFAULT_HOTKEY}？` : `启用默认热键 ${DEFAULT_HOTKEY}？`,
    () => void setHotkey(DEFAULT_HOTKEY),
    { confirmText: enabled ? "重置" : "启用" },
  );
}

// —— 开机自启：注册表 Run 项是唯一状态源（autostart 插件直读直写），
// 不进 Settings/导出导入；先行切换视觉，失败回滚 ——
const autostart = ref(false);

async function onAutostartChange(next: boolean) {
  autostart.value = next;
  try {
    if (next) await enable();
    else await disable();
    msg.value = next ? "已开启开机自启" : "已关闭开机自启";
    error.value = "";
    log.info("[autostart] 切换为", next);
  } catch (e) {
    autostart.value = !next;
    error.value = String(e);
  }
}

// —— 主界面背景色：input 实时预览（不落盘），取色器关闭才提交 ——
// （MDN 对 <input type="color"> 的标准分工：input 每次颜色变化触发，change 关闭取色器时触发）
const previewColor = ref<string | null>(null);
// 撤销目标：最近一次落盘前的颜色；null 表示无可撤销，用完即失效（单步撤销）
const undoColor = ref<string | null>(null);
// 会话中被放弃的颜色（会话中点了撤销/重置）：取色器关闭时不再提交它；null 表示未放弃
const discardedColor = ref<string | null>(null);
const colorInput = ref<HTMLInputElement | null>(null);

/**
 * 色块显示与撤销判断的唯一依据：预览优先。
 * 色块的 :value 必须绑它而不能绑 settings.backgroundColor——拖动期间每次重渲染，
 * Vue 都会拿 DOM 里已更新的新色与仍是旧色的 vnode 值比较，把色块打回旧色。
 */
const shownColor = computed(() => previewColor.value ?? settings.value.backgroundColor);
// 取色器里改了色但还没关（未落盘）也算一次可撤销的改动，故改色瞬间就显示撤销
const pendingChange = computed(
  () => previewColor.value !== null && previewColor.value !== settings.value.backgroundColor,
);
const canUndo = computed(
  () => pendingChange.value || (undoColor.value !== null && shownColor.value !== undoColor.value),
);

function onBackgroundColorInput(color: string) {
  previewColor.value = color;
  void emit("background-preview", color);
}

async function persistBackgroundColor(color: string): Promise<boolean> {
  try {
    await saveSettings({ backgroundColor: color });
    await emit("settings-changed", null);
    error.value = "";
    return true;
  } catch (e) {
    error.value = String(e);
    return false;
  }
}

/** 放弃当前取色会话：撤回预览，取色器关闭时不再提交它的颜色 */
function discardColorSession() {
  discardedColor.value = previewColor.value;
  previewColor.value = null;
  void emit("background-preview", settings.value.backgroundColor);
}

/** 取色器关闭收口（change）：一律以输入框的 DOM 实际值为准，不依赖拖动期间有过 input */
async function closeColorSession() {
  const picked = colorInput.value?.value;
  const before = settings.value.backgroundColor;
  const previewed = previewColor.value;
  previewColor.value = null;
  if (discardedColor.value !== null) {
    // 会话已被撤销/重置放弃：取色器关闭时会把颜色重新推给 input（并再次广播预览），
    // 这里统一把主窗口拉回已保存色，色块由 shownColor 自动跟回
    discardedColor.value = null;
    void emit("background-preview", before);
    return;
  }
  if (!picked || picked === before) {
    // 无变化（对 DOM 而言值没动）：把可能残留的预览退回已保存色
    if (previewed !== null) void emit("background-preview", before);
    return;
  }
  if (await persistBackgroundColor(picked)) {
    undoColor.value = before; // 整段拖动记为一步
  } else {
    void emit("background-preview", before);
  }
}

// 重置：撤销目标是重置前的颜色。会话中点击则先放弃未落盘的取色
async function onBackgroundColorReset() {
  const before = settings.value.backgroundColor;
  if (previewColor.value !== null) discardColorSession();
  if (before === DEFAULT_BACKGROUND_COLOR) return;
  if (await persistBackgroundColor(DEFAULT_BACKGROUND_COLOR)) undoColor.value = before;
}

// 撤销：会话中撤回预览（不落盘），会话外回到 undoColor，用完即失效
async function onBackgroundColorUndo() {
  if (pendingChange.value) {
    discardColorSession();
    return;
  }
  const back = undoColor.value;
  previewColor.value = null;
  if (!back || back === settings.value.backgroundColor) return;
  undoColor.value = null;
  // 色块由 shownColor 驱动（:value 绑它），无需手写 el.value
  if (!(await persistBackgroundColor(back))) {
    void emit("background-preview", settings.value.backgroundColor);
  }
}

// 背景透明度：input 即保存广播（设置窗口与主窗口状态隔离，无法本地预览不落盘），
// 写盘是小 JSON，与分级编辑「每次修改整表提交」的先例一致
async function onOpacityChange(v: number) {
  try {
    await saveSettings({ backgroundOpacity: v });
    await emit("settings-changed", null);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
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
            <dt class="text-slate-300">字体家族</dt>
            <dd class="text-sm text-slate-500">
              候选为本机已安装字体，支持中英文搜索；中文名映射见数据目录下 font-family-map.toml
            </dd>
          </div>
          <FontSelect :model-value="settings.fontFamily" @update:model-value="onFontFamilyChange" />
        </div>

        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="text-slate-300">主界面背景色</dt>
            <dd class="text-sm text-slate-500">
              点击取色器自定义，拖动时主窗口实时预览，关闭后保存
            </dd>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <button
              v-if="canUndo"
              :class="btnSmCls"
              title="撤销本次修改，恢复上一次的颜色"
              @click="onBackgroundColorUndo()"
            >
              撤销
            </button>
            <input
              ref="colorInput"
              type="color"
              :value="shownColor"
              class="h-7 w-10 cursor-pointer rounded bg-slate-800"
              @input="onBackgroundColorInput(($event.target as HTMLInputElement).value)"
              @change="closeColorSession()"
            />
            <button
              v-if="shownColor !== DEFAULT_BACKGROUND_COLOR"
              :class="btnSmCls"
              title="恢复默认背景色"
              @click="onBackgroundColorReset()"
            >
              重置
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="text-slate-300">背景透明度</dt>
            <dd class="text-sm text-slate-500">
              主窗口背景不透明程度，值越小透出桌面越多（下限 10%）
            </dd>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <input
              type="range"
              min="10"
              max="100"
              :value="settings.backgroundOpacity"
              class="w-32 accent-slate-400"
              @input="onOpacityChange(Number(($event.target as HTMLInputElement).value))"
            />
            <span class="w-10 text-right text-sm text-slate-400">
              {{ settings.backgroundOpacity }}%
            </span>
          </div>
        </div>

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
            <dt class="text-slate-300">全局热键</dt>
            <dd class="text-sm text-slate-500">
              切换主窗口的显示；点击可自定义热键，需含 Ctrl / Alt / Shift
              中至少一个，被其它程序占用会在下方提示
            </dd>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <button
              :class="[btnCls, recording ? 'ring-1 ring-slate-500' : '']"
              :title="
                recording
                  ? 'Esc 取消，Backspace 关闭热键'
                  : settings.hotkey
                    ? '点击后按下组合键'
                    : `点击启用默认热键 ${DEFAULT_HOTKEY}`
              "
              @click="settings.hotkey === null ? resetHotkey() : (recording = !recording)"
            >
              {{ recording ? "请按组合键…" : (settings.hotkey ?? "未启用") }}
            </button>
            <button
              v-if="settings.hotkey !== DEFAULT_HOTKEY && !recording"
              :class="btnSmCls"
              :title="settings.hotkey ? '恢复为默认热键' : '启用默认热键'"
              @click="resetHotkey"
            >
              重置
            </button>
            <button
              v-if="settings.hotkey && !recording"
              :class="btnSmCls"
              title="关闭全局热键"
              @click="closeHotkey"
            >
              关闭
            </button>
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="text-slate-300">开机自启</dt>
            <dd class="text-sm text-slate-500">登录 Windows 后自动运行 cdown</dd>
          </div>
          <div class="shrink-0">
            <SettingsToggle :model-value="autostart" @change="onAutostartChange" />
          </div>
        </div>

        <div class="flex items-center justify-between gap-3 py-3">
          <div class="min-w-0">
            <dt class="text-slate-300">设置备份</dt>
            <dd class="text-sm text-slate-500">
              导出/导入紧急度分级、列宽与全局热键；导入为替换语义，只覆盖设置，倒计时不动
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
