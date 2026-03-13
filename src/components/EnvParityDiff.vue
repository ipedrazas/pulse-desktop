<script setup lang="ts">
defineProps<{
  parity: {
    missing_keys: string[];
    extra_keys: string[];
    empty_keys: string[];
    example_count: number;
    actual_count: number;
  };
}>();
</script>

<template>
  <div>
    <h2 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
      Environment Parity
    </h2>
    <div class="bg-gray-900 rounded-lg border border-gray-800 p-4 space-y-4">
      <div class="flex gap-6 text-sm">
        <span class="text-gray-500">
          .env.example: <span class="text-gray-300">{{ parity.example_count }} keys</span>
        </span>
        <span class="text-gray-500">
          .env: <span class="text-gray-300">{{ parity.actual_count }} keys</span>
        </span>
      </div>

      <!-- Missing keys -->
      <div v-if="parity.missing_keys.length > 0">
        <div class="text-xs font-medium text-red-400 mb-1">
          Missing from .env ({{ parity.missing_keys.length }})
        </div>
        <div class="flex flex-wrap gap-1">
          <span
            v-for="key in parity.missing_keys"
            :key="key"
            class="px-2 py-0.5 bg-red-900/30 border border-red-800/50 rounded text-xs text-red-300 font-mono"
          >
            {{ key }}
          </span>
        </div>
      </div>

      <!-- Extra keys -->
      <div v-if="parity.extra_keys.length > 0">
        <div class="text-xs font-medium text-blue-400 mb-1">
          Extra in .env ({{ parity.extra_keys.length }})
        </div>
        <div class="flex flex-wrap gap-1">
          <span
            v-for="key in parity.extra_keys"
            :key="key"
            class="px-2 py-0.5 bg-blue-900/30 border border-blue-800/50 rounded text-xs text-blue-300 font-mono"
          >
            {{ key }}
          </span>
        </div>
      </div>

      <!-- Empty keys -->
      <div v-if="parity.empty_keys.length > 0">
        <div class="text-xs font-medium text-yellow-400 mb-1">
          Empty values ({{ parity.empty_keys.length }})
        </div>
        <div class="flex flex-wrap gap-1">
          <span
            v-for="key in parity.empty_keys"
            :key="key"
            class="px-2 py-0.5 bg-yellow-900/30 border border-yellow-800/50 rounded text-xs text-yellow-300 font-mono"
          >
            {{ key }}
          </span>
        </div>
      </div>

      <!-- All good -->
      <div
        v-if="parity.missing_keys.length === 0 && parity.extra_keys.length === 0 && parity.empty_keys.length === 0"
        class="text-sm text-green-400"
      >
        Environment is in sync.
      </div>
    </div>
  </div>
</template>
