<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  ports: number[];
}>();

const portStatus = ref<Map<number, boolean>>(new Map());

onMounted(async () => {
  for (const port of props.ports) {
    try {
      const inUse: boolean = await invoke("check_port", { port });
      portStatus.value.set(port, inUse);
    } catch {
      portStatus.value.set(port, false);
    }
  }
});
</script>

<template>
  <div v-if="ports.length > 0" class="flex gap-2">
    <span
      v-for="port in ports"
      :key="port"
      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-mono"
      :class="portStatus.get(port) ? 'bg-green-900/30 text-green-400 border border-green-800/50' : 'bg-gray-800 text-gray-400 border border-gray-700'"
    >
      <span
        class="w-1.5 h-1.5 rounded-full"
        :class="portStatus.get(port) ? 'bg-green-400' : 'bg-gray-600'"
      ></span>
      :{{ port }}
    </span>
  </div>
</template>
