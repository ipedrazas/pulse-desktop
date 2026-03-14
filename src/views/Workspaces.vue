<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

const projectsStore = useProjectsStore();

interface Workspace {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
}

interface WorkspaceWithProjects {
  workspace: Workspace;
  project_ids: string[];
}

const workspaces = ref<WorkspaceWithProjects[]>([]);
const newName = ref("");
const newDesc = ref("");
const loading = ref(false);
const showCreate = ref(false);

async function fetchWorkspaces() {
  loading.value = true;
  try {
    workspaces.value = await invoke("list_workspaces");
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function createWorkspace() {
  if (!newName.value.trim()) return;
  try {
    await invoke("create_workspace", {
      name: newName.value.trim(),
      description: newDesc.value.trim() || null,
    });
    newName.value = "";
    newDesc.value = "";
    showCreate.value = false;
    await fetchWorkspaces();
  } catch (e) {
    console.error(e);
  }
}

async function deleteWorkspace(id: string) {
  try {
    await invoke("delete_workspace", { workspaceId: id });
    await fetchWorkspaces();
  } catch (e) {
    console.error(e);
  }
}

async function addCurrentProject(workspaceId: string) {
  if (!projectsStore.currentProject) return;
  try {
    await invoke("add_project_to_workspace", {
      workspaceId,
      projectId: projectsStore.currentProject.id,
    });
    await fetchWorkspaces();
  } catch (e) {
    console.error(e);
  }
}

async function removeProjectFromWorkspace(workspaceId: string, projectId: string) {
  try {
    await invoke("remove_project_from_workspace", { workspaceId, projectId });
    await fetchWorkspaces();
  } catch (e) {
    console.error(e);
  }
}

function projectName(id: string): string {
  const p = projectsStore.projects.find((p) => p.id === id);
  return p?.name ?? id.slice(0, 8);
}

onMounted(fetchWorkspaces);
</script>

<template>
  <div class="p-6 space-y-6 max-w-4xl">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold text-white">Workspaces</h1>
      <button
        @click="showCreate = !showCreate"
        class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-500"
      >
        {{ showCreate ? "Cancel" : "New Workspace" }}
      </button>
    </div>

    <!-- Create form -->
    <div v-if="showCreate" class="bg-gray-900 border border-gray-800 rounded-lg p-4 space-y-3">
      <input
        v-model="newName"
        placeholder="Workspace name"
        class="w-full bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
      />
      <input
        v-model="newDesc"
        placeholder="Description (optional)"
        class="w-full bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
      />
      <button
        @click="createWorkspace"
        :disabled="!newName.trim()"
        class="px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-500 disabled:opacity-50"
      >
        Create
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-gray-400 text-sm">Loading workspaces...</div>

    <!-- Workspace list -->
    <div v-else-if="workspaces.length === 0" class="text-gray-500 text-sm">
      No workspaces yet. Create one to group your projects.
    </div>

    <div v-else class="space-y-4">
      <div
        v-for="ws in workspaces"
        :key="ws.workspace.id"
        class="bg-gray-900 border border-gray-800 rounded-lg p-4"
      >
        <div class="flex items-start justify-between">
          <div>
            <h3 class="text-white font-medium">{{ ws.workspace.name }}</h3>
            <p v-if="ws.workspace.description" class="text-sm text-gray-400 mt-0.5">
              {{ ws.workspace.description }}
            </p>
          </div>
          <button
            @click="deleteWorkspace(ws.workspace.id)"
            class="text-xs text-red-400 hover:text-red-300"
          >
            Delete
          </button>
        </div>

        <!-- Projects in workspace -->
        <div class="mt-3 space-y-1">
          <div
            v-for="pid in ws.project_ids"
            :key="pid"
            class="flex items-center justify-between px-2 py-1 bg-gray-800 rounded text-sm"
          >
            <span class="text-gray-300">{{ projectName(pid) }}</span>
            <button
              @click="removeProjectFromWorkspace(ws.workspace.id, pid)"
              class="text-xs text-gray-500 hover:text-red-400"
            >
              Remove
            </button>
          </div>
          <div v-if="ws.project_ids.length === 0" class="text-xs text-gray-600">
            No projects in this workspace
          </div>
        </div>

        <!-- Add current project -->
        <button
          v-if="
            projectsStore.currentProject &&
            !ws.project_ids.includes(projectsStore.currentProject.id)
          "
          @click="addCurrentProject(ws.workspace.id)"
          class="mt-2 text-xs text-blue-400 hover:text-blue-300"
        >
          + Add current project
        </button>
      </div>
    </div>
  </div>
</template>
