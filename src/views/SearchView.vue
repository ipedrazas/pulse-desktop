<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);

const query = ref("");
const filePattern = ref("");
const searching = ref(false);

interface SearchResult {
  file: string;
  line_number: number;
  line: string;
}

const results = ref<SearchResult[]>([]);

async function doSearch() {
  if (!project.value || !query.value.trim()) return;
  searching.value = true;
  try {
    results.value = await invoke("search_project", {
      rootPath: project.value.root_path,
      query: query.value,
      filePattern: filePattern.value || null,
    });
  } finally {
    searching.value = false;
  }
}

const groupedResults = computed(() => {
  const groups: Record<string, SearchResult[]> = {};
  for (const r of results.value) {
    if (!groups[r.file]) groups[r.file] = [];
    groups[r.file].push(r);
  }
  return groups;
});

const fileCount = computed(() => Object.keys(groupedResults.value).length);
</script>

<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold text-white">Search</h1>

    <!-- Search form -->
    <form @submit.prevent="doSearch" class="flex gap-2">
      <input
        v-model="query"
        type="text"
        placeholder="Search text..."
        class="flex-1 px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
      />
      <input
        v-model="filePattern"
        type="text"
        placeholder="*.ts, *.go..."
        class="w-40 px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
      />
      <button
        type="submit"
        :disabled="searching || !query.trim()"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
      >
        {{ searching ? "..." : "Search" }}
      </button>
    </form>

    <!-- Results summary -->
    <div v-if="results.length > 0" class="text-sm text-gray-500">
      {{ results.length }} matches in {{ fileCount }} files
    </div>
    <div v-else-if="query && !searching" class="text-sm text-gray-500">
      No results found.
    </div>

    <!-- Results grouped by file -->
    <div v-for="(fileResults, file) in groupedResults" :key="file" class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
      <div class="px-4 py-2 border-b border-gray-800 text-sm font-mono text-blue-400">
        {{ file }}
      </div>
      <div class="divide-y divide-gray-800/50">
        <div
          v-for="result in fileResults"
          :key="`${file}:${result.line_number}`"
          class="flex gap-3 px-4 py-1.5 text-xs font-mono hover:bg-gray-800/50"
        >
          <span class="text-gray-600 w-10 text-right shrink-0">{{ result.line_number }}</span>
          <span class="text-gray-300 whitespace-pre overflow-x-auto">{{ result.line }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
