mod capabilities;
mod commands;
mod db;
mod models;
mod runner;

use commands::context;
use commands::projects;
use commands::runs;
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
            // Runs
            runs::execute_macro_step,
            runs::list_runs,
            runs::get_run_logs,
            runs::cancel_run,
            // Context
            context::get_context_files,
            context::build_context_string,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
