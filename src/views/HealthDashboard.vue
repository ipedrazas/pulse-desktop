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
  python: {
    outdated: Array<{
      name: string;
      current: string;
      latest: string;
    }>;
    vulnerabilities: Array<{
      package: string;
      vulnerability_id: string;
      description: string;
      fixed_in: string;
    }>;
    outdated_count: number;
    vuln_count: number;
    python_version: string | null;
    venv_detected: boolean;
  } | null;
  rust: {
    outdated: Array<{
      name: string;
      current: string;
      latest: string;
      kind: string;
    }>;
    vulnerabilities: Array<{
      id: string;
      package: string;
      title: string;
      severity: string;
      url: string;
    }>;
    outdated_count: number;
    vuln_count: number;
    rust_version: string | null;
  } | null;
  java: {
    outdated: Array<{
      group: string;
      artifact: string;
      current: string;
      latest: string;
    }>;
    outdated_count: number;
    java_version: string | null;
    build_tool: string;
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
          <div class="text-2xl font-bold" :class="(health.node?.outdated_count || 0) + (health.go?.outdated_count || 0) + (health.python?.outdated_count || 0) + (health.rust?.outdated_count || 0) + (health.java?.outdated_count || 0) > 0 ? 'text-yellow-400' : 'text-green-400'">
            {{ (health.node?.outdated_count || 0) + (health.go?.outdated_count || 0) + (health.python?.outdated_count || 0) + (health.rust?.outdated_count || 0) + (health.java?.outdated_count || 0) }}
          </div>
        </div>
        <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
          <div class="text-xs text-gray-500 uppercase tracking-wider mb-1">Vulnerabilities</div>
          <div class="text-2xl font-bold" :class="(health.node?.vuln_count || 0) + (health.go?.vuln_count || 0) + (health.python?.vuln_count || 0) + (health.rust?.vuln_count || 0) > 0 ? 'text-red-400' : 'text-green-400'">
            {{ (health.node?.vuln_count || 0) + (health.go?.vuln_count || 0) + (health.python?.vuln_count || 0) + (health.rust?.vuln_count || 0) }}
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

      <!-- Python dependencies -->
      <div v-if="health.python">
        <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Python Dependencies
          <span v-if="health.python.python_version" class="text-gray-600 normal-case ml-2">
            {{ health.python.python_version }}
          </span>
        </h2>
        <DependencyTable
          :items="health.python.outdated.map(d => ({
            name: d.name,
            current: d.current,
            latest: d.latest,
            type: 'pip',
          }))"
        />

        <div v-if="health.python.vulnerabilities.length > 0" class="mt-4">
          <h3 class="text-sm font-medium text-red-400 mb-2">
            Vulnerabilities ({{ health.python.vulnerabilities.length }})
          </h3>
          <div class="space-y-1">
            <div
              v-for="vuln in health.python.vulnerabilities"
              :key="vuln.vulnerability_id"
              class="flex items-center gap-3 py-2 px-3 bg-gray-900 rounded border border-gray-800 text-sm"
            >
              <span class="text-yellow-400 font-mono text-xs shrink-0">{{ vuln.vulnerability_id }}</span>
              <span class="text-white">{{ vuln.package }}</span>
              <span class="text-gray-500 truncate">{{ vuln.description }}</span>
              <span v-if="vuln.fixed_in" class="text-green-400 text-xs shrink-0">
                fixed: {{ vuln.fixed_in }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Rust dependencies -->
      <div v-if="health.rust">
        <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Rust Dependencies
          <span v-if="health.rust.rust_version" class="text-gray-600 normal-case ml-2">
            {{ health.rust.rust_version }}
          </span>
        </h2>
        <DependencyTable
          :items="health.rust.outdated.map(d => ({
            name: d.name,
            current: d.current,
            latest: d.latest,
            type: d.kind,
          }))"
        />

        <div v-if="health.rust.vulnerabilities.length > 0" class="mt-4">
          <h3 class="text-sm font-medium text-red-400 mb-2">
            Vulnerabilities ({{ health.rust.vulnerabilities.length }})
          </h3>
          <div class="space-y-1">
            <div
              v-for="vuln in health.rust.vulnerabilities"
              :key="vuln.id"
              class="flex items-center gap-3 py-2 px-3 bg-gray-900 rounded border border-gray-800 text-sm"
            >
              <span class="text-yellow-400 font-mono text-xs shrink-0">{{ vuln.id }}</span>
              <span class="text-white">{{ vuln.package }}</span>
              <span class="text-gray-500 truncate">{{ vuln.title }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Java dependencies -->
      <div v-if="health.java">
        <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
          Java Dependencies
          <span v-if="health.java.java_version" class="text-gray-600 normal-case ml-2">
            {{ health.java.java_version }}
          </span>
          <span class="text-gray-600 normal-case ml-2">({{ health.java.build_tool }})</span>
        </h2>
        <DependencyTable
          :items="health.java.outdated.map(d => ({
            name: d.group + ':' + d.artifact,
            current: d.current,
            latest: d.latest,
            type: health!.java!.build_tool,
          }))"
        />
      </div>

      <!-- Env Parity -->
      <EnvParityDiff v-if="health.env_parity" :parity="health.env_parity" />
    </template>
  </div>
</template>
