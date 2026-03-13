<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);

interface Snapshot {
  id: number;
  project_id: string;
  kind: string;
  commit_sha: string | null;
  payload: string;
  created_at: string;
}

interface SnapshotDiffResult {
  kind: string;
  older: { id: number; commit_sha: string | null; created_at: string; item_count: number };
  newer: { id: number; commit_sha: string | null; created_at: string; item_count: number };
  added: string[];
  removed: string[];
  changed: string[];
}

const snapshots = ref<Snapshot[]>([]);
const selectedOlder = ref<number | null>(null);
const selectedNewer = ref<number | null>(null);
const diff = ref<SnapshotDiffResult | null>(null);
const loading = ref(false);
const filterKind = ref<string>("health");

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await loadSnapshots();
});

async function loadSnapshots() {
  if (!project.value) return;
  snapshots.value = await invoke("list_snapshots", {
    projectId: project.value.id,
    kind: filterKind.value || null,
  });
}

const filteredSnapshots = computed(() => {
  if (!filterKind.value) return snapshots.value;
  return snapshots.value.filter((s) => s.kind === filterKind.value);
});

async function compareDiff() {
  if (!selectedOlder.value || !selectedNewer.value) return;
  loading.value = true;
  try {
    diff.value = await invoke("diff_snapshots", {
      olderId: selectedOlder.value,
      newerId: selectedNewer.value,
    });
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold text-white">Snapshot History</h1>

    <!-- Filter -->
    <div class="flex items-center gap-4">
      <select
        v-model="filterKind"
        @change="loadSnapshots"
        class="px-3 py-1.5 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white"
      >
        <option value="health">Health</option>
        <option value="deps_node">Node Deps</option>
        <option value="deps_go">Go Deps</option>
        <option value="">All</option>
      </select>

      <span class="text-sm text-gray-500">{{ filteredSnapshots.length }} snapshots</span>
    </div>

    <!-- Snapshot list -->
    <div v-if="filteredSnapshots.length >= 2" class="space-y-4">
      <div class="flex gap-4">
        <div class="flex-1">
          <label class="text-xs text-gray-500 mb-1 block">Older snapshot</label>
          <select
            v-model="selectedOlder"
            class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white"
          >
            <option :value="null">Select...</option>
            <option v-for="s in filteredSnapshots" :key="s.id" :value="s.id">
              {{ s.created_at }} {{ s.commit_sha ? `(${s.commit_sha})` : "" }}
            </option>
          </select>
        </div>
        <div class="flex-1">
          <label class="text-xs text-gray-500 mb-1 block">Newer snapshot</label>
          <select
            v-model="selectedNewer"
            class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white"
          >
            <option :value="null">Select...</option>
            <option v-for="s in filteredSnapshots" :key="s.id" :value="s.id">
              {{ s.created_at }} {{ s.commit_sha ? `(${s.commit_sha})` : "" }}
            </option>
          </select>
        </div>
        <div class="flex items-end">
          <button
            @click="compareDiff"
            :disabled="!selectedOlder || !selectedNewer || loading"
            class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
          >
            Compare
          </button>
        </div>
      </div>

      <!-- Diff result -->
      <div v-if="diff" class="space-y-4">
        <div class="grid grid-cols-3 gap-4">
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <div class="text-2xl font-bold text-green-400">{{ diff.added.length }}</div>
            <div class="text-xs text-gray-500 mt-1">Added</div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <div class="text-2xl font-bold text-red-400">{{ diff.removed.length }}</div>
            <div class="text-xs text-gray-500 mt-1">Removed</div>
          </div>
          <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 text-center">
            <div class="text-2xl font-bold text-yellow-400">{{ diff.changed.length }}</div>
            <div class="text-xs text-gray-500 mt-1">Changed</div>
          </div>
        </div>

        <div v-if="diff.added.length > 0">
          <h3 class="text-xs font-medium text-green-400 mb-1">Added</h3>
          <div class="flex flex-wrap gap-1">
            <span v-for="item in diff.added" :key="item" class="px-2 py-0.5 bg-green-900/30 border border-green-800/50 rounded text-xs text-green-300 font-mono">
              {{ item }}
            </span>
          </div>
        </div>

        <div v-if="diff.removed.length > 0">
          <h3 class="text-xs font-medium text-red-400 mb-1">Removed</h3>
          <div class="flex flex-wrap gap-1">
            <span v-for="item in diff.removed" :key="item" class="px-2 py-0.5 bg-red-900/30 border border-red-800/50 rounded text-xs text-red-300 font-mono">
              {{ item }}
            </span>
          </div>
        </div>

        <div v-if="diff.changed.length > 0">
          <h3 class="text-xs font-medium text-yellow-400 mb-1">Changed</h3>
          <div class="flex flex-wrap gap-1">
            <span v-for="item in diff.changed" :key="item" class="px-2 py-0.5 bg-yellow-900/30 border border-yellow-800/50 rounded text-xs text-yellow-300 font-mono">
              {{ item }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="filteredSnapshots.length < 2" class="text-sm text-gray-500">
      Need at least 2 snapshots to compare. Run health checks to generate snapshots.
    </div>
  </div>
</template>
