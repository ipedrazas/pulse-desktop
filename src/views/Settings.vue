<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useProjectsStore } from "../stores/projects";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);
const config = computed(() => projectsStore.pulseConfig);

interface WatcherStatus {
  id: string;
  title: string;
  enabled: boolean;
  state: string;
  last_run_at: string | null;
  last_exit_code: number | null;
}

const watcherStatuses = ref<WatcherStatus[]>([]);

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await loadWatcherStatuses();

  listen<{ watcher_id: string; state: string; exit_code?: number }>(
    "watcher:status",
    (event) => {
      const ws = watcherStatuses.value.find(
        (w) => `${props.id}:${w.id}` === event.payload.watcher_id
      );
      if (ws) {
        ws.state = event.payload.state;
        if (event.payload.exit_code !== undefined) {
          ws.last_exit_code = event.payload.exit_code;
        }
      }
    }
  );
});

async function loadWatcherStatuses() {
  if (!project.value) return;
  watcherStatuses.value = await invoke("get_watcher_statuses", {
    projectId: project.value.id,
  });

  // If no statuses from manager, build from config
  if (watcherStatuses.value.length === 0 && config.value?.watchers) {
    watcherStatuses.value = config.value.watchers.map((w) => ({
      id: w.id,
      title: w.title,
      enabled: w.enabled ?? false,
      state: "idle",
      last_run_at: null,
      last_exit_code: null,
    }));
  }
}

async function toggleWatcher(watcherId: string, enabled: boolean) {
  if (!project.value) return;
  await invoke("set_watcher_enabled", {
    projectId: project.value.id,
    watcherId,
    enabled,
  });
  const ws = watcherStatuses.value.find((w) => w.id === watcherId);
  if (ws) {
    ws.enabled = enabled;
    if (!enabled) ws.state = "idle";
  }
}

function stateColor(state: string) {
  switch (state) {
    case "pass":
      return "bg-green-400";
    case "fail":
      return "bg-red-400";
    case "running":
      return "bg-blue-400";
    default:
      return "bg-gray-600";
  }
}
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold text-white">Settings</h1>

    <!-- Watchers section -->
    <div>
      <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
        Background Watchers
      </h2>

      <div v-if="watcherStatuses.length === 0" class="text-sm text-gray-600">
        No watchers defined in .pulse.yaml
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="ws in watcherStatuses"
          :key="ws.id"
          class="flex items-center justify-between p-4 bg-gray-900 rounded-lg border border-gray-800"
        >
          <div class="flex items-center gap-3">
            <div
              class="w-2 h-2 rounded-full"
              :class="stateColor(ws.state)"
            ></div>
            <div>
              <div class="text-sm text-white font-medium">{{ ws.title }}</div>
              <div class="text-xs text-gray-500 mt-0.5">
                <span class="capitalize">{{ ws.state }}</span>
                <span v-if="ws.last_run_at" class="ml-2">
                  Last: {{ ws.last_run_at }}
                </span>
                <span v-if="ws.last_exit_code !== null" class="ml-2">
                  Exit: {{ ws.last_exit_code }}
                </span>
              </div>
            </div>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              :checked="ws.enabled"
              @change="toggleWatcher(ws.id, !ws.enabled)"
              class="sr-only peer"
            />
            <div class="w-9 h-5 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-gray-400 after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600 peer-checked:after:bg-white"></div>
          </label>
        </div>
      </div>
    </div>

    <!-- Project info -->
    <div v-if="project">
      <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
        Project Info
      </h2>
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 space-y-2 text-sm">
        <div class="flex justify-between">
          <span class="text-gray-500">ID</span>
          <span class="text-gray-300 font-mono text-xs">{{ project.id }}</span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-500">Path</span>
          <span class="text-gray-300 text-xs">{{ project.root_path }}</span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-500">Type</span>
          <span class="text-gray-300">{{ project.project_type || "Unknown" }}</span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-500">.pulse.yaml</span>
          <span :class="project.has_pulse_yaml ? 'text-green-400' : 'text-gray-600'">
            {{ project.has_pulse_yaml ? "Found" : "Not found" }}
          </span>
        </div>
        <div class="flex justify-between">
          <span class="text-gray-500">.a2.yaml</span>
          <span :class="project.has_a2_yaml ? 'text-green-400' : 'text-gray-600'">
            {{ project.has_a2_yaml ? "Found" : "Not found" }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
