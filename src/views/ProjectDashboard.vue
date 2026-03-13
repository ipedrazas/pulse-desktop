<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useRouter } from "vue-router";
import { useProjectsStore } from "../stores/projects";
import { useRunsStore } from "../stores/runs";

const props = defineProps<{ id: string }>();
const router = useRouter();
const projectsStore = useProjectsStore();
const runsStore = useRunsStore();

const project = computed(() => projectsStore.currentProject);
const config = computed(() => projectsStore.pulseConfig);
const recentRuns = computed(() => runsStore.runs.slice(0, 5));

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await runsStore.fetchRuns(props.id);
  runsStore.setupEventListeners();
});

function statusColor(status: string) {
  switch (status) {
    case "success":
      return "text-green-400";
    case "failure":
      return "text-red-400";
    case "running":
      return "text-blue-400";
    case "cancelled":
      return "text-yellow-500";
    default:
      return "text-gray-500";
  }
}
</script>

<template>
  <div v-if="project" class="space-y-6">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-bold text-white">{{ project.name }}</h1>
      <p class="text-sm text-gray-500 mt-1">{{ project.root_path }}</p>
    </div>

    <!-- Info cards -->
    <div class="grid grid-cols-3 gap-4">
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="text-xs text-gray-500 uppercase tracking-wider mb-2">Type</div>
        <div class="text-white font-medium">
          {{ project.project_type || "Unknown" }}
        </div>
      </div>
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="text-xs text-gray-500 uppercase tracking-wider mb-2">Config</div>
        <div class="flex gap-2">
          <span
            :class="project.has_pulse_yaml ? 'text-green-400' : 'text-gray-600'"
            class="text-sm"
          >
            .pulse.yaml
          </span>
          <span
            :class="project.has_a2_yaml ? 'text-green-400' : 'text-gray-600'"
            class="text-sm"
          >
            .a2.yaml
          </span>
        </div>
      </div>
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="text-xs text-gray-500 uppercase tracking-wider mb-2">Git</div>
        <div v-if="project.git" class="text-sm">
          <span class="text-blue-400">{{ project.git.branch }}</span>
          <span class="text-gray-600 ml-2 font-mono">{{ project.git.sha }}</span>
        </div>
        <div v-else class="text-gray-600 text-sm">Not a git repo</div>
      </div>
    </div>

    <!-- Macros quick access -->
    <div v-if="config?.macros?.length">
      <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
        Macros
      </h2>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="macro_ in config.macros"
          :key="macro_.id"
          @click="router.push({ name: 'run-center', params: { id: props.id } })"
          class="px-3 py-1.5 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-md text-sm text-gray-300 transition-colors"
        >
          {{ macro_.title }}
        </button>
      </div>
    </div>

    <!-- Recent runs -->
    <div>
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider">
          Recent Runs
        </h2>
        <router-link
          :to="{ name: 'run-center', params: { id: props.id } }"
          class="text-xs text-blue-400 hover:text-blue-300"
        >
          View all
        </router-link>
      </div>
      <div v-if="recentRuns.length === 0" class="text-sm text-gray-600">
        No runs yet. Execute a macro from the Run Center.
      </div>
      <ul v-else class="space-y-1">
        <li
          v-for="run in recentRuns"
          :key="run.id"
          class="flex items-center justify-between py-2 px-3 bg-gray-900 rounded border border-gray-800 text-sm"
        >
          <div class="flex items-center gap-3">
            <span :class="statusColor(run.status)" class="font-medium">
              {{ run.status }}
            </span>
            <span class="text-gray-400 font-mono text-xs truncate max-w-xs">
              {{ run.command }}
            </span>
          </div>
          <span class="text-gray-600 text-xs">{{ run.started_at }}</span>
        </li>
      </ul>
    </div>
  </div>

  <div v-else class="text-gray-500">Loading project...</div>
</template>
