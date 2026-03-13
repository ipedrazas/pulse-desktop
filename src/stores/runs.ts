import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Run {
  id: number;
  project_id: string;
  kind: string;
  macro_id: string | null;
  status: string;
  command: string;
  cwd: string;
  exit_code: number | null;
  started_at: string;
  finished_at: string | null;
}

export interface RunLog {
  id: number;
  run_id: number;
  stream: string;
  chunk: string;
  ts: string;
}

interface RunLogEvent {
  run_id: number;
  stream: string;
  line: string;
}

interface RunStatusEvent {
  run_id: number;
  status: string;
  exit_code: number | null;
}

export const useRunsStore = defineStore("runs", () => {
  const runs = ref<Run[]>([]);
  const activeRunLogs = ref<Map<number, string[]>>(new Map());
  const activeRunId = ref<number | null>(null);

  async function fetchRuns(projectId: string) {
    runs.value = await invoke("list_runs", { projectId });
  }

  async function executeStep(
    projectId: string,
    macroId: string,
    step: { run: string; cwd: string | null; confirm: boolean | null },
    cwd: string
  ): Promise<number> {
    const runId: number = await invoke("execute_macro_step", {
      projectId,
      macroId,
      step,
      cwd,
    });
    activeRunId.value = runId;
    activeRunLogs.value.set(runId, []);
    return runId;
  }

  async function getRunLogs(runId: number): Promise<RunLog[]> {
    return await invoke("get_run_logs", { runId });
  }

  async function cancelRun(runId: number) {
    await invoke("cancel_run", { runId });
  }

  function setupEventListeners() {
    listen<RunLogEvent>("run:log", (event) => {
      const { run_id, stream, line } = event.payload;
      const logs = activeRunLogs.value.get(run_id) || [];
      const prefix = stream === "stderr" ? "[ERR] " : "";
      logs.push(prefix + line);
      activeRunLogs.value.set(run_id, logs);
    });

    listen<RunStatusEvent>("run:status", (event) => {
      const { run_id, status, exit_code } = event.payload;
      const run = runs.value.find((r) => r.id === run_id);
      if (run) {
        run.status = status;
        run.exit_code = exit_code;
      }
    });
  }

  return {
    runs,
    activeRunLogs,
    activeRunId,
    fetchRuns,
    executeStep,
    getRunLogs,
    cancelRun,
    setupEventListeners,
  };
});
