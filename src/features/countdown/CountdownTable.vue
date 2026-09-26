<script setup lang="ts">
import { formatDays } from "./logic";
import type { CountdownRow } from "./useCountdown";
import type { CountdownItem } from "@/bindings";

defineProps<{ rows: CountdownRow[] }>();
const emit = defineEmits<{ edit: [item: CountdownItem]; remove: [id: string] }>();
</script>

<template>
  <main class="min-h-0 flex-1 overflow-y-auto px-2 pb-1">
    <div
      class="grid grid-cols-[1fr_5.5rem_4.5rem_2.75rem] gap-x-2 border-b border-slate-800 px-1 py-1 text-xs text-slate-500"
    >
      <span>名称</span>
      <span class="text-right">目标日期</span>
      <span class="text-right">剩余</span>
      <span></span>
    </div>
    <div v-if="rows.length === 0" class="py-8 text-center text-xs text-slate-500">
      暂无倒计时，在下方添加
    </div>
    <div
      v-for="row in rows"
      :key="row.item.id"
      class="group grid grid-cols-[1fr_5.5rem_4.5rem_2.75rem] items-center gap-x-2 rounded px-1 py-1.5 hover:bg-slate-800/60"
      :class="
        row.state === 'expired'
          ? 'text-red-400/70'
          : row.state === 'soon'
            ? 'text-red-400'
            : 'text-slate-200'
      "
    >
      <span class="truncate text-sm" :title="row.item.note ?? row.item.title">
        {{ row.item.title }}
        <span v-if="row.item.note" class="ml-1 text-[10px] text-slate-500">{{
          row.item.note
        }}</span>
      </span>
      <span class="text-right text-xs text-slate-400">{{ row.item.target_date }}</span>
      <span class="text-right text-xs font-medium">{{ formatDays(row.days) }}</span>
      <span class="flex justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        <button
          class="px-1 text-slate-500 hover:text-slate-100"
          title="编辑"
          @click="emit('edit', row.item)"
        >
          ✎
        </button>
        <button
          class="px-1 text-slate-500 hover:text-red-300"
          title="删除"
          @click="emit('remove', row.item.id)"
        >
          ✕
        </button>
      </span>
    </div>
  </main>
</template>
