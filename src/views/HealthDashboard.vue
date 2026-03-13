<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";
import DependencyTable from "../components/DependencyTable.vue";
import EnvParityDiff from "../components/EnvParityDiff.vue";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);
const loading = ref(false);

interface HealthSummary {
  project_type: string | null;
  node: {
    outdated: Array<{
      name: string;
      current: string;
      wanted: string;
      latest: string;
      dep_type: string;
    }>;
    vulnerabilities: Array<{
      name: string;
      severity: string;
      title: string;
      url: string;
      range: string;
    }>;
    outdated_count: number;
    vuln_count: number;
  } | null;
  go: {
    modules: Array<{
      path: string;
      version: string;
      update_version: string | null;
      indirect: boolean;
    }>;
    vulnerabilities: Array<{
      id: string;
      aliases: string[];
      summary: string;
      module_path: string;
      found_version: string;
      fixed_version: string;
    }>;
    outdated_count: number;
    vuln_count: number;
  } | null;
  env_parity: {
    missing_keys: string[];
    extra_keys: string[];
    empty_keys: string[];
    example_count: number;
    actual_count: number;
  } | null;
}

const health = ref<HealthSummary | null>(null);

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await refresh();
});

async function refresh() {
  if (!project.value) return;
  loading.value = true;
  try {
    health.value = await invoke("get_health_summary", {
      projectId: project.value.id,
      rootPath: project.value.root_path,
    });
  } finally {
    loading.value = false;
  }
}

function severityColor(severity: string) {
  switch (severity) {
    case "critical":
      return "text-red-500";
    case "high":
      return "text-orange-400";
    case "moderate":
    case "medium":
      return "text-yellow-400";
    case "low":
      return "text-blue-400";
    default:
      return "text-gray-400";
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-white">Health Dashboard</h1>
      <button
        @click="refresh"
        :disabled="loading"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
      >
        {{ loading ? "Scanning..." : "Refresh" }}
      </button>
    </div>

    <div v-if="loading && !health" class="text-gray-500 text-sm">
      Analyzing project health...
    </div>

    <template v-if="health">
      <!-- Summary cards -->
      <div class="grid grid-cols-3 gap-4">
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
          <div class="text-xs text-gray-500 uppercase tracking-wider mb-1">Outdated</div>
          <div class="text-2xl font-bold" :class="(health.node?.outdated_count || 0) + (health.go?.outdated_count || 0) > 0 ? 'text-yellow-400' : 'text-green-400'">
            {{ (health.node?.outdated_count || 0) + (health.go?.outdated_count || 0) }}
          </div>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
          <div class="text-xs text-gray-500 uppercase tracking-wider mb-1">Vulnerabilities</div>
          <div class="text-2xl font-bold" :class="(health.node?.vuln_count || 0) + (health.go?.vuln_count || 0) > 0 ? 'text-red-400' : 'text-green-400'">
            {{ (health.node?.vuln_count || 0) + (health.go?.vuln_count || 0) }}
          </div>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
          <div class="text-xs text-gray-500 uppercase tracking-wider mb-1">Env Parity</div>
          <div v-if="health.env_parity" class="text-2xl font-bold" :class="health.env_parity.missing_keys.length > 0 ? 'text-red-400' : 'text-green-400'">
            {{ health.env_parity.missing_keys.length === 0 ? "OK" : health.env_parity.missing_keys.length + " missing" }}
          </div>
          <div v-else class="text-sm text-gray-600">No .env.example</div>
        </div>
      </div>

      <!-- Node dependencies -->
      <div v-if="health.node">
        <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Node Dependencies
        </h2>
        <DependencyTable
          :items="health.node.outdated.map(d => ({
            name: d.name,
            current: d.current,
            latest: d.latest,
            type: d.dep_type,
          }))"
        />

        <!-- Vulnerabilities -->
        <div v-if="health.node.vulnerabilities.length > 0" class="mt-4">
          <h3 class="text-sm font-medium text-red-400 mb-2">
            Vulnerabilities ({{ health.node.vulnerabilities.length }})
          </h3>
          <div class="space-y-1">
            <div
              v-for="vuln in health.node.vulnerabilities"
              :key="vuln.name"
              class="flex items-center gap-3 py-2 px-3 bg-gray-900 rounded border border-gray-800 text-sm"
            >
              <span :class="severityColor(vuln.severity)" class="font-medium w-20 shrink-0">
                {{ vuln.severity }}
              </span>
              <span class="text-white">{{ vuln.name }}</span>
              <span class="text-gray-500 truncate">{{ vuln.title }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Go modules -->
      <div v-if="health.go">
        <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Go Modules
        </h2>
        <DependencyTable
          :items="health.go.modules
            .filter(m => m.update_version)
            .map(m => ({
              name: m.path,
              current: m.version,
              latest: m.update_version || m.version,
              type: m.indirect ? 'indirect' : 'direct',
            }))"
        />

        <div v-if="health.go.vulnerabilities.length > 0" class="mt-4">
          <h3 class="text-sm font-medium text-red-400 mb-2">
            Vulnerabilities ({{ health.go.vulnerabilities.length }})
          </h3>
          <div class="space-y-1">
            <div
              v-for="vuln in health.go.vulnerabilities"
              :key="vuln.id"
              class="flex items-center gap-3 py-2 px-3 bg-gray-900 rounded border border-gray-800 text-sm"
            >
              <span class="text-yellow-400 font-mono text-xs shrink-0">{{ vuln.id }}</span>
              <span class="text-gray-400 truncate">{{ vuln.summary }}</span>
              <span v-if="vuln.fixed_version" class="text-green-400 text-xs shrink-0">
                fixed: {{ vuln.fixed_version }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Env Parity -->
      <EnvParityDiff v-if="health.env_parity" :parity="health.env_parity" />
    </template>
  </div>
</template>
