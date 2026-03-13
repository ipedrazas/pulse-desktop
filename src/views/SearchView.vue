<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);

const query = ref("");
const filePattern = ref("");
const searching = ref(false);
const scope = ref<"project" | "all">("project");

interface SearchResult {
  file: string;
  line_number: number;
  line: string;
  project_name: string | null;
  project_id: string | null;
}

const results = ref<SearchResult[]>([]);

onMounted(async () => {
  if (!project.value) {
    await projectsStore.fetchProjects();
  }
});

async function doSearch() {
  if (!query.value.trim()) return;
  searching.value = true;
  try {
    if (scope.value === "all") {
      results.value = await invoke("search_all_projects", {
        query: query.value,
        filePattern: filePattern.value || null,
      });
    } else {
      if (!project.value) return;
      results.value = await invoke("search_project", {
        rootPath: project.value.root_path,
        query: query.value,
        filePattern: filePattern.value || null,
      });
    }
  } finally {
    searching.value = false;
  }
}

const groupedResults = computed(() => {
  if (scope.value === "all") {
    // Group by project then file
    const groups: Record<string, Record<string, SearchResult[]>> = {};
    for (const r of results.value) {
      const proj = r.project_name || "Unknown";
      if (!groups[proj]) groups[proj] = {};
      if (!groups[proj][r.file]) groups[proj][r.file] = [];
      groups[proj][r.file].push(r);
    }
    return groups;
  } else {
    const groups: Record<string, SearchResult[]> = {};
    for (const r of results.value) {
      if (!groups[r.file]) groups[r.file] = [];
      groups[r.file].push(r);
    }
    return { _: groups };
  }
});

const fileCount = computed(() => {
  const files = new Set(results.value.map((r) => r.file));
  return files.size;
});
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
        class="w-36 px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
      />
      <select
        v-model="scope"
        class="px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-sm text-white"
      >
        <option value="project">This project</option>
        <option value="all">All projects</option>
      </select>
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

    <!-- Results -->
    <template v-for="(projectFiles, projectName) in groupedResults" :key="projectName">
      <div v-if="scope === 'all' && projectName !== '_'" class="text-sm font-medium text-purple-400 mt-4">
        {{ projectName }}
      </div>

      <template v-if="typeof projectFiles === 'object'">
        <div
          v-for="(fileResults, file) in (projectFiles as Record<string, SearchResult[]>)"
          :key="file"
          class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden"
        >
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
      </template>
    </template>
  </div>
</template>
