<script setup lang="ts">
import { onMounted, ref } from "vue";
import { emit } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { commands } from "@/bindings";
import SettingsRow from "./SettingsRow.vue";
import { countdownState } from "./useCountdown";

// 独立设置窗口的根视图：与主窗口各自持有状态副本，保存后广播刷新
const { settings, ready, error, reload, saveSettings } = countdownState();

const msg = ref("");

onMounted(() => {
  void reload();
});

function fileStamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(
    d.getMinutes(),
  )}${p(d.getSeconds())}`;
}

async function onThreshold(days: number) {
  try {
    await saveSettings({ redThresholdDays: days });
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
</script>

<template>
  <div v-if="ready" class="h-full overflow-y-auto bg-slate-900 p-3 text-slate-100">
    <SettingsRow :threshold="settings.red_threshold_days" @change="onThreshold" />
    <p class="mt-2 text-[10px] leading-4 text-slate-600">
      倒计时天数 ≤ 阈值时该行标红；列宽在主面板表头拖拽调整，保存后主面板实时生效。
    </p>

    <div class="mt-4 grid grid-cols-2 gap-2">
      <button
        class="rounded bg-slate-700 px-2.5 py-1 text-xs text-slate-100 hover:bg-slate-600"
        @click="onExport('items')"
      >
        导出倒计时
      </button>
      <button
        class="rounded bg-slate-700 px-2.5 py-1 text-xs text-slate-100 hover:bg-slate-600"
        @click="onImport('items')"
      >
        导入倒计时
      </button>
      <button
        class="rounded bg-slate-700 px-2.5 py-1 text-xs text-slate-100 hover:bg-slate-600"
        @click="onExport('settings')"
      >
        导出设置
      </button>
      <button
        class="rounded bg-slate-700 px-2.5 py-1 text-xs text-slate-100 hover:bg-slate-600"
        @click="onImport('settings')"
      >
        导入设置
      </button>
    </div>
    <p class="mt-2 text-[10px] leading-4 text-slate-600">
      倒计时与设置各自独立导出/导入；导入为替换语义，只覆盖对应部分。
    </p>
    <p v-if="msg" class="mt-2 rounded bg-emerald-900/50 px-2 py-1 text-xs text-emerald-200">
      {{ msg }}
    </p>
    <p v-if="error" class="mt-2 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200">
      {{ error }}
    </p>
  </div>
</template>
