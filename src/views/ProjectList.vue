<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectsStore } from "../stores/projects";

const router = useRouter();
const store = useProjectsStore();

onMounted(() => {
  store.fetchProjects();
});

async function openFolder() {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    const info = await store.openProject(selected as string);
    router.push({ name: "project-dashboard", params: { id: info.id } });
  }
}

async function openRecent(projectId: string, rootPath: string) {
  await store.openProject(rootPath);
  router.push({ name: "project-dashboard", params: { id: projectId } });
}

async function remove(projectId: string, event: Event) {
  event.stopPropagation();
  await store.removeProject(projectId);
}

function timeAgo(dateStr: string): string {
  const date = new Date(dateStr + "Z");
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}
</script>

<template>
  <div class="max-w-2xl mx-auto">
    <div class="flex items-center justify-between mb-8">
      <h1 class="text-2xl font-bold text-white">Projects</h1>
      <button
        @click="openFolder"
        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors"
      >
        Open Folder
      </button>
    </div>

    <!-- Loading -->
    <div v-if="store.loading" class="text-gray-500 text-sm">Loading...</div>

    <!-- Empty state -->
    <div
      v-else-if="store.projects.length === 0"
      class="text-center py-16 text-gray-500"
    >
      <p class="text-lg mb-2">No projects yet</p>
      <p class="text-sm">Open a folder to register your first project.</p>
    </div>

    <!-- Project list -->
    <ul v-else class="space-y-2">
      <li
        v-for="project in store.projects"
        :key="project.id"
        @click="openRecent(project.id, project.root_path)"
        class="flex items-center justify-between p-4 bg-gray-900 rounded-lg border border-gray-800 hover:border-gray-700 cursor-pointer transition-colors group"
      >
        <div class="min-w-0">
          <div class="font-medium text-white truncate">{{ project.name }}</div>
          <div class="text-xs text-gray-500 truncate mt-0.5">
            {{ project.root_path }}
          </div>
        </div>
        <div class="flex items-center gap-3 shrink-0 ml-4">
          <span class="text-xs text-gray-600">
            {{ timeAgo(project.last_opened) }}
          </span>
          <button
            @click="remove(project.id, $event)"
            class="text-gray-700 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all text-sm"
            title="Remove project"
          >
            &times;
          </button>
        </div>
      </li>
    </ul>
  </div>
</template>
