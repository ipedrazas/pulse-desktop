<script setup lang="ts">
interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileNode[] | null;
  size: number | null;
}

defineProps<{
  node: FileNode;
  depth: number;
  expandedDirs: Set<string>;
  selectedFile: string | null;
}>();

const emit = defineEmits<{
  (e: "toggle-dir", path: string): void;
  (e: "open-file", path: string): void;
}>();

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  return (bytes / 1024).toFixed(1) + " KB";
}
</script>

<template>
  <div>
    <div
      class="flex items-center gap-1 px-2 py-0.5 text-xs cursor-pointer rounded hover:bg-gray-800/50 transition-colors"
      :class="{ 'bg-blue-600/20 text-blue-400': selectedFile === node.path }"
      :style="{ paddingLeft: depth * 12 + 8 + 'px' }"
      @click="node.is_dir ? emit('toggle-dir', node.path) : emit('open-file', node.path)"
    >
      <span class="text-gray-600 w-4 text-center shrink-0">
        {{ node.is_dir ? (expandedDirs.has(node.path) ? "▼" : "▶") : "" }}
      </span>
      <span :class="node.is_dir ? 'text-gray-400' : 'text-gray-300'">
        {{ node.name }}
      </span>
      <span
        v-if="!node.is_dir && node.size !== null"
        class="text-gray-700 ml-auto text-[10px]"
      >
        {{ formatSize(node.size) }}
      </span>
    </div>
    <template v-if="node.is_dir && expandedDirs.has(node.path) && node.children">
      <TreeNode
        v-for="child in node.children"
        :key="child.path"
        :node="child"
        :depth="depth + 1"
        :expanded-dirs="expandedDirs"
        :selected-file="selectedFile"
        @toggle-dir="emit('toggle-dir', $event)"
        @open-file="emit('open-file', $event)"
      />
    </template>
  </div>
</template>
