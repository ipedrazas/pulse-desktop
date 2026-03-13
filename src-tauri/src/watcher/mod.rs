pub mod debounce;

use crate::models::{MacroStep, PulseConfig, WatcherConfig};
use tauri::Emitter;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherStatus {
    pub id: String,
    pub title: String,
    pub enabled: bool,
    pub state: String, // "idle", "running", "pass", "fail"
    pub last_run_at: Option<String>,
    pub last_exit_code: Option<i32>,
}

pub struct WatcherManager {
    watchers: Arc<Mutex<HashMap<String, WatcherHandle>>>,
}

struct WatcherHandle {
    config: WatcherConfig,
    status: WatcherStatus,
    _watcher: Option<RecommendedWatcher>,
}

impl WatcherManager {
    pub fn new() -> Self {
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_watchers(
        &self,
        project_id: &str,
        root_path: &Path,
        config: &PulseConfig,
        pool: sqlx::SqlitePool,
        app_handle: tauri::AppHandle,
    ) {
        let watcher_configs = match &config.watchers {
            Some(w) => w.clone(),
            None => return,
        };

        let macros = config.macros.clone().unwrap_or_default();

        for wc in watcher_configs {
            let enabled = wc.enabled.unwrap_or(false);
            let status = WatcherStatus {
                id: wc.id.clone(),
                title: wc.title.clone(),
                enabled,
                state: "idle".to_string(),
                last_run_at: None,
                last_exit_code: None,
            };

            let handle = WatcherHandle {
                config: wc.clone(),
                status,
                _watcher: None,
            };

            let key = format!("{}:{}", project_id, wc.id);
            self.watchers.lock().unwrap().insert(key.clone(), handle);

            if enabled {
                self.start_watcher(
                    &key,
                    root_path,
                    &wc,
                    &macros,
                    pool.clone(),
                    project_id.to_string(),
                    app_handle.clone(),
                );
            }
        }
    }

    fn start_watcher(
        &self,
        key: &str,
        root_path: &Path,
        wc: &WatcherConfig,
        macros: &[crate::models::MacroConfig],
        pool: sqlx::SqlitePool,
        project_id: String,
        app_handle: tauri::AppHandle,
    ) {
        let glob_pattern = wc.glob.clone().unwrap_or_else(|| "**/*".to_string());
        let debounce_ms = wc.debounce_ms.unwrap_or(1500);
        let macro_id = wc.macro_ref.clone().unwrap_or_default();

        // Find the macro steps
        let steps: Vec<MacroStep> = macros
            .iter()
            .find(|m| m.id == macro_id)
            .map(|m| m.steps.clone())
            .unwrap_or_default();

        if steps.is_empty() {
            return;
        }

        let root = root_path.to_path_buf();
        let watchers = self.watchers.clone();
        let key_owned = key.to_string();

        let (tx, mut rx) = mpsc::channel::<PathBuf>(100);

        // Spawn notify watcher
        let tx_clone = tx.clone();
        let root_clone = root.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        for path in event.paths {
                            if let Ok(rel) = path.strip_prefix(&root_clone) {
                                let rel_str = rel.to_string_lossy();
                                if glob::Pattern::new(&glob_pattern)
                                    .map(|p| p.matches(&rel_str))
                                    .unwrap_or(false)
                                {
                                    let _ = tx_clone.blocking_send(path);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
        .ok();

        if let Some(ref mut w) = watcher {
            let _ = w.watch(&root, RecursiveMode::Recursive);
        }

        // Store the watcher handle
        if let Ok(mut map) = watchers.lock() {
            if let Some(handle) = map.get_mut(&key_owned) {
                handle._watcher = watcher;
            }
        }

        // Spawn debounced executor
        let watchers_exec = self.watchers.clone();
        let key_exec = key_owned.clone();
        tokio::spawn(async move {
            let mut debouncer = debounce::Debouncer::new(debounce_ms);

            while rx.recv().await.is_some() {
                if !debouncer.should_fire().await {
                    continue;
                }

                // Update status to running
                {
                    if let Ok(mut map) = watchers_exec.lock() {
                        if let Some(h) = map.get_mut(&key_exec) {
                            h.status.state = "running".to_string();
                        }
                    }
                }
                let _ = app_handle.emit("watcher:status", serde_json::json!({
                    "watcher_id": &key_exec,
                    "state": "running",
                }));

                // Execute macro steps
                let mut success = true;
                for step in &steps {
                    let parts: Vec<&str> = step.run.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }

                    let step_cwd = step
                        .cwd
                        .as_ref()
                        .map(|c| root.join(c))
                        .unwrap_or_else(|| root.clone());

                    let result = tokio::process::Command::new(parts[0])
                        .args(&parts[1..])
                        .current_dir(&step_cwd)
                        .output()
                        .await;

                    match result {
                        Ok(output) => {
                            let exit_code = output.status.code().unwrap_or(-1);

                            // Store run in DB
                            let _ = sqlx::query(
                                "INSERT INTO runs (project_id, kind, macro_id, status, command, cwd, exit_code, finished_at)
                                 VALUES (?, 'watcher', ?, ?, ?, ?, ?, datetime('now'))",
                            )
                            .bind(&project_id)
                            .bind(&macro_id)
                            .bind(if output.status.success() { "success" } else { "failure" })
                            .bind(&step.run)
                            .bind(step_cwd.to_string_lossy().as_ref())
                            .bind(exit_code)
                            .execute(&pool)
                            .await;

                            if !output.status.success() {
                                success = false;
                                break;
                            }
                        }
                        Err(_) => {
                            success = false;
                            break;
                        }
                    }
                }

                let state = if success { "pass" } else { "fail" };
                let exit_code = if success { Some(0) } else { Some(1) };

                {
                    if let Ok(mut map) = watchers_exec.lock() {
                        if let Some(h) = map.get_mut(&key_exec) {
                            h.status.state = state.to_string();
                            h.status.last_exit_code = exit_code;
                            h.status.last_run_at =
                                Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
                        }
                    }
                }
                let _ = app_handle.emit("watcher:status", serde_json::json!({
                    "watcher_id": &key_exec,
                    "state": state,
                    "exit_code": exit_code,
                }));
            }
        });
    }

    pub fn get_statuses(&self, project_id: &str) -> Vec<WatcherStatus> {
        let prefix = format!("{}:", project_id);
        self.watchers
            .lock()
            .unwrap()
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, h)| h.status.clone())
            .collect()
    }

    pub fn set_enabled(&self, project_id: &str, watcher_id: &str, enabled: bool) {
        let key = format!("{}:{}", project_id, watcher_id);
        if let Ok(mut map) = self.watchers.lock() {
            if let Some(h) = map.get_mut(&key) {
                h.status.enabled = enabled;
                h.config.enabled = Some(enabled);
                // If disabling, drop the notify watcher
                if !enabled {
                    h._watcher = None;
                    h.status.state = "idle".to_string();
                }
            }
        }
    }

    pub fn stop_all(&self, project_id: &str) {
        let prefix = format!("{}:", project_id);
        let mut map = self.watchers.lock().unwrap();
        let keys: Vec<String> = map
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        for key in keys {
            map.remove(&key);
        }
    }
}
