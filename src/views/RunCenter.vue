<script setup lang="ts">
import { onMounted, computed, ref } from "vue";
import { useProjectsStore } from "../stores/projects";
import { useRunsStore } from "../stores/runs";
import LogStream from "../components/LogStream.vue";
import ConfirmGate from "../components/ConfirmGate.vue";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const runsStore = useRunsStore();

const project = computed(() => projectsStore.currentProject);
const config = computed(() => projectsStore.pulseConfig);
const macros = computed(() => config.value?.macros || []);
const activeTab = ref<"macros" | "history">("macros");
const pendingConfirm = ref<{
  step: { run: string; cwd: string | null; confirm: boolean | null };
  macroId: string;
  resolve: (confirmed: boolean) => void;
} | null>(null);

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await runsStore.fetchRuns(props.id);
  runsStore.setupEventListeners();
});

const currentLogs = computed(() => {
  if (!runsStore.activeRunId) return [];
  return runsStore.activeRunLogs.get(runsStore.activeRunId) || [];
});

async function executeMacro(macro_: {
  id: string;
  title: string;
  steps: Array<{ run: string; cwd: string | null; confirm: boolean | null }>;
}) {
  if (!project.value) return;
  const rootPath = project.value.root_path;

  for (const step of macro_.steps) {
    // Check if step needs confirmation
    if (step.confirm) {
      const confirmed = await requestConfirmation(step, macro_.id);
      if (!confirmed) return;
    }

    await runsStore.executeStep(props.id, macro_.id, step, rootPath);
  }

  // Refresh run history
  await runsStore.fetchRuns(props.id);
}

function requestConfirmation(
  step: { run: string; cwd: string | null; confirm: boolean | null },
  macroId: string
): Promise<boolean> {
  return new Promise((resolve) => {
    pendingConfirm.value = { step, macroId, resolve };
  });
}

function handleConfirm(confirmed: boolean) {
  if (pendingConfirm.value) {
    pendingConfirm.value.resolve(confirmed);
    pendingConfirm.value = null;
  }
}

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
  <div class="space-y-6">
    <h1 class="text-2xl font-bold text-white">Run Center</h1>

    <!-- Tabs -->
    <div class="flex gap-1 border-b border-gray-800">
      <button
        @click="activeTab = 'macros'"
        class="px-4 py-2 text-sm font-medium border-b-2 transition-colors"
        :class="
          activeTab === 'macros'
            ? 'border-blue-500 text-blue-400'
            : 'border-transparent text-gray-500 hover:text-gray-300'
        "
      >
        Macros
      </button>
      <button
        @click="activeTab = 'history'"
        class="px-4 py-2 text-sm font-medium border-b-2 transition-colors"
        :class="
          activeTab === 'history'
            ? 'border-blue-500 text-blue-400'
            : 'border-transparent text-gray-500 hover:text-gray-300'
        "
      >
        History
      </button>
    </div>

    <!-- Macros tab -->
    <div v-if="activeTab === 'macros'" class="space-y-4">
      <div v-if="macros.length === 0" class="text-gray-500 text-sm">
        No macros defined. Add macros to your .pulse.yaml file.
      </div>
      <div
        v-for="macro_ in macros"
        :key="macro_.id"
        class="bg-gray-900 rounded-lg border border-gray-800 p-4"
      >
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-medium text-white">{{ macro_.title }}</h3>
          <button
            @click="executeMacro(macro_)"
            class="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-sm rounded transition-colors"
          >
            Run
          </button>
        </div>
        <ul class="space-y-1">
          <li
            v-for="(step, i) in macro_.steps"
            :key="i"
            class="flex items-center gap-2 text-sm"
          >
            <span class="text-gray-600 font-mono text-xs w-5">{{ i + 1 }}.</span>
            <code class="text-gray-400 font-mono text-xs">{{ step.run }}</code>
            <span v-if="step.cwd" class="text-gray-600 text-xs">
              in {{ step.cwd }}
            </span>
            <span
              v-if="step.confirm"
              class="text-yellow-500 text-xs"
            >
              (requires confirmation)
            </span>
          </li>
        </ul>
      </div>

      <!-- Log output -->
      <LogStream v-if="currentLogs.length > 0" :lines="currentLogs" />
    </div>

    <!-- History tab -->
    <div v-if="activeTab === 'history'" class="space-y-2">
      <div v-if="runsStore.runs.length === 0" class="text-gray-500 text-sm">
        No runs yet.
      </div>
      <div
        v-for="run in runsStore.runs"
        :key="run.id"
        class="flex items-center justify-between py-3 px-4 bg-gray-900 rounded-lg border border-gray-800 text-sm"
      >
        <div class="flex items-center gap-4">
          <span :class="statusColor(run.status)" class="font-medium w-20">
            {{ run.status }}
          </span>
          <code class="text-gray-400 font-mono text-xs truncate max-w-md">
            {{ run.command }}
          </code>
        </div>
        <div class="flex items-center gap-4 text-xs text-gray-600">
          <span v-if="run.exit_code !== null">exit: {{ run.exit_code }}</span>
          <span>{{ run.started_at }}</span>
          <button
            v-if="run.status === 'running'"
            @click="runsStore.cancelRun(run.id)"
            class="text-red-400 hover:text-red-300"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>

    <!-- Confirm gate -->
    <ConfirmGate
      v-if="pendingConfirm"
      :command="pendingConfirm.step.run"
      @confirm="handleConfirm(true)"
      @cancel="handleConfirm(false)"
    />
  </div>
</template>
