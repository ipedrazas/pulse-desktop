<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const props = defineProps<{
  projectId: string;
  service: {
    name: string;
    cwd: string | null;
    type: string | null;
    dev: { command: string; ports: number[] | null } | null;
    health: { url: string } | null;
  };
  rootPath: string;
}>();

const processId = ref<number | null>(null);
const running = ref(false);
const healthy = ref<boolean | null>(null);
const logs = ref<string[]>([]);
const showLogs = ref(false);
let unlisten: UnlistenFn | null = null;
let healthInterval: ReturnType<typeof setInterval> | null = null;

const ports = computed(() => props.service.dev?.ports || []);

onMounted(async () => {
  unlisten = await listen<{ service_name: string; line: string; stream: string }>(
    "service:log",
    (event) => {
      if (event.payload.service_name === props.service.name) {
        const prefix = event.payload.stream === "stderr" ? "[ERR] " : "";
        logs.value.push(prefix + event.payload.line);
        if (logs.value.length > 500) logs.value.shift();
      }
    }
  );
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (healthInterval) clearInterval(healthInterval);
});

async function start() {
  if (!props.service.dev) return;
  const cwd = props.service.cwd
    ? `${props.rootPath}/${props.service.cwd}`
    : props.rootPath;

  try {
    processId.value = await invoke("start_service", {
      projectId: props.projectId,
      serviceName: props.service.name,
      command: props.service.dev.command,
      cwd,
      ports: ports.value,
    });
    running.value = true;
    logs.value = [];

    // Start health polling
    if (props.service.health) {
      healthInterval = setInterval(checkHealth, 5000);
      setTimeout(checkHealth, 2000);
    }
  } catch (e) {
    logs.value.push(`[ERR] Failed to start: ${e}`);
  }
}

async function stop() {
  if (processId.value === null) return;
  await invoke("stop_service", { processId: processId.value });
  running.value = false;
  healthy.value = null;
  if (healthInterval) {
    clearInterval(healthInterval);
    healthInterval = null;
  }
}

async function checkHealth() {
  if (!props.service.health) return;
  try {
    healthy.value = await invoke("check_service_health", {
      url: props.service.health.url,
    });
  } catch {
    healthy.value = false;
  }
}
</script>

<template>
  <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center gap-3">
        <div
          class="w-2 h-2 rounded-full"
          :class="running ? (healthy === true ? 'bg-green-400' : healthy === false ? 'bg-yellow-400' : 'bg-blue-400') : 'bg-gray-600'"
        ></div>
        <h3 class="font-medium text-white">{{ service.name }}</h3>
        <span v-if="service.type" class="text-xs text-gray-600">
          {{ service.type }}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="showLogs !== undefined"
          @click="showLogs = !showLogs"
          class="text-xs text-gray-500 hover:text-gray-300 transition-colors"
        >
          {{ showLogs ? "Hide Logs" : "Logs" }}
        </button>
        <button
          v-if="!running && service.dev"
          @click="start"
          class="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-xs rounded transition-colors"
        >
          Start
        </button>
        <button
          v-if="running"
          @click="stop"
          class="px-3 py-1 bg-red-600 hover:bg-red-700 text-white text-xs rounded transition-colors"
        >
          Stop
        </button>
      </div>
    </div>

    <!-- Info -->
    <div class="flex gap-4 text-xs text-gray-500">
      <span v-if="service.dev">
        <span class="text-gray-600">cmd:</span>
        <code class="text-gray-400 ml-1">{{ service.dev.command }}</code>
      </span>
      <span v-if="ports.length > 0">
        <span class="text-gray-600">ports:</span>
        <span class="text-gray-400 ml-1">{{ ports.join(", ") }}</span>
      </span>
      <span v-if="service.health">
        <span class="text-gray-600">health:</span>
        <span class="ml-1" :class="healthy === true ? 'text-green-400' : healthy === false ? 'text-red-400' : 'text-gray-400'">
          {{ healthy === true ? "healthy" : healthy === false ? "unhealthy" : "unknown" }}
        </span>
      </span>
    </div>

    <!-- Logs -->
    <div
      v-if="showLogs && logs.length > 0"
      class="mt-3 bg-gray-950 rounded p-2 max-h-48 overflow-y-auto font-mono text-xs"
    >
      <div
        v-for="(line, i) in logs"
        :key="i"
        :class="line.startsWith('[ERR]') ? 'text-red-400' : 'text-gray-400'"
      >
        {{ line }}
      </div>
    </div>
  </div>
</template>
