import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface Project {
  id: string;
  name: string;
  root_path: string;
  remote_url: string | null;
  created_at: string;
  last_opened: string;
}

export interface GitInfo {
  branch: string;
  sha: string;
  dirty: boolean;
}

export interface ProjectInfo {
  id: string;
  name: string;
  root_path: string;
  remote_url: string | null;
  created_at: string;
  last_opened: string;
  project_type: string | null;
  git: GitInfo | null;
  has_pulse_yaml: boolean;
  has_a2_yaml: boolean;
}

export interface PulseConfig {
  version: number | null;
  context: {
    files: string[] | null;
    copy_bundle: {
      max_bytes: number | null;
      include_git: { branch: boolean; sha: boolean; dirty: boolean } | null;
    } | null;
  } | null;
  services: Array<{
    name: string;
    cwd: string | null;
    type: string | null;
    dev: { command: string; ports: number[] | null } | null;
    health: { url: string } | null;
  }> | null;
  macros: Array<{
    id: string;
    title: string;
    steps: Array<{
      run: string;
      cwd: string | null;
      confirm: boolean | null;
    }>;
  }> | null;
}

export const useProjectsStore = defineStore("projects", () => {
  const projects = ref<Project[]>([]);
  const currentProject = ref<ProjectInfo | null>(null);
  const pulseConfig = ref<PulseConfig | null>(null);
  const loading = ref(false);

  async function fetchProjects() {
    loading.value = true;
    try {
      projects.value = await invoke("list_projects");
    } finally {
      loading.value = false;
    }
  }

  async function openProject(path: string): Promise<ProjectInfo> {
    const info: ProjectInfo = await invoke("open_project", { path });
    currentProject.value = info;
    await loadPulseConfig(info.root_path);
    return info;
  }

  async function loadProject(projectId: string) {
    currentProject.value = await invoke("get_project_info", {
      projectId,
    });
    if (currentProject.value) {
      await loadPulseConfig(currentProject.value.root_path);
    }
  }

  async function loadPulseConfig(rootPath: string) {
    pulseConfig.value = await invoke("get_pulse_config", { rootPath });
  }

  async function removeProject(projectId: string) {
    await invoke("remove_project", { projectId });
    projects.value = projects.value.filter((p) => p.id !== projectId);
    if (currentProject.value?.id === projectId) {
      currentProject.value = null;
    }
  }

  return {
    projects,
    currentProject,
    pulseConfig,
    loading,
    fetchProjects,
    openProject,
    loadProject,
    removeProject,
  };
});
