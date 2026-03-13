<script setup lang="ts">
defineProps<{
  items: Array<{
    name: string;
    current: string;
    latest: string;
    type: string;
  }>;
}>();
</script>

<template>
  <div v-if="items.length === 0" class="text-sm text-gray-600">
    All dependencies are up to date.
  </div>
  <div v-else class="bg-gray-900 rounded-lg border border-gray-800 overflow-hidden">
    <table class="w-full text-sm">
      <thead>
        <tr class="border-b border-gray-800 text-xs text-gray-500 uppercase">
          <th class="text-left px-4 py-2 font-medium">Package</th>
          <th class="text-left px-4 py-2 font-medium">Current</th>
          <th class="text-left px-4 py-2 font-medium">Latest</th>
          <th class="text-left px-4 py-2 font-medium">Type</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-800/50">
        <tr v-for="item in items" :key="item.name" class="hover:bg-gray-800/50">
          <td class="px-4 py-2 text-white font-mono text-xs">{{ item.name }}</td>
          <td class="px-4 py-2 text-gray-400 font-mono text-xs">{{ item.current }}</td>
          <td class="px-4 py-2 text-yellow-400 font-mono text-xs">{{ item.latest }}</td>
          <td class="px-4 py-2 text-gray-600 text-xs">{{ item.type }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
