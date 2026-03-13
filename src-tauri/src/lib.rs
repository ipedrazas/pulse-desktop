mod capabilities;
mod commands;
mod db;
mod models;
mod runner;
mod search;
mod watcher;

use commands::a2;
use commands::apimap;
use commands::connectors;
use commands::context;
use commands::filebrowser;
use commands::health;
use commands::processes;
use commands::projects;
use commands::runs;
use commands::search as search_cmd;
use commands::snapshots;
use commands::watchers;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = db::init_db().await.expect("Failed to initialize database");
                handle.manage(pool);
            });

            // Register watcher manager
            app.manage(watcher::WatcherManager::new());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Projects
            projects::open_project,
            projects::list_projects,
            projects::get_project_info,
            projects::remove_project,
            projects::get_pulse_config,
            projects::get_git_status,
            projects::get_worklog,
            projects::save_worklog_entry,
            // Runs
            runs::execute_macro_step,
            runs::list_runs,
            runs::get_run_logs,
            runs::cancel_run,
            // Context
            context::get_context_files,
            context::build_context_string,
            // Health
            health::get_health_summary,
            health::get_env_parity,
            health::get_node_health,
            health::get_go_health,
            // Processes
            processes::start_service,
            processes::stop_service,
            processes::list_services,
            processes::check_port,
            processes::check_service_health,
            // Connectors
            connectors::resolve_connectors,
            connectors::stream_connector,
            // Search
            search_cmd::search_project,
            search_cmd::search_all_projects,
            // a2
            a2::run_a2,
            // File browser
            filebrowser::get_file_tree,
            filebrowser::read_file_content,
            // Watchers
            watchers::get_watcher_statuses,
            watchers::set_watcher_enabled,
            // Snapshots
            snapshots::list_snapshots,
            snapshots::diff_snapshots,
            // API Map
            apimap::discover_api_map,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
