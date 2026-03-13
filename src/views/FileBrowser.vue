<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);

interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileNode[] | null;
  size: number | null;
}

const files = ref<FileNode[]>([]);
const selectedFile = ref<string | null>(null);
const fileContent = ref<string>("");
const loadingContent = ref(false);
const expandedDirs = ref<Set<string>>(new Set());

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  if (project.value) {
    files.value = await invoke("get_file_tree", {
      rootPath: project.value.root_path,
      maxDepth: 5,
    });
  }
});

// Build tree from flat list
const fileTree = computed(() => {
  const root: FileNode[] = [];
  const dirMap = new Map<string, FileNode>();

  for (const f of files.value) {
    const parts = f.path.split("/");
    if (parts.length === 1) {
      const node = { ...f, children: f.is_dir ? [] : null };
      root.push(node);
      if (f.is_dir) dirMap.set(f.path, node);
    } else {
      const parentPath = parts.slice(0, -1).join("/");
      const parent = dirMap.get(parentPath);
      if (parent && parent.children) {
        const node = { ...f, children: f.is_dir ? [] : null };
        parent.children.push(node);
        if (f.is_dir) dirMap.set(f.path, node);
      }
    }
  }

  return root;
});

function toggleDir(path: string) {
  if (expandedDirs.value.has(path)) {
    expandedDirs.value.delete(path);
  } else {
    expandedDirs.value.add(path);
  }
  expandedDirs.value = new Set(expandedDirs.value);
}

async function openFile(filePath: string) {
  if (!project.value) return;
  selectedFile.value = filePath;
  loadingContent.value = true;
  try {
    fileContent.value = await invoke("read_file_content", {
      rootPath: project.value.root_path,
      filePath,
    });
  } catch (e) {
    fileContent.value = `Error: ${e}`;
  } finally {
    loadingContent.value = false;
  }
}

</script>

<template>
  <div class="flex h-full -m-6">
    <!-- Tree panel -->
    <div class="w-72 border-r border-gray-800 overflow-y-auto p-2 shrink-0">
      <div class="text-xs text-gray-500 uppercase tracking-wider px-2 py-1 mb-1">
        Files
      </div>
      <div v-if="files.length === 0" class="text-sm text-gray-600 px-2">
        Loading...
      </div>
      <template v-else>
        <TreeNode
          v-for="node in fileTree"
          :key="node.path"
          :node="node"
          :depth="0"
          :expanded-dirs="expandedDirs"
          :selected-file="selectedFile"
          @toggle-dir="toggleDir"
          @open-file="openFile"
        />
      </template>
    </div>

    <!-- Content panel -->
    <div class="flex-1 overflow-hidden flex flex-col">
      <div v-if="selectedFile" class="border-b border-gray-800 px-4 py-2 flex items-center gap-2 shrink-0">
        <span class="text-sm font-mono text-blue-400">{{ selectedFile }}</span>
      </div>
      <div v-if="loadingContent" class="flex-1 flex items-center justify-center text-gray-500 text-sm">
        Loading...
      </div>
      <pre
        v-else-if="selectedFile"
        class="flex-1 overflow-auto p-4 text-xs font-mono text-gray-300 leading-5"
      >{{ fileContent }}</pre>
      <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">
        Select a file to view its contents
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, type PropType } from "vue";

interface FileNodeType {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileNodeType[] | null;
  size: number | null;
}

const TreeNode = defineComponent({
  name: "TreeNode",
  props: {
    node: { type: Object as PropType<FileNodeType>, required: true },
    depth: { type: Number, default: 0 },
    expandedDirs: { type: Object as PropType<Set<string>>, required: true },
    selectedFile: { type: String as PropType<string | null>, default: null },
  },
  emits: ["toggle-dir", "open-file"],
  template: `
    <div>
      <div
        class="flex items-center gap-1 px-2 py-0.5 text-xs cursor-pointer rounded hover:bg-gray-800/50 transition-colors"
        :class="{ 'bg-blue-600/20 text-blue-400': selectedFile === node.path }"
        :style="{ paddingLeft: (depth * 12 + 8) + 'px' }"
        @click="node.is_dir ? $emit('toggle-dir', node.path) : $emit('open-file', node.path)"
      >
        <span class="text-gray-600 w-4 text-center shrink-0">
          {{ node.is_dir ? (expandedDirs.has(node.path) ? '▼' : '▶') : '' }}
        </span>
        <span :class="node.is_dir ? 'text-gray-400' : 'text-gray-300'">
          {{ node.name }}
        </span>
        <span v-if="!node.is_dir && node.size !== null" class="text-gray-700 ml-auto text-[10px]">
          {{ node.size < 1024 ? node.size + ' B' : (node.size / 1024).toFixed(1) + ' KB' }}
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
          @toggle-dir="$emit('toggle-dir', $event)"
          @open-file="$emit('open-file', $event)"
        />
      </template>
    </div>
  `,
});

export default { components: { TreeNode } };
</script>
