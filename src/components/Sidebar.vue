<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useProjectsStore } from "../stores/projects";

const route = useRoute();
const router = useRouter();
const projectsStore = useProjectsStore();

const currentProject = computed(() => projectsStore.currentProject);
const projectId = computed(() => currentProject.value?.id);

const navItems = computed(() => {
  if (!projectId.value) return [];
  return [
    {
      label: "Dashboard",
      route: { name: "project-dashboard", params: { id: projectId.value } },
      icon: "grid",
    },
    {
      label: "Run Center",
      route: { name: "run-center", params: { id: projectId.value } },
      icon: "play",
    },
    {
      label: "Context",
      route: { name: "context-manager", params: { id: projectId.value } },
      icon: "file-text",
    },
  ];
});

function isActive(name: string) {
  return route.name === name;
}
</script>

<template>
  <aside class="w-56 bg-gray-900 border-r border-gray-800 flex flex-col">
    <!-- Brand -->
    <div class="p-4 border-b border-gray-800">
      <button
        class="text-lg font-bold tracking-tight text-white hover:text-blue-400 transition-colors"
        @click="router.push('/')"
      >
        Pulse
      </button>
    </div>

    <!-- Project nav -->
    <nav v-if="currentProject" class="flex-1 py-2">
      <div class="px-4 py-2 text-xs font-medium text-gray-500 uppercase tracking-wider">
        {{ currentProject.name }}
      </div>
      <ul class="space-y-0.5 px-2">
        <li v-for="item in navItems" :key="item.label">
          <router-link
            :to="item.route"
            class="flex items-center gap-2 px-3 py-2 rounded-md text-sm transition-colors"
            :class="
              isActive(item.route.name as string)
                ? 'bg-blue-600/20 text-blue-400'
                : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'
            "
          >
            {{ item.label }}
          </router-link>
        </li>
      </ul>
    </nav>

    <!-- Empty state -->
    <div v-else class="flex-1 flex items-center justify-center px-4">
      <p class="text-sm text-gray-600 text-center">
        Open a project to get started
      </p>
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-gray-800">
      <router-link
        to="/"
        class="text-xs text-gray-500 hover:text-gray-300 transition-colors"
      >
        All Projects
      </router-link>
    </div>
  </aside>
</template>
