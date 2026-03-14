<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";
import TreeNode from "../components/TreeNode.vue";

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
const loadingTree = ref(false);
const treeError = ref<string | null>(null);
const expandedDirs = ref<Set<string>>(new Set());

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await loadFileTree();
});

async function loadFileTree() {
  if (!project.value) return;
  loadingTree.value = true;
  treeError.value = null;
  try {
    files.value = await invoke("get_file_tree", {
      rootPath: project.value.root_path,
      maxDepth: 5,
    });
  } catch (e) {
    treeError.value = String(e);
  } finally {
    loadingTree.value = false;
  }
}

// Build tree from flat list — parents always appear before children
// because the walker emits entries in depth-first order.
const fileTree = computed(() => {
  const root: FileNode[] = [];
  const dirMap = new Map<string, FileNode>();

  for (const f of files.value) {
    const parts = f.path.split("/");
    const node: FileNode = { ...f, children: f.is_dir ? [] : null };

    if (parts.length === 1) {
      root.push(node);
    } else {
      const parentPath = parts.slice(0, -1).join("/");
      const parent = dirMap.get(parentPath);
      if (parent && parent.children) {
        parent.children.push(node);
      } else {
        // Orphan — add to root (shouldn't happen with correct sort)
        root.push(node);
      }
    }

    if (f.is_dir) {
      dirMap.set(f.path, node);
    }
  }

  return root;
});

function toggleDir(path: string) {
  const next = new Set(expandedDirs.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  expandedDirs.value = next;
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

function fileExtension(path: string): string {
  const parts = path.split(".");
  return parts.length > 1 ? parts[parts.length - 1] : "";
}
</script>

<template>
  <div class="flex h-full -m-6">
    <!-- Tree panel -->
    <div class="w-72 border-r border-gray-800 overflow-y-auto p-2 shrink-0">
      <div class="text-xs text-gray-500 uppercase tracking-wider px-2 py-1 mb-1">
        Files
      </div>

      <div v-if="loadingTree" class="text-sm text-gray-600 px-2">
        Loading file tree...
      </div>
      <div v-else-if="treeError" class="text-sm text-red-400 px-2">
        {{ treeError }}
      </div>
      <div v-else-if="fileTree.length === 0" class="text-sm text-gray-600 px-2">
        No files found.
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
        <span class="text-[10px] text-gray-600 ml-auto">{{ fileExtension(selectedFile) }}</span>
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
