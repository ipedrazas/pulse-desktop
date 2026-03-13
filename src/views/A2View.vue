<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);
const config = computed(() => projectsStore.pulseConfig);

interface A2Check {
  name: string;
  status: string;
  message: string;
  file: string | null;
  line: number | null;
  fix_macro: string | null;
}

interface A2Result {
  checks: A2Check[];
  pass_count: number;
  fail_count: number;
  warning_count: number;
}

const result = ref<A2Result | null>(null);
const running = ref(false);
const error = ref<string | null>(null);

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
});

async function runA2() {
  if (!project.value) return;
  running.value = true;
  error.value = null;
  try {
    result.value = await invoke("run_a2", {
      projectId: project.value.id,
      rootPath: project.value.root_path,
      fixHints: null,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    running.value = false;
  }
}

function statusIcon(status: string) {
  switch (status) {
    case "pass":
      return { text: "PASS", class: "text-green-400" };
    case "fail":
      return { text: "FAIL", class: "text-red-400" };
    case "warning":
      return { text: "WARN", class: "text-yellow-400" };
    default:
      return { text: "?", class: "text-gray-400" };
  }
}

function macroTitle(macroId: string): string {
  const m = config.value?.macros?.find((m) => m.id === macroId);
  return m?.title || macroId;
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-white">a2 Policy Checks</h1>
      <button
        @click="runA2"
        :disabled="running"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
      >
        {{ running ? "Running..." : "Run a2" }}
      </button>
    </div>

    <div v-if="error" class="p-4 bg-red-900/30 border border-red-800/50 rounded-lg text-sm text-red-300">
      {{ error }}
    </div>

    <template v-if="result">
      <!-- Summary -->
      <div class="grid grid-cols-3 gap-4">
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
          <div class="text-2xl font-bold text-green-400">{{ result.pass_count }}</div>
          <div class="text-xs text-gray-500 mt-1">Passed</div>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
          <div class="text-2xl font-bold text-red-400">{{ result.fail_count }}</div>
          <div class="text-xs text-gray-500 mt-1">Failed</div>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
          <div class="text-2xl font-bold text-yellow-400">{{ result.warning_count }}</div>
          <div class="text-xs text-gray-500 mt-1">Warnings</div>
        </div>
      </div>

      <!-- Checks list -->
      <div class="space-y-1">
        <div
          v-for="check in result.checks"
          :key="check.name"
          class="flex items-center gap-3 py-2 px-4 bg-gray-900 rounded-lg border border-gray-800"
        >
          <span
            :class="statusIcon(check.status).class"
            class="text-xs font-bold w-12 shrink-0"
          >
            {{ statusIcon(check.status).text }}
          </span>
          <div class="flex-1 min-w-0">
            <div class="text-sm text-white">{{ check.name }}</div>
            <div v-if="check.message && check.message !== check.name" class="text-xs text-gray-500 truncate">
              {{ check.message }}
            </div>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <span v-if="check.file" class="text-xs text-gray-600 font-mono">
              {{ check.file }}{{ check.line ? `:${check.line}` : "" }}
            </span>
            <button
              v-if="check.fix_macro && check.status === 'fail'"
              class="px-2 py-0.5 bg-orange-600/20 text-orange-400 text-xs rounded hover:bg-orange-600/30 transition-colors"
            >
              Fix: {{ macroTitle(check.fix_macro) }}
            </button>
          </div>
        </div>
      </div>
    </template>

    <div v-else-if="!error && !running" class="text-center py-12 text-gray-500">
      <p class="text-lg mb-2">Run a2 to check project policies</p>
      <p class="text-sm">Requires a2 binary on PATH and .a2.yaml in project root.</p>
    </div>
  </div>
</template>
