<script setup lang="ts">
import { onMounted, computed, ref } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useProjectsStore } from "../stores/projects";
import { useRunsStore } from "../stores/runs";
import ServiceCard from "../components/ServiceCard.vue";

const props = defineProps<{ id: string }>();
const router = useRouter();
const projectsStore = useProjectsStore();
const runsStore = useRunsStore();

const project = computed(() => projectsStore.currentProject);
const config = computed(() => projectsStore.pulseConfig);
const recentRuns = computed(() => runsStore.runs.slice(0, 5));
const services = computed(() => config.value?.services || []);
const connectors = computed(() => config.value?.connectors || []);

interface ResolvedConnector {
  id: string;
  title: string;
  resolved_command: string | null;
  resolved_url: string | null;
}

interface AgentSession {
  id: number;
  project_id: string;
  title: string | null;
  tool: string | null;
  task_summary: string | null;
  created_at: string;
  updated_at: string;
}

const resolvedConnectors = ref<ResolvedConnector[]>([]);
const worklog = ref<AgentSession[]>([]);
const copiedConnector = ref<string | null>(null);
const expandedRunId = ref<number | null>(null);
const expandedRunLogs = ref<{ stream: string; chunk: string }[]>([]);
const loadingLogs = ref(false);

async function toggleRunDetail(runId: number) {
  if (expandedRunId.value === runId) {
    expandedRunId.value = null;
    expandedRunLogs.value = [];
    return;
  }
  expandedRunId.value = runId;
  loadingLogs.value = true;
  try {
    expandedRunLogs.value = await runsStore.getRunLogs(runId);
  } catch {
    expandedRunLogs.value = [];
  } finally {
    loadingLogs.value = false;
  }
}

// Run history trends
const successRate = computed(() => {
  const finished = runsStore.runs.filter((r) => r.status !== "running");
  if (finished.length === 0) return null;
  const successes = finished.filter((r) => r.status === "success").length;
  return Math.round((successes / finished.length) * 100);
});

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await runsStore.fetchRuns(props.id);
  runsStore.setupEventListeners();

  // Load connectors
  if (connectors.value.length > 0) {
    resolvedConnectors.value = await invoke("resolve_connectors", {
      connectors: connectors.value,
    });
  }

  // Load worklog
  worklog.value = await invoke("get_worklog", { projectId: props.id });
});

async function copyConnectorCommand(cmd: string, connectorId: string) {
  await writeText(cmd);
  copiedConnector.value = connectorId;
  setTimeout(() => (copiedConnector.value = null), 2000);
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
  <div v-if="project" class="space-y-6">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-bold text-white">{{ project.name }}</h1>
      <p class="text-sm text-gray-500 mt-1">{{ project.root_path }}</p>
    </div>

    <!-- Info cards -->
    <div class="grid grid-cols-4 gap-4">
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="text-xs text-gray-500 uppercase tracking-wider mb-2">Type</div>
        <div class="text-white font-medium">
          {{ project.project_type || "Unknown" }}
        </div>
        <div v-if="project.language_dirs" class="mt-2 space-y-1">
          <div
            v-for="(dirs, lang) in project.language_dirs"
            :key="lang"
            class="flex items-center gap-2 text-xs"
          >
            <span class="text-blue-400 font-medium">{{ lang }}</span>
            <span class="text-gray-500">{{ dirs.join(", ") }}</span>
          </div>
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
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="text-xs text-gray-500 uppercase tracking-wider mb-2">Success Rate</div>
        <div v-if="successRate !== null" class="text-2xl font-bold" :class="successRate >= 80 ? 'text-green-400' : successRate >= 50 ? 'text-yellow-400' : 'text-red-400'">
          {{ successRate }}%
        </div>
        <div v-else class="text-gray-600 text-sm">No runs</div>
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

    <!-- Services -->
    <div v-if="services.length > 0">
      <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
        Services
      </h2>
      <div class="space-y-2">
        <ServiceCard
          v-for="svc in services"
          :key="svc.name"
          :project-id="props.id"
          :service="svc"
          :root-path="project.root_path"
        />
      </div>
    </div>

    <!-- Connectors -->
    <div v-if="resolvedConnectors.length > 0">
      <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
        Connectors
      </h2>
      <div class="flex flex-wrap gap-2">
        <div
          v-for="conn in resolvedConnectors"
          :key="conn.id"
          class="flex items-center gap-2 px-3 py-2 bg-gray-900 rounded-lg border border-gray-800"
        >
          <span class="text-sm text-gray-300">{{ conn.title }}</span>
          <a
            v-if="conn.resolved_url"
            :href="conn.resolved_url"
            target="_blank"
            class="text-xs text-blue-400 hover:text-blue-300"
          >
            Open
          </a>
          <button
            v-if="conn.resolved_command"
            @click="copyConnectorCommand(conn.resolved_command!, conn.id)"
            class="text-xs transition-colors"
            :class="copiedConnector === conn.id ? 'text-green-400' : 'text-gray-500 hover:text-gray-300'"
          >
            {{ copiedConnector === conn.id ? "Copied!" : "Copy cmd" }}
          </button>
        </div>
      </div>
    </div>

    <!-- Worklog -->
    <div v-if="worklog.length > 0">
      <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
        Worklog
      </h2>
      <div class="space-y-1">
        <div
          v-for="session in worklog"
          :key="session.id"
          class="flex items-center justify-between py-2 px-3 bg-gray-900 rounded border border-gray-800 text-sm"
        >
          <div class="flex items-center gap-3">
            <span v-if="session.tool" class="text-xs text-purple-400 font-medium">
              {{ session.tool }}
            </span>
            <span class="text-gray-300">{{ session.title || "Untitled session" }}</span>
          </div>
          <span class="text-gray-600 text-xs">{{ session.updated_at }}</span>
        </div>
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
          class="bg-gray-900 rounded border border-gray-800"
        >
          <button
            @click="toggleRunDetail(run.id)"
            class="w-full flex items-center justify-between py-2 px-3 text-sm hover:bg-gray-800/50 transition-colors text-left"
          >
            <div class="flex items-center gap-3">
              <span :class="statusColor(run.status)" class="font-medium">
                {{ run.status }}
              </span>
              <span class="text-gray-400 font-mono text-xs truncate max-w-xs">
                {{ run.command }}
              </span>
            </div>
            <div class="flex items-center gap-3">
              <span v-if="run.exit_code !== null" class="text-gray-600 text-xs">
                exit: {{ run.exit_code }}
              </span>
              <span class="text-gray-600 text-xs">{{ run.started_at }}</span>
            </div>
          </button>
          <div v-if="expandedRunId === run.id" class="border-t border-gray-800">
            <div v-if="loadingLogs" class="p-3 text-xs text-gray-500">Loading logs...</div>
            <div v-else-if="expandedRunLogs.length === 0" class="p-3 text-xs text-gray-500">
              No output captured.
            </div>
            <div v-else class="p-3 max-h-64 overflow-y-auto font-mono text-xs leading-5">
              <div
                v-for="(log, i) in expandedRunLogs"
                :key="i"
                :class="log.stream === 'stderr' ? 'text-red-400' : 'text-gray-400'"
              >
                {{ log.chunk }}
              </div>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>

  <div v-else class="text-gray-500">Loading project...</div>
</template>
