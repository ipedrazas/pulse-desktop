<script setup lang="ts">
import { ref, nextTick, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProjectsStore } from "../stores/projects";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import mermaid from "mermaid";

mermaid.initialize({
  startOnLoad: false,
  theme: "dark",
  themeVariables: {
    darkMode: true,
    background: "#111827",
    primaryColor: "#1e40af",
    primaryTextColor: "#e5e7eb",
    primaryBorderColor: "#374151",
    lineColor: "#6b7280",
    secondaryColor: "#1f2937",
    tertiaryColor: "#1f2937",
    fontSize: "14px",
  },
});

const props = defineProps<{ id: string }>();
const projectsStore = useProjectsStore();
const project = computed(() => projectsStore.currentProject);

interface DiagramEntry {
  title: string;
  source_file: string | null;
  mermaid: string;
}

const diagrams = ref<DiagramEntry[]>([]);
const loading = ref(false);
const selectedIndex = ref<number | null>(null);
const viewMode = ref<"source" | "rendered">("rendered");
const renderedSvg = ref("");
const renderError = ref<string | null>(null);
const saving = ref(false);
const saveFileName = ref("");
const showSaveDialog = ref(false);
const saveSuccess = ref<string | null>(null);

// Pan & zoom state
const zoom = ref(1);
const panX = ref(0);
const panY = ref(0);
const isPanning = ref(false);
const panStart = ref({ x: 0, y: 0 });
const diagramContainer = ref<HTMLElement | null>(null);

function resetView() {
  zoom.value = 1;
  panX.value = 0;
  panY.value = 0;
}

function zoomIn() {
  zoom.value = Math.min(zoom.value * 1.25, 5);
}

function zoomOut() {
  zoom.value = Math.max(zoom.value / 1.25, 0.1);
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY > 0 ? 1 / 1.15 : 1.15;
  const newZoom = Math.max(0.1, Math.min(5, zoom.value * delta));

  // Zoom toward cursor position
  if (diagramContainer.value) {
    const rect = diagramContainer.value.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    panX.value = cx - (cx - panX.value) * (newZoom / zoom.value);
    panY.value = cy - (cy - panY.value) * (newZoom / zoom.value);
  }

  zoom.value = newZoom;
}

function onPointerDown(e: PointerEvent) {
  isPanning.value = true;
  panStart.value = { x: e.clientX - panX.value, y: e.clientY - panY.value };
  (e.target as HTMLElement)?.setPointerCapture?.(e.pointerId);
}

function onPointerMove(e: PointerEvent) {
  if (!isPanning.value) return;
  panX.value = e.clientX - panStart.value.x;
  panY.value = e.clientY - panStart.value.y;
}

function onPointerUp() {
  isPanning.value = false;
}

const selected = computed(() =>
  selectedIndex.value !== null ? diagrams.value[selectedIndex.value] : null,
);

onMounted(async () => {
  if (!project.value || project.value.id !== props.id) {
    await projectsStore.loadProject(props.id);
  }
  await loadDiagrams();
});

async function loadDiagrams() {
  if (!project.value) return;
  loading.value = true;
  try {
    // Extract from markdown files
    const extracted: DiagramEntry[] = await invoke("extract_mermaid_diagrams", {
      rootPath: project.value.root_path,
    });

    // Add Pulse DB schema
    const dbDiagram: DiagramEntry = await invoke("generate_db_diagram");

    diagrams.value = [...extracted, dbDiagram];

    if (diagrams.value.length > 0) {
      await selectDiagram(0);
    }
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function selectDiagram(index: number) {
  selectedIndex.value = index;
  renderedSvg.value = "";
  renderError.value = null;
  saveSuccess.value = null;
  showSaveDialog.value = false;
  resetView();

  if (viewMode.value === "rendered") {
    await renderDiagram();
  }
}

async function renderDiagram() {
  const d = selected.value;
  if (!d) return;
  renderError.value = null;
  viewMode.value = "rendered";

  await nextTick();
  try {
    const id = "mermaid-" + Date.now();
    const { svg } = await mermaid.render(id, d.mermaid);
    renderedSvg.value = svg;
  } catch (e) {
    renderError.value = String(e);
    renderedSvg.value = "";
  }
}

async function setViewMode(mode: "source" | "rendered") {
  viewMode.value = mode;
  if (mode === "rendered") {
    await renderDiagram();
  }
}

async function copyMermaid() {
  if (selected.value) {
    await writeText(selected.value.mermaid);
  }
}

function openSaveDialog() {
  if (!selected.value) return;
  // Suggest a filename based on the title
  const slug = selected.value.title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  saveFileName.value = `${slug}.md`;
  showSaveDialog.value = true;
  saveSuccess.value = null;
}

async function saveDiagram() {
  if (!project.value || !selected.value || !saveFileName.value.trim()) return;
  saving.value = true;
  try {
    const savedName: string = await invoke("save_diagram_to_file", {
      rootPath: project.value.root_path,
      fileName: saveFileName.value.trim(),
      mermaidContent: selected.value.mermaid,
    });
    saveSuccess.value = savedName;
    showSaveDialog.value = false;
  } catch (e) {
    console.error(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="flex h-full -m-6">
    <!-- Diagram list panel -->
    <div class="w-72 border-r border-gray-800 overflow-y-auto flex flex-col shrink-0">
      <div class="px-4 py-3 border-b border-gray-800 flex items-center justify-between">
        <span class="text-xs text-gray-500 uppercase tracking-wider">Diagrams</span>
        <button
          @click="loadDiagrams"
          :disabled="loading"
          class="text-xs text-gray-400 hover:text-white transition-colors"
        >
          {{ loading ? "..." : "Refresh" }}
        </button>
      </div>

      <div v-if="loading" class="p-4 text-sm text-gray-600">Scanning project...</div>

      <div v-else-if="diagrams.length === 0" class="p-4 text-sm text-gray-600">
        No diagrams found. Add <code class="text-gray-400">```mermaid</code> blocks to your markdown files.
      </div>

      <div v-else class="flex-1 py-1">
        <button
          v-for="(d, i) in diagrams"
          :key="i"
          @click="selectDiagram(i)"
          class="w-full text-left px-4 py-2 text-sm transition-colors"
          :class="
            selectedIndex === i
              ? 'bg-blue-600/20 text-blue-400'
              : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'
          "
        >
          <div class="truncate">{{ d.title }}</div>
          <div v-if="d.source_file" class="text-[10px] text-gray-600 truncate mt-0.5">
            {{ d.source_file }}
          </div>
          <div v-else class="text-[10px] text-purple-600 mt-0.5">generated</div>
        </button>
      </div>
    </div>

    <!-- Diagram viewer -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <template v-if="selected">
        <!-- Toolbar -->
        <div class="border-b border-gray-800 px-4 py-2 flex items-center justify-between shrink-0">
          <span class="text-sm text-white font-medium truncate">{{ selected.title }}</span>
          <div class="flex gap-2 shrink-0">
            <button
              @click="setViewMode('rendered')"
              class="px-3 py-1 text-xs rounded transition-colors"
              :class="
                viewMode === 'rendered'
                  ? 'bg-gray-700 text-white'
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
              "
            >
              View
            </button>
            <button
              @click="setViewMode('source')"
              class="px-3 py-1 text-xs rounded transition-colors"
              :class="
                viewMode === 'source'
                  ? 'bg-gray-700 text-white'
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'
              "
            >
              Source
            </button>
            <button
              @click="copyMermaid"
              class="px-3 py-1 text-xs bg-gray-800 text-gray-400 rounded hover:bg-gray-700"
            >
              Copy
            </button>
            <button
              @click="openSaveDialog"
              class="px-3 py-1 text-xs bg-gray-800 text-gray-400 rounded hover:bg-gray-700"
            >
              Save to file
            </button>
          </div>
        </div>

        <!-- Save dialog -->
        <div
          v-if="showSaveDialog"
          class="border-b border-gray-800 px-4 py-3 bg-gray-900/50 flex items-center gap-3"
        >
          <label class="text-xs text-gray-400 shrink-0">File name:</label>
          <input
            v-model="saveFileName"
            @keyup.enter="saveDiagram"
            class="flex-1 bg-gray-800 text-white text-sm rounded px-3 py-1.5 outline-none focus:ring-1 focus:ring-blue-500 font-mono"
          />
          <button
            @click="saveDiagram"
            :disabled="saving || !saveFileName.trim()"
            class="px-3 py-1.5 text-xs bg-green-600 text-white rounded hover:bg-green-500 disabled:opacity-50"
          >
            {{ saving ? "..." : "Save" }}
          </button>
          <button
            @click="showSaveDialog = false"
            class="text-xs text-gray-500 hover:text-gray-300"
          >
            Cancel
          </button>
        </div>

        <!-- Save success -->
        <div
          v-if="saveSuccess"
          class="border-b border-gray-800 px-4 py-2 bg-green-900/20 text-xs text-green-400"
        >
          Saved to {{ saveSuccess }}
        </div>

        <!-- Rendered view -->
        <div
          v-if="viewMode === 'rendered'"
          class="flex-1 overflow-hidden relative"
        >
          <div
            v-if="renderError"
            class="p-4 m-4 bg-red-900/20 border border-red-800/50 rounded-lg text-sm text-red-400"
          >
            <p class="font-medium mb-1">Render error</p>
            <pre class="text-xs overflow-auto">{{ renderError }}</pre>
          </div>
          <div
            v-else-if="renderedSvg"
            ref="diagramContainer"
            class="w-full h-full cursor-grab active:cursor-grabbing select-none"
            @wheel="onWheel"
            @pointerdown="onPointerDown"
            @pointermove="onPointerMove"
            @pointerup="onPointerUp"
          >
            <div
              :style="{
                transform: `translate(${panX}px, ${panY}px) scale(${zoom})`,
                transformOrigin: '0 0',
              }"
              class="inline-block p-6"
              v-html="renderedSvg"
            />
          </div>
          <div v-else class="flex-1 flex items-center justify-center text-gray-500 text-sm p-8">
            Rendering...
          </div>

          <!-- Zoom controls -->
          <div
            v-if="renderedSvg && !renderError"
            class="absolute bottom-4 right-4 flex items-center gap-1 bg-gray-900/90 border border-gray-700 rounded-lg px-1 py-1"
          >
            <button
              @click="zoomOut"
              class="w-7 h-7 flex items-center justify-center text-gray-400 hover:text-white rounded hover:bg-gray-700 transition-colors text-sm"
            >
              -
            </button>
            <button
              @click="resetView"
              class="px-2 h-7 flex items-center justify-center text-xs text-gray-400 hover:text-white rounded hover:bg-gray-700 transition-colors font-mono min-w-[3rem]"
            >
              {{ Math.round(zoom * 100) }}%
            </button>
            <button
              @click="zoomIn"
              class="w-7 h-7 flex items-center justify-center text-gray-400 hover:text-white rounded hover:bg-gray-700 transition-colors text-sm"
            >
              +
            </button>
          </div>
        </div>

        <!-- Source view -->
        <pre
          v-else
          class="flex-1 overflow-auto p-4 text-sm text-gray-300 font-mono leading-6"
        >{{ selected.mermaid }}</pre>
      </template>

      <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-sm">
        Select a diagram from the list
      </div>
    </div>
  </div>
</template>
