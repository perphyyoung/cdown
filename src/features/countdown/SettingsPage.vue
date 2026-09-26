<script setup lang="ts">
import { onMounted } from "vue";
import { emit } from "@tauri-apps/api/event";
import SettingsRow from "./SettingsRow.vue";
import { countdownState } from "./useCountdown";

// 独立设置窗口的根视图：与主窗口各自持有状态副本，保存后广播刷新
const { settings, ready, error, reload, saveSettings } = countdownState();

onMounted(() => {
  void reload();
});

async function onThreshold(days: number) {
  try {
    await saveSettings({ redThresholdDays: days });
    await emit("settings-changed", null);
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
    <p v-if="error" class="mt-2 rounded bg-red-900/50 px-2 py-1 text-xs text-red-200">
      {{ error }}
    </p>
  </div>
</template>
