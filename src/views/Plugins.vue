<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface Plugin {
  id: string;
  name: string;
  description: string | null;
  version: string | null;
  entry_point: string;
  plugin_type: string;
  enabled: number;
  config: string | null;
  installed_at: string;
}

const plugins = ref<Plugin[]>([]);
const loading = ref(false);
const showRegister = ref(false);

const form = ref({
  name: "",
  description: "",
  version: "",
  entry_point: "",
  plugin_type: "command",
  config: "",
});

async function fetchPlugins() {
  loading.value = true;
  try {
    plugins.value = await invoke("list_plugins");
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
}

async function registerPlugin() {
  if (!form.value.name.trim() || !form.value.entry_point.trim()) return;
  try {
    await invoke("register_plugin", {
      name: form.value.name.trim(),
      description: form.value.description.trim() || null,
      version: form.value.version.trim() || null,
      entryPoint: form.value.entry_point.trim(),
      pluginType: form.value.plugin_type,
      config: form.value.config.trim() || null,
    });
    form.value = { name: "", description: "", version: "", entry_point: "", plugin_type: "command", config: "" };
    showRegister.value = false;
    await fetchPlugins();
  } catch (e) {
    console.error(e);
  }
}

async function togglePlugin(id: string, currentlyEnabled: number) {
  try {
    await invoke("toggle_plugin", { pluginId: id, enabled: currentlyEnabled === 0 });
    await fetchPlugins();
  } catch (e) {
    console.error(e);
  }
}

async function removePlugin(id: string) {
  try {
    await invoke("remove_plugin", { pluginId: id });
    await fetchPlugins();
  } catch (e) {
    console.error(e);
  }
}

onMounted(fetchPlugins);
</script>

<template>
  <div class="p-6 space-y-6 max-w-4xl">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold text-white">Plugins</h1>
      <button
        @click="showRegister = !showRegister"
        class="px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-500"
      >
        {{ showRegister ? "Cancel" : "Register Plugin" }}
      </button>
    </div>

    <!-- Register form -->
    <div v-if="showRegister" class="bg-gray-900 border border-gray-800 rounded-lg p-4 space-y-3">
      <input
        v-model="form.name"
        placeholder="Plugin name"
        class="w-full bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
      />
      <input
        v-model="form.description"
        placeholder="Description (optional)"
        class="w-full bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
      />
      <div class="flex gap-3">
        <input
          v-model="form.version"
          placeholder="Version"
          class="flex-1 bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
        />
        <select
          v-model="form.plugin_type"
          class="bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none"
        >
          <option value="command">Command</option>
          <option value="checker">Checker</option>
          <option value="connector">Connector</option>
        </select>
      </div>
      <input
        v-model="form.entry_point"
        placeholder="Entry point (script path or command)"
        class="w-full bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500"
      />
      <textarea
        v-model="form.config"
        placeholder="Config JSON (optional)"
        rows="2"
        class="w-full bg-gray-800 text-white text-sm rounded px-3 py-2 outline-none focus:ring-1 focus:ring-blue-500 font-mono"
      />
      <button
        @click="registerPlugin"
        :disabled="!form.name.trim() || !form.entry_point.trim()"
        class="px-4 py-2 text-sm bg-green-600 text-white rounded hover:bg-green-500 disabled:opacity-50"
      >
        Register
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-gray-400 text-sm">Loading plugins...</div>

    <!-- Empty -->
    <div v-else-if="plugins.length === 0" class="text-gray-500 text-sm">
      No plugins registered. Add plugins to extend Pulse functionality.
    </div>

    <!-- Plugin list -->
    <div v-else class="space-y-3">
      <div
        v-for="plugin in plugins"
        :key="plugin.id"
        class="bg-gray-900 border border-gray-800 rounded-lg p-4"
      >
        <div class="flex items-start justify-between">
          <div class="flex items-center gap-3">
            <button
              @click="togglePlugin(plugin.id, plugin.enabled)"
              class="w-10 h-5 rounded-full transition-colors relative"
              :class="plugin.enabled ? 'bg-green-600' : 'bg-gray-700'"
            >
              <span
                class="absolute top-0.5 w-4 h-4 bg-white rounded-full transition-transform"
                :class="plugin.enabled ? 'left-5' : 'left-0.5'"
              />
            </button>
            <div>
              <h3 class="text-white font-medium text-sm">
                {{ plugin.name }}
                <span v-if="plugin.version" class="text-gray-500 text-xs ml-1">
                  v{{ plugin.version }}
                </span>
              </h3>
              <p v-if="plugin.description" class="text-xs text-gray-400 mt-0.5">
                {{ plugin.description }}
              </p>
            </div>
          </div>
          <button
            @click="removePlugin(plugin.id)"
            class="text-xs text-red-400 hover:text-red-300"
          >
            Remove
          </button>
        </div>

        <div class="mt-2 flex gap-4 text-xs text-gray-500">
          <span>Type: {{ plugin.plugin_type }}</span>
          <span>Entry: {{ plugin.entry_point }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
