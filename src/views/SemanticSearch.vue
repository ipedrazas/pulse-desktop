<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

const projectsStore = useProjectsStore();

interface SemanticChunk {
  id: number;
  file_path: string;
  chunk_type: string;
  name: string | null;
  content: string;
  language: string | null;
  start_line: number | null;
  end_line: number | null;
  keywords: string | null;
}

interface SemanticSearchResult {
  chunk: SemanticChunk;
  score: number;
}

const query = ref("");
const results = ref<SemanticSearchResult[]>([]);
const searching = ref(false);
const indexing = ref(false);
const indexedCount = ref<number | null>(null);
const expandedId = ref<number | null>(null);

async function indexProject() {
  if (!projectsStore.currentProject) return;
  indexing.value = true;
  try {
    indexedCount.value = await invoke("index_project_semantics", {
      projectId: projectsStore.currentProject.id,
      rootPath: projectsStore.currentProject.root_path,
    });
  } catch (e) {
    console.error(e);
  } finally {
    indexing.value = false;
  }
}

async function search() {
  if (!query.value.trim() || !projectsStore.currentProject) return;
  searching.value = true;
  try {
    results.value = await invoke("semantic_search", {
      projectId: projectsStore.currentProject.id,
      query: query.value.trim(),
    });
  } catch (e) {
    console.error(e);
  } finally {
    searching.value = false;
  }
}

function toggleExpand(id: number) {
  expandedId.value = expandedId.value === id ? null : id;
}

function scoreColor(score: number): string {
  if (score >= 0.7) return "text-green-400";
  if (score >= 0.4) return "text-yellow-400";
  return "text-gray-400";
}
</script>

<template>
  <div class="p-6 space-y-6 max-w-5xl">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold text-white">Semantic Search</h1>
      <button
        @click="indexProject"
        :disabled="indexing"
        class="px-3 py-1.5 text-sm bg-gray-800 text-gray-300 rounded hover:bg-gray-700 disabled:opacity-50"
      >
        {{ indexing ? "Indexing..." : "Re-index Project" }}
      </button>
    </div>

    <p v-if="indexedCount !== null" class="text-xs text-gray-500">
      Indexed {{ indexedCount }} chunks
    </p>

    <!-- Search input -->
    <div class="flex gap-2">
      <input
        v-model="query"
        @keyup.enter="search"
        placeholder="Search by concept, e.g. 'authentication middleware' or 'database connection'"
        class="flex-1 bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
      />
      <button
        @click="search"
        :disabled="searching || !query.trim()"
        class="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-500 disabled:opacity-50"
      >
        {{ searching ? "..." : "Search" }}
      </button>
    </div>

    <!-- Results -->
    <div v-if="results.length > 0" class="space-y-3">
      <p class="text-xs text-gray-500">{{ results.length }} results</p>

      <div
        v-for="result in results"
        :key="result.chunk.id"
        class="bg-gray-900 border border-gray-800 rounded-lg overflow-hidden"
      >
        <button
          @click="toggleExpand(result.chunk.id)"
          class="w-full px-4 py-3 flex items-center justify-between text-left hover:bg-gray-800/50"
        >
          <div class="flex items-center gap-3 min-w-0">
            <span class="text-sm text-gray-300 truncate">{{ result.chunk.file_path }}</span>
            <span
              v-if="result.chunk.start_line"
              class="text-xs text-gray-600"
            >
              :{{ result.chunk.start_line }}-{{ result.chunk.end_line }}
            </span>
            <span
              v-if="result.chunk.language"
              class="text-[10px] px-1.5 py-0.5 bg-gray-800 text-gray-500 rounded"
            >
              {{ result.chunk.language }}
            </span>
          </div>
          <span :class="scoreColor(result.score)" class="text-xs font-mono">
            {{ (result.score * 100).toFixed(0) }}%
          </span>
        </button>

        <div v-if="expandedId === result.chunk.id" class="border-t border-gray-800">
          <pre
            class="p-4 text-xs text-gray-300 overflow-auto max-h-64 font-mono"
          >{{ result.chunk.content }}</pre>
          <div class="px-4 py-2 border-t border-gray-800 text-xs text-gray-600">
            Keywords: {{ result.chunk.keywords }}
          </div>
        </div>
      </div>
    </div>

    <div
      v-else-if="query && !searching"
      class="text-gray-500 text-sm"
    >
      No results. Try re-indexing the project first.
    </div>
  </div>
</template>
