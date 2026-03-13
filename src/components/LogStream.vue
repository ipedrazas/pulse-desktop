<script setup lang="ts">
import { ref, watch, nextTick } from "vue";

const props = defineProps<{ lines: string[] }>();
const container = ref<HTMLElement | null>(null);

watch(
  () => props.lines.length,
  async () => {
    await nextTick();
    if (container.value) {
      container.value.scrollTop = container.value.scrollHeight;
    }
  }
);
</script>

<template>
  <div class="bg-gray-950 rounded-lg border border-gray-800 overflow-hidden">
    <div class="px-3 py-1.5 border-b border-gray-800 text-xs text-gray-500">
      Output
    </div>
    <div
      ref="container"
      class="p-3 max-h-80 overflow-y-auto font-mono text-xs leading-5"
    >
      <div
        v-for="(line, i) in lines"
        :key="i"
        :class="line.startsWith('[ERR]') ? 'text-red-400' : 'text-gray-400'"
      >
        {{ line }}
      </div>
      <div v-if="lines.length === 0" class="text-gray-600">
        Waiting for output...
      </div>
    </div>
  </div>
</template>
