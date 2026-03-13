<script setup lang="ts">
defineProps<{ command: string }>();
defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
    <div class="bg-gray-900 border border-gray-700 rounded-lg p-6 max-w-md w-full mx-4 shadow-xl">
      <h3 class="text-lg font-medium text-white mb-2">Confirm Execution</h3>
      <p class="text-sm text-gray-400 mb-4">
        This step requires your approval before running:
      </p>
      <code class="block bg-gray-950 rounded p-3 text-sm text-yellow-400 font-mono mb-6">
        {{ command }}
      </code>
      <div class="flex justify-end gap-3">
        <button
          @click="$emit('cancel')"
          class="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 transition-colors"
        >
          Cancel
        </button>
        <button
          @click="$emit('confirm')"
          class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-lg transition-colors"
        >
          Run
        </button>
      </div>
    </div>
  </div>
</template>
