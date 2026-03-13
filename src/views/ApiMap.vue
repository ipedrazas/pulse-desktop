<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);

interface ApiEndpoint {
  method: string;
  path: string;
  handler: string;
  file: string;
  line: number;
}

interface ApiMapResult {
  framework: string;
  endpoints: ApiEndpoint[];
}

const result = ref<ApiMapResult | null>(null);
const loading = ref(false);

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await discover();
});

async function discover() {
  if (!project.value) return;
  loading.value = true;
  try {
    result.value = await invoke("discover_api_map", {
      rootPath: project.value.root_path,
    });
  } finally {
    loading.value = false;
  }
}

function methodColor(method: string) {
  switch (method) {
    case "GET":
      return "bg-green-600/20 text-green-400";
    case "POST":
      return "bg-blue-600/20 text-blue-400";
    case "PUT":
      return "bg-yellow-600/20 text-yellow-400";
    case "DELETE":
      return "bg-red-600/20 text-red-400";
    case "PATCH":
      return "bg-purple-600/20 text-purple-400";
    default:
      return "bg-gray-600/20 text-gray-400";
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-white">API Map</h1>
      <div class="flex items-center gap-3">
        <span v-if="result" class="text-xs text-gray-500">
          {{ result.framework }} - {{ result.endpoints.length }} endpoints
        </span>
        <button
          @click="discover"
          :disabled="loading"
          class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
        >
          {{ loading ? "Scanning..." : "Refresh" }}
        </button>
      </div>
    </div>

    <div v-if="loading && !result" class="text-gray-500 text-sm">
      Discovering API endpoints...
    </div>

    <div v-else-if="result && result.endpoints.length > 0" class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
      <table class="w-full text-sm">
        <thead>
          <tr class="border-b border-gray-800 text-xs text-gray-500 uppercase">
            <th class="text-left px-4 py-2 font-medium w-20">Method</th>
            <th class="text-left px-4 py-2 font-medium">Path</th>
            <th class="text-left px-4 py-2 font-medium">Handler</th>
            <th class="text-left px-4 py-2 font-medium">Location</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-800/50">
          <tr v-for="(ep, i) in result.endpoints" :key="i" class="hover:bg-gray-800/50">
            <td class="px-4 py-2">
              <span
                class="px-2 py-0.5 rounded text-xs font-bold"
                :class="methodColor(ep.method)"
              >
                {{ ep.method }}
              </span>
            </td>
            <td class="px-4 py-2 text-white font-mono text-xs">{{ ep.path }}</td>
            <td class="px-4 py-2 text-gray-400 font-mono text-xs">{{ ep.handler }}</td>
            <td class="px-4 py-2 text-gray-600 font-mono text-xs">
              {{ ep.file }}:{{ ep.line }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-else-if="result && result.endpoints.length === 0" class="text-center py-12 text-gray-500">
      <p class="text-lg mb-2">No API endpoints discovered</p>
      <p class="text-sm">Supports Express/Node.js and Go (net/http, chi, gin) routes.</p>
    </div>
  </div>
</template>
