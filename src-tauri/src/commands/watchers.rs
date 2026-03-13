use crate::watcher::{WatcherManager, WatcherStatus};
use tauri::State;

#[tauri::command]
pub async fn get_watcher_statuses(
    watcher_manager: State<'_, WatcherManager>,
    project_id: String,
) -> Result<Vec<WatcherStatus>, String> {
    Ok(watcher_manager.get_statuses(&project_id))
}

#[tauri::command]
pub async fn set_watcher_enabled(
    watcher_manager: State<'_, WatcherManager>,
    project_id: String,
    watcher_id: String,
    enabled: bool,
) -> Result<(), String> {
    watcher_manager.set_enabled(&project_id, &watcher_id, enabled);
    Ok(())
}
