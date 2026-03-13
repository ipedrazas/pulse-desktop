<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useProjectsStore } from "../stores/projects";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();

const project = computed(() => projectsStore.currentProject);
const bundle = ref<{
  files: Array<{ path: string; content: string; exists: boolean }>;
  git_branch: string | null;
  git_sha: string | null;
  git_dirty: boolean | null;
  total_bytes: number;
} | null>(null);
const copied = ref(false);
const selectedFiles = ref<Set<string>>(new Set());

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  if (project.value) {
    bundle.value = await invoke("get_context_files", {
      rootPath: project.value.root_path,
    });
    // Select all files by default
    if (bundle.value) {
      for (const f of bundle.value.files) {
        if (f.exists) selectedFiles.value.add(f.path);
      }
    }
  }
});

function toggleFile(path: string) {
  if (selectedFiles.value.has(path)) {
    selectedFiles.value.delete(path);
  } else {
    selectedFiles.value.add(path);
  }
}

async function copyBundle() {
  if (!project.value) return;
  const text: string = await invoke("build_context_string", {
    rootPath: project.value.root_path,
  });
  await writeText(text);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-white">Context Manager</h1>
      <button
        @click="copyBundle"
        class="px-4 py-2 text-sm font-medium rounded-lg transition-colors"
        :class="
          copied
            ? 'bg-green-600 text-white'
            : 'bg-blue-600 hover:bg-blue-700 text-white'
        "
      >
        {{ copied ? "Copied!" : "Copy Bundle" }}
      </button>
    </div>

    <div v-if="!bundle" class="text-gray-500 text-sm">
      No .pulse.yaml found or no context files defined.
    </div>

    <template v-else>
      <!-- Git info -->
      <div
        v-if="bundle.git_branch"
        class="text-sm text-gray-400 flex items-center gap-4"
      >
        <span>
          <span class="text-gray-600">Branch:</span>
          <span class="text-blue-400 ml-1">{{ bundle.git_branch }}</span>
        </span>
        <span v-if="bundle.git_sha">
          <span class="text-gray-600">SHA:</span>
          <span class="ml-1 font-mono">{{ bundle.git_sha }}</span>
        </span>
        <span class="ml-auto text-gray-600">
          Total: {{ formatBytes(bundle.total_bytes) }}
        </span>
      </div>

      <!-- File list -->
      <div class="space-y-3">
        <div
          v-for="file in bundle.files"
          :key="file.path"
          class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden"
        >
          <div
            class="flex items-center justify-between px-4 py-2 border-b border-gray-800 cursor-pointer"
            @click="toggleFile(file.path)"
          >
            <div class="flex items-center gap-2">
              <input
                type="checkbox"
                :checked="selectedFiles.has(file.path)"
                class="rounded border-gray-700"
                @click.stop
                @change="toggleFile(file.path)"
              />
              <span class="text-sm font-mono" :class="file.exists ? 'text-gray-300' : 'text-red-400'">
                {{ file.path }}
              </span>
            </div>
            <span v-if="!file.exists" class="text-xs text-red-400">missing</span>
            <span v-else class="text-xs text-gray-600">
              {{ formatBytes(file.content.length) }}
            </span>
          </div>
          <pre
            v-if="file.exists && selectedFiles.has(file.path)"
            class="p-4 text-xs text-gray-400 overflow-x-auto max-h-64 overflow-y-auto font-mono"
          >{{ file.content }}</pre>
        </div>
      </div>
    </template>
  </div>
</template>
