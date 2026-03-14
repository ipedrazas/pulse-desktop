<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const projectsStore = useProjectsStore();

interface DiagramResult {
  kind: string;
  mermaid: string;
}

const diagram = ref<DiagramResult | null>(null);
const loading = ref(false);
const activeTab = ref<"folder" | "db">("folder");

async function generateFolderDiagram() {
  if (!projectsStore.currentProject) return;
  loading.value = true;
  activeTab.value = "folder";
  try {
    diagram.value = await invoke("generate_folder_diagram", {
      rootPath: projectsStore.currentProject.root_path,
    });
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function generateDbDiagram() {
  loading.value = true;
  activeTab.value = "db";
  try {
    diagram.value = await invoke("generate_db_diagram");
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function copyMermaid() {
  if (diagram.value) {
    await writeText(diagram.value.mermaid);
  }
}
</script>

<template>
  <div class="p-6 space-y-6 max-w-5xl">
    <h1 class="text-xl font-semibold text-white">Diagrams</h1>

    <div class="flex gap-2">
      <button
        @click="generateFolderDiagram"
        class="px-4 py-2 rounded text-sm font-medium transition-colors"
        :class="
          activeTab === 'folder'
            ? 'bg-blue-600 text-white'
            : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
        "
      >
        Folder Structure
      </button>
      <button
        @click="generateDbDiagram"
        class="px-4 py-2 rounded text-sm font-medium transition-colors"
        :class="
          activeTab === 'db'
            ? 'bg-blue-600 text-white'
            : 'bg-gray-800 text-gray-300 hover:bg-gray-700'
        "
      >
        DB Schema
      </button>
    </div>

    <div v-if="loading" class="text-gray-400 text-sm">Generating diagram...</div>

    <div v-else-if="diagram" class="space-y-4">
      <div class="flex items-center justify-between">
        <span class="text-sm text-gray-400">
          {{ diagram.kind === "folder_structure" ? "Folder Structure" : "Database Schema" }}
          (Mermaid)
        </span>
        <button
          @click="copyMermaid"
          class="px-3 py-1 text-xs bg-gray-800 text-gray-300 rounded hover:bg-gray-700"
        >
          Copy Mermaid
        </button>
      </div>

      <pre
        class="bg-gray-900 border border-gray-800 rounded-lg p-4 text-sm text-gray-300 overflow-auto max-h-[600px] font-mono"
      >{{ diagram.mermaid }}</pre>

      <p class="text-xs text-gray-500">
        Paste into any Mermaid renderer (GitHub, Notion, mermaid.live) to visualize.
      </p>
    </div>

    <div v-else class="text-gray-500 text-sm">
      Select a diagram type to generate Mermaid markup.
    </div>
  </div>
</template>
