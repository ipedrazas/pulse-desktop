<script setup lang="ts">
import { computed } from "vue";
import { useProjectsStore } from "../stores/projects";

const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);
const git = computed(() => project.value?.git);
</script>

<template>
  <footer class="h-7 bg-gray-900 border-t border-gray-800 flex items-center px-4 text-xs text-gray-500 gap-4 shrink-0">
    <template v-if="project">
      <span class="text-gray-400">{{ project.name }}</span>
      <template v-if="git">
        <span>
          <span class="text-gray-600">branch:</span>
          <span class="text-blue-400 ml-1">{{ git.branch }}</span>
        </span>
        <span>
          <span class="text-gray-600">sha:</span>
          <span class="text-gray-400 ml-1 font-mono">{{ git.sha }}</span>
        </span>
        <span v-if="git.dirty" class="text-yellow-500">modified</span>
      </template>
      <span v-if="project.project_type" class="ml-auto text-gray-600">
        {{ project.project_type }}
      </span>
    </template>
    <template v-else>
      <span>No project open</span>
    </template>
  </footer>
</template>
