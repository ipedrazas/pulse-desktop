<script setup lang="ts">
import Sidebar from "../components/Sidebar.vue";
import StatusBar from "../components/StatusBar.vue";
</script>

<template>
  <div class="flex h-screen bg-gray-950 text-gray-100">
    <Sidebar />
    <div class="flex flex-col flex-1 overflow-hidden">
      <main class="flex-1 overflow-y-auto p-6">
        <router-view />
      </main>
      <StatusBar />
    </div>
  </div>
</template>
