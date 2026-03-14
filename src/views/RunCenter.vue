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
const expandedRunId = ref<number | null>(null);
const expandedRunLogs = ref<{ stream: string; chunk: string }[]>([]);
const loadingLogs = ref(false);

interface ParsedCheck {
  name: string;
  status: string;
  message: string;
  file: string | null;
  line: number | null;
}

const parsedA2Checks = ref<ParsedCheck[]>([]);
const expandedRunStderr = ref<string | null>(null);

function parseA2Json(jsonStr: string): ParsedCheck[] {
  try {
    const value = JSON.parse(jsonStr);
    const items: Record<string, unknown>[] = Array.isArray(value)
      ? value
      : value.checks || value.results || [value];

    return items.map((item) => {
      const name = (item.name || item.rule || item.id || "unknown") as string;
      const rawStatus = ((item.status || item.result || "fail") as string).toLowerCase();
      let status = "fail";
      if (["pass", "ok", "success"].includes(rawStatus)) status = "pass";
      else if (["warn", "warning"].includes(rawStatus)) status = "warning";

      return {
        name,
        status,
        message: (item.message || item.description || "") as string,
        file: (item.file || item.path || null) as string | null,
        line: (item.line || item.lineNumber || null) as number | null,
      };
    });
  } catch {
    return [];
  }
}

function checkStatusIcon(status: string) {
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

async function toggleRunDetail(runId: number) {
  if (expandedRunId.value === runId) {
    expandedRunId.value = null;
    expandedRunLogs.value = [];
    parsedA2Checks.value = [];
    expandedRunStderr.value = null;
    return;
  }
  expandedRunId.value = runId;
  loadingLogs.value = true;
  parsedA2Checks.value = [];
  expandedRunStderr.value = null;
  try {
    const logs = await runsStore.getRunLogs(runId);
    expandedRunLogs.value = logs;

    // If this is an a2 run, try to parse stdout as structured checks
    const run = runsStore.runs.find((r) => r.id === runId);
    if (run?.kind === "a2") {
      const stdoutLog = logs.find((l) => l.stream === "stdout");
      if (stdoutLog?.chunk) {
        parsedA2Checks.value = parseA2Json(stdoutLog.chunk);
      }
      const stderrLog = logs.find((l) => l.stream === "stderr");
      if (stderrLog?.chunk) {
        expandedRunStderr.value = stderrLog.chunk;
      }
    }
  } catch {
    expandedRunLogs.value = [];
  } finally {
    loadingLogs.value = false;
  }
}

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
        class="bg-gray-900 rounded-lg border border-gray-800"
      >
        <button
          @click="toggleRunDetail(run.id)"
          class="w-full flex items-center justify-between py-3 px-4 text-sm hover:bg-gray-800/50 transition-colors text-left"
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
              @click.stop="runsStore.cancelRun(run.id)"
              class="text-red-400 hover:text-red-300"
            >
              Cancel
            </button>
          </div>
        </button>
        <div v-if="expandedRunId === run.id" class="border-t border-gray-800">
          <div v-if="loadingLogs" class="p-4 text-xs text-gray-500">Loading logs...</div>
          <div v-else-if="expandedRunLogs.length === 0" class="p-4 text-xs text-gray-500">
            No output captured.
          </div>

          <!-- Parsed a2 checks -->
          <template v-else-if="run.kind === 'a2' && parsedA2Checks.length > 0">
            <!-- Summary bar -->
            <div class="flex gap-4 px-4 py-2 bg-gray-950 text-xs">
              <span class="text-green-400">
                {{ parsedA2Checks.filter(c => c.status === 'pass').length }} passed
              </span>
              <span class="text-red-400">
                {{ parsedA2Checks.filter(c => c.status === 'fail').length }} failed
              </span>
              <span class="text-yellow-400">
                {{ parsedA2Checks.filter(c => c.status === 'warning').length }} warnings
              </span>
            </div>
            <div class="max-h-96 overflow-y-auto divide-y divide-gray-800/50">
              <div
                v-for="(check, i) in parsedA2Checks"
                :key="i"
                class="flex items-center gap-3 py-2 px-4"
              >
                <span
                  :class="checkStatusIcon(check.status).class"
                  class="text-xs font-bold w-12 shrink-0"
                >
                  {{ checkStatusIcon(check.status).text }}
                </span>
                <div class="flex-1 min-w-0">
                  <div class="text-sm text-white">{{ check.name }}</div>
                  <div
                    v-if="check.message && check.message !== check.name"
                    class="text-xs text-gray-500 truncate"
                  >
                    {{ check.message }}
                  </div>
                </div>
                <span v-if="check.file" class="text-xs text-gray-600 font-mono shrink-0">
                  {{ check.file }}{{ check.line ? `:${check.line}` : "" }}
                </span>
              </div>
            </div>
            <!-- stderr -->
            <div v-if="expandedRunStderr" class="border-t border-gray-800">
              <div class="px-4 py-1.5 text-[10px] text-gray-600 uppercase tracking-wider">stderr</div>
              <pre class="px-4 pb-3 text-xs text-red-400 font-mono overflow-auto max-h-32">{{ expandedRunStderr }}</pre>
            </div>
          </template>

          <!-- Generic log output (non-a2 runs) -->
          <div v-else class="p-4 max-h-96 overflow-y-auto font-mono text-xs leading-5">
            <div
              v-for="(log, i) in expandedRunLogs"
              :key="i"
              :class="log.stream === 'stderr' ? 'text-red-400' : 'text-gray-400'"
            >
              {{ log.chunk }}
            </div>
          </div>
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
